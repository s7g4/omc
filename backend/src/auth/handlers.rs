use super::models::{
    AuthResponse, Claims, LoginRequest, LogoutRequest, RefreshRequest, RegisterRequest, User,
};
use crate::AppState;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{extract::State, http::StatusCode, Json};
use jsonwebtoken::{encode, EncodingKey, Header};
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const ACCESS_TOKEN_TTL_HOURS: i64 = 1;
const REFRESH_TOKEN_TTL_DAYS: i64 = 7;

#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered", body = AuthResponse),
        (status = 409, description = "Username already taken")
    ),
    tag = "auth"
)]
pub async fn register_user(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, &'static str)> {
    if payload.username.trim().is_empty() || payload.password.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Username and password cannot be empty",
        ));
    }

    // Check if user already exists
    let existing_user = sqlx::query!("SELECT id FROM users WHERE username = $1", payload.username)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database query failure"))?;

    if existing_user.is_some() {
        return Err((StatusCode::CONFLICT, "Username is already taken"));
    }

    // Hash password using Argon2
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to hash password"))?
        .to_string();

    // Insert user into DB (default role: operator)
    let new_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, username, password_hash, role) VALUES ($1, $2, $3, 'operator')",
        new_id,
        payload.username,
        password_hash
    )
    .execute(&state.db)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database insertion failure",
        )
    })?;

    let token = generate_jwt(&payload.username, new_id, "operator")?;
    let refresh_token = issue_refresh_token(&state, new_id).await?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            refresh_token,
            username: payload.username,
            role: "operator".to_string(),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Authenticated", body = AuthResponse),
        (status = 401, description = "Invalid credentials")
    ),
    tag = "auth"
)]
pub async fn login_user(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, &'static str)> {
    // Find user by username
    let user = sqlx::query_as!(
        User,
        "SELECT id, username, password_hash, role, created_at FROM users WHERE username = $1",
        payload.username
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database query failure"))?;

    let user = match user {
        Some(u) => u,
        None => return Err((StatusCode::UNAUTHORIZED, "Invalid username or password")),
    };

    // Verify password hash
    let parsed_hash = PasswordHash::new(&user.password_hash).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to parse stored password hash",
        )
    })?;

    let argon2 = Argon2::default();
    if argon2
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err((StatusCode::UNAUTHORIZED, "Invalid username or password"));
    }

    let token = generate_jwt(&user.username, user.id, &user.role)?;
    let refresh_token = issue_refresh_token(&state, user.id).await?;

    Ok((
        StatusCode::OK,
        Json(AuthResponse {
            token,
            refresh_token,
            username: user.username,
            role: user.role,
        }),
    ))
}

