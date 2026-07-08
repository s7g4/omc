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

    let row = sqlx::query!(
        r#"
        SELECT rt.id, rt.user_id, rt.expires_at, rt.revoked_at, u.username, u.role
        FROM refresh_tokens rt
        JOIN users u ON u.id = rt.user_id
        WHERE rt.token_hash = $1
        "#,
        hash
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database query failure"))?;

    let row = match row {
        Some(r) => r,
        None => return Err((StatusCode::UNAUTHORIZED, "Unknown refresh token")),
    };

    if row.revoked_at.is_some() {
        // Reuse of an already-rotated token: treat as compromised and revoke the chain.
        revoke_token_chain(&state, row.id).await;
        return Err((
            StatusCode::UNAUTHORIZED,
            "Refresh token reuse detected; session revoked",
        ));
    }

    if row.expires_at < chrono::Utc::now() {
        return Err((StatusCode::UNAUTHORIZED, "Refresh token expired"));
    }

    let new_refresh = issue_refresh_token(&state, row.user_id).await?;
    let new_refresh_hash = hash_token(&new_refresh);

    sqlx::query!(
        r#"
        UPDATE refresh_tokens
        SET revoked_at = NOW(), replaced_by = (SELECT id FROM refresh_tokens WHERE token_hash = $2)
        WHERE id = $1
        "#,
        row.id,
        new_refresh_hash
    )
    .execute(&state.db)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to rotate refresh token",
        )
    })?;

    let token = generate_jwt(&row.username, row.user_id, &row.role)?;

    Ok((
        StatusCode::OK,
        Json(AuthResponse {
            token,
            refresh_token: new_refresh,
            username: row.username,
            role: row.role,
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
    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "mission_control_default_secret_key_12345".to_string());

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
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to encode token"))
}
