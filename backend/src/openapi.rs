use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

struct BearerAuthAddon;

impl Modify for BearerAuthAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::auth::handlers::register_user,
        crate::auth::handlers::login_user,
        crate::auth::handlers::refresh_token,
        crate::auth::handlers::logout_user,
        crate::missions::handlers::create_mission,
        crate::missions::handlers::list_missions,
        crate::missions::handlers::delete_mission,
        crate::telemetry::handlers::ingest_telemetry,
        crate::telemetry::handlers::inject_fault,
        crate::health::live,
        crate::health::ready,
        crate::audit::list_audit_logs,
    ),
    components(schemas(
        crate::auth::models::RegisterRequest,
        crate::auth::models::LoginRequest,
        crate::auth::models::AuthResponse,
        crate::auth::models::RefreshRequest,
        crate::auth::models::LogoutRequest,
        crate::missions::models::Mission,
        crate::missions::models::CreateMissionRequest,
        crate::telemetry::models::Telemetry,
        crate::telemetry::models::CreateTelemetry,
        crate::telemetry::handlers::InjectFaultRequest,
        crate::health::checks::DependencyStatus,
        crate::audit::models::AuditLog,
    )),
    tags(
        (name = "auth", description = "Authentication, refresh token rotation, and session revocation"),
        (name = "missions", description = "Mission CRUD and satellite assignment"),
        (name = "telemetry", description = "Telemetry ingestion and simulator fault injection"),
        (name = "health", description = "Liveness and readiness probes"),
        (name = "audit", description = "Immutable audit trail of API activity"),
    ),
    modifiers(&BearerAuthAddon)
)]
pub struct ApiDoc;
