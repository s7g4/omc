use super::models::Claims;
use jsonwebtoken::{decode, DecodingKey, Validation};

const INSECURE_DEFAULT_SECRET: &str = "mission_control_default_secret_key_12345";

/// Single source of truth for the JWT signing/verification secret — previously copy-pasted
/// verbatim in three places (auth/handlers.rs, auth/middleware.rs, audit/mod.rs), which meant
/// fixing the insecure fallback would have needed three coordinated edits instead of one.
pub fn jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| INSECURE_DEFAULT_SECRET.to_string())
}

/// Logs a loud, impossible-to-miss warning at startup if `JWT_SECRET` isn't set. Doesn't hard
/// panic: local/demo usage (`docker compose up` with no `.env`) should still boot, but a real
/// deployment that forgot to set it should not fail *silently* — a hardcoded secret sitting in
/// a public repo is a real credential leak if it ever signs a token anyone relies on.
pub fn warn_if_using_default_secret() {
    if std::env::var("JWT_SECRET").is_err() {
        tracing::warn!(
            "JWT_SECRET is not set — falling back to a hardcoded, publicly-known default. \
             Every token issued by this instance is forgeable by anyone who has read this \
             repository. Set JWT_SECRET before running this anywhere beyond localhost."
        );
    }
}

pub fn decode_claims(token: &str) -> Option<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret().as_bytes()),
        &Validation::default(),
    )
    .ok()
    .map(|data| data.claims)
}
