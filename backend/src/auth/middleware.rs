use super::models::Claims;
use super::secret::decode_claims;
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
};

#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 1. Get Authorization header
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "Missing Authorization header"))?;

        // 2. Validate prefix
        if !auth_header.starts_with("Bearer ") {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Authorization header must be a Bearer token",
            ));
        }

        let token = &auth_header[7..];

        // 3. Decode and validate
        decode_claims(token).ok_or((
            StatusCode::UNAUTHORIZED,
            "Invalid or expired authorization token",
        ))
    }
}

/// Wraps `Claims`, additionally rejecting with 403 unless the caller has the `admin` role.
/// The inner `Claims` is intentionally unused by most callers — extracting `AdminClaims`
/// successfully *is* the authorization check.
#[allow(dead_code)]
pub struct AdminClaims(pub Claims);

#[async_trait]
impl<S> FromRequestParts<S> for AdminClaims
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let claims = Claims::from_request_parts(parts, state).await?;
        if claims.role != "admin" {
            return Err((StatusCode::FORBIDDEN, "Admin role required for this action"));
        }
        Ok(AdminClaims(claims))
    }
}