/// Exchanges a still-valid refresh token for a new access+refresh pair, rotating the old one.
/// If the presented token was already rotated (i.e. reused), the entire descendant chain is
/// revoked on the assumption that the token has been stolen/replayed.
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "New access/refresh pair issued", body = AuthResponse),
        (status = 401, description = "Unknown, expired, or reused refresh token")
    ),
    tag = "auth"
)]
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, &'static str)> {
    let hash = hash_token(&payload.refresh_token);

    // Atomically claim the token by revoking it IFF it hasn't been revoked yet — this is the
    // only statement that decides whether this caller "wins" the rotation. A separate
    // SELECT-then-UPDATE would leave a window where two concurrent requests for the same token
    // both read revoked_at = NULL and both proceed, producing two live descendant chains from
    // one token and defeating reuse detection. With `revoked_at IS NULL` in the WHERE clause of
    // the UPDATE itself, only one concurrent caller can ever match; the other legitimately gets
    // 0 rows back and is treated as a (possibly benign, e.g. a double-fired request) reuse.
    let claimed = sqlx::query!(
        r#"
        UPDATE refresh_tokens
        SET revoked_at = NOW()
        WHERE token_hash = $1 AND revoked_at IS NULL
        RETURNING id, user_id, expires_at
        "#,
        hash
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database query failure"))?;

    let claimed = match claimed {
        Some(row) => row,
        None => {
            // Either the token never existed, or it did and lost the race above (reuse).
            // Only the latter warrants revoking the rest of the chain.
            let existing_id =
                sqlx::query_scalar!("SELECT id FROM refresh_tokens WHERE token_hash = $1", hash)
                    .fetch_optional(&state.db)
                    .await
                    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database query failure"))?;

            if let Some(id) = existing_id {
                revoke_token_chain(&state, id).await;
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "Refresh token reuse detected; session revoked",
                ));
            }
            return Err((StatusCode::UNAUTHORIZED, "Unknown refresh token"));
        }
    };

    if claimed.expires_at < chrono::Utc::now() {
        return Err((StatusCode::UNAUTHORIZED, "Refresh token expired"));
    }

    let user = sqlx::query!(
        "SELECT username, role FROM users WHERE id = $1",
        claimed.user_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database query failure"))?;

    let new_refresh = issue_refresh_token(&state, claimed.user_id).await?;
    let new_refresh_hash = hash_token(&new_refresh);

    sqlx::query!(
        r#"
        UPDATE refresh_tokens
        SET replaced_by = (SELECT id FROM refresh_tokens WHERE token_hash = $2)
        WHERE id = $1
        "#,
        claimed.id,
        new_refresh_hash
    )
    .execute(&state.db)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to link rotated refresh token",
        )
    })?;

    let token = generate_jwt(&user.username, claimed.user_id, &user.role)?;

    Ok((
        StatusCode::OK,
        Json(AuthResponse {
            token,
            refresh_token: new_refresh,
            username: user.username,
            role: user.role,
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    request_body = LogoutRequest,
    responses((status = 204, description = "Refresh token revoked")),
    tag = "auth"
)]
pub async fn logout_user(
    State(state): State<AppState>,
    Json(payload): Json<LogoutRequest>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let hash = hash_token(&payload.refresh_token);

    sqlx::query!(
        "UPDATE refresh_tokens SET revoked_at = NOW() WHERE token_hash = $1 AND revoked_at IS NULL",
        hash
    )
    .execute(&state.db)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to revoke refresh token",
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}

async fn revoke_token_chain(state: &AppState, start_id: Uuid) {
    // Walk the replaced_by chain forward, revoking every descendant token.
    let mut current = Some(start_id);
    while let Some(id) = current {
        let _ = sqlx::query!(
            "UPDATE refresh_tokens SET revoked_at = COALESCE(revoked_at, NOW()) WHERE id = $1",
            id
        )
        .execute(&state.db)
        .await;

        current = sqlx::query_scalar!("SELECT replaced_by FROM refresh_tokens WHERE id = $1", id)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
            .flatten();
    }
}

async fn issue_refresh_token(
    state: &AppState,
    user_id: Uuid,
) -> Result<String, (StatusCode, &'static str)> {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let token = hex::encode(bytes);
    let hash = hash_token(&token);
    let expires_at = chrono::Utc::now() + chrono::Duration::days(REFRESH_TOKEN_TTL_DAYS);

    sqlx::query!(
        "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
        user_id,
        hash,
        expires_at
    )
    .execute(&state.db)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to issue refresh token",
        )
    })?;

    Ok(token)
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

fn generate_jwt(
    username: &str,
    user_id: Uuid,
    role: &str,
) -> Result<String, (StatusCode, &'static str)> {
    let expiration = chrono::Utc::now() + chrono::Duration::hours(ACCESS_TOKEN_TTL_HOURS);
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        exp: expiration.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(super::secret::jwt_secret().as_bytes()),
    )
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to encode token"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_token_is_deterministic() {
        assert_eq!(hash_token("same-token"), hash_token("same-token"));
    }

    #[test]
    fn hash_token_differs_for_different_inputs() {
        assert_ne!(hash_token("token-a"), hash_token("token-b"));
    }

    #[test]
    fn hash_token_does_not_leak_the_original() {
        let hash = hash_token("super-secret-refresh-token");
        assert!(!hash.contains("super-secret-refresh-token"));
        // sha256 -> 32 bytes -> 64 hex chars
        assert_eq!(hash.len(), 64);
    }
}
