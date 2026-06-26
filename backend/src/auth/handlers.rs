use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use jsonwebtoken::{encode, Header, EncodingKey};
use uuid::Uuid;
use crate::AppState;
use super::models::{RegisterRequest, LoginRequest, AuthResponse, Claims, User};

pub async fn register_user(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, &'static str)> {
    if payload.username.trim().is_empty() || payload.password.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Username and password cannot be empty"));
    }

    // Check if user already exists
    let existing_user = sqlx::query!(
        "SELECT id FROM users WHERE username = $1",
        payload.username
    )
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

    // Insert user into DB
    let new_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3)",
        new_id,
        payload.username,
        password_hash
    )
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database insertion failure"))?;

    // Generate JWT token
    let token = generate_jwt(&payload.username, new_id)?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            username: payload.username,
        }),
    ))
}

pub async fn login_user(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, &'static str)> {
    // Find user by username
    let user = sqlx::query_as!(
        User,
        "SELECT id, username, password_hash, created_at FROM users WHERE username = $1",
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
    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to parse stored password hash"))?;

    let argon2 = Argon2::default();
    if argon2.verify_password(payload.password.as_bytes(), &parsed_hash).is_err() {
        return Err((StatusCode::UNAUTHORIZED, "Invalid username or password"));
    }

    // Generate JWT token
    let token = generate_jwt(&user.username, user.id)?;

    Ok((
        StatusCode::OK,
        Json(AuthResponse {
            token,
            username: user.username,
        }),
    ))
}

fn generate_jwt(username: &str, user_id: Uuid) -> Result<String, (StatusCode, &'static str)> {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "mission_control_default_secret_key_12345".to_string());
    
    let expiration = chrono::Utc::now() + chrono::Duration::hours(24);
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp: expiration.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes())
    )
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to encode token"))
}
