use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod audit;
mod auth;
mod db;
mod health;
mod metrics;
mod missions;
mod observability;
mod openapi;
mod resilience;
mod settings;
mod telemetry;
mod websockets;

use resilience::CircuitBreaker;
use settings::Settings;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub redis: redis::Client,
    pub nats: async_nats::Client,
    pub settings: Arc<Settings>,
    pub redis_breaker: Arc<CircuitBreaker>,
    pub nats_breaker: Arc<CircuitBreaker>,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let settings = Arc::new(Settings::load().expect("Failed to load layered configuration"));
    let otel_layer = observability::init_tracer(&settings);

    tracing_subscriber::registry()
        .with(otel_layer)
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::new(&settings.logging.level))
        .init();

    auth::secret::warn_if_using_default_secret();

    // Initialize metrics collectors
    metrics::init_metrics();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable must be set");

    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6380".to_string());

    let nats_url =
        std::env::var("NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".to_string());

    // Initialize database pool
    let pool = db::init_db(&database_url).await;

    // Initialize Redis client
    let redis_client = redis::Client::open(redis_url).expect("Failed to initialize Redis client");

    // Initialize NATS client
    let nats_client = async_nats::connect(nats_url)
        .await
        .expect("Failed to connect to NATS");

    // Configure NATS JetStream. `max_age` bounds the replay window: the WebSocket handler's
    // consumer uses `DeliverPolicy::All` (see websockets/handler.rs) so a client connecting
    // mid-mission still sees recent telemetry, but without a retention policy the stream would
    // grow forever and every new connection would replay the *entire* history since the stream
    // was created — flooding fresh dashboard sessions with stale data instead of catching them
    // up. `update_stream` (not just `get_or_create_stream`) is used so this also takes effect
    // on a stream that already existed from before this policy was added.
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    let stream_config = async_nats::jetstream::stream::Config {
        name: "TELEMETRY_STREAM".to_string(),
        subjects: vec!["telemetry.>".to_string()],
        max_age: std::time::Duration::from_secs(600),
        ..Default::default()
    };
    jetstream
        .get_or_create_stream(stream_config.clone())
        .await
        .expect("Failed to create NATS JetStream stream");
    jetstream
        .update_stream(stream_config)
        .await
        .expect("Failed to update NATS JetStream stream retention policy");

    let redis_breaker = Arc::new(CircuitBreaker::new(
        settings.circuit_breaker.failure_threshold,
        settings.circuit_breaker.cooldown_seconds,
    ));
    let nats_breaker = Arc::new(CircuitBreaker::new(
        settings.circuit_breaker.failure_threshold,
        settings.circuit_breaker.cooldown_seconds,
    ));

    // Bundle into shared state
    let state = AppState {
        db: pool,
        redis: redis_client,
        nats: nats_client,
        settings: settings.clone(),
        redis_breaker,
        nats_breaker,
    };

    // Auth gets its own, stricter rate-limit bucket — separate from the general one below,
    // which also covers high-frequency telemetry ingestion. Sharing a single bucket would mean
    // either the limit is loose enough to make brute-forcing /auth/login cheap, or tight enough
    // to throttle the simulator's once-a-second POSTs.
    let auth_governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(settings.rate_limit.auth_replenish_per_second)
            .burst_size(settings.rate_limit.auth_burst_size)
            .finish()
            .expect("Invalid auth rate limit configuration"),
    );
    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(settings.rate_limit.replenish_per_second)
            .burst_size(settings.rate_limit.burst_size)
            .finish()
            .expect("Invalid rate limit configuration"),
    );

    let auth_routes = Router::new()
        .route("/register", post(auth::handlers::register_user))
        .route("/login", post(auth::handlers::login_user))
        .route("/refresh", post(auth::handlers::refresh_token))
        .route("/logout", post(auth::handlers::logout_user))
        .layer(GovernorLayer {
            config: auth_governor_config,
        });

    let api_routes = Router::new()
        .route("/telemetry", post(telemetry::handlers::ingest_telemetry))
        .route(
            "/telemetry/:id/history",
            get(telemetry::handlers::get_history),
        )
        .route("/telemetry/ws", get(websockets::handler::ws_handler))
        .nest("/auth", auth_routes)
        .route(
            "/missions",
            get(missions::handlers::list_missions).post(missions::handlers::create_mission),
        )
        .route(
            "/missions/:id",
            get(missions::handlers::get_mission)
                .put(missions::handlers::update_mission)
                .delete(missions::handlers::delete_mission),
        )
        .route(
            "/missions/:id/assign",
            post(missions::handlers::assign_satellite),
        )
        .route(
            "/missions/:id/unassign",
            post(missions::handlers::unassign_satellite),
        )
        .route("/simulator/inject", post(telemetry::handlers::inject_fault))
        .route("/audit-logs", get(audit::list_audit_logs))
        .layer(GovernorLayer {
            config: governor_config,
        })
        .route_layer(axum::middleware::from_fn(metrics::track_metrics))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            audit::audit_log_layer,
        ));

    // `cors.allowed_origins` (base.toml/production.toml/APP__CORS__ALLOWED_ORIGINS) actually
    // drives this layer — it used to be defined in config but silently ignored in favor of a
    // hardcoded wildcard, which meant restricting it in production.toml had no real effect.
    let allow_origin = if settings.cors.allowed_origins.iter().any(|o| o == "*") {
        tower_http::cors::AllowOrigin::any()
    } else {
        let origins: Vec<axum::http::HeaderValue> = settings
            .cors
            .allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        tower_http::cors::AllowOrigin::list(origins)
    };

    let cors = tower_http::cors::CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ]);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/live", get(health::live))
        .route("/ready", get(health::ready))
        .route("/metrics", get(metrics::metrics_handler))
        .merge(
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()),
        )
        .nest("/api/v1", api_routes)
        .layer(cors)
        .with_state(state.clone());

    // Start gRPC Ingestion Server in the background
    let grpc_host = std::env::var("GRPC_HOST").unwrap_or_else(|_| "[::1]".to_string());
    let grpc_port = std::env::var("GRPC_PORT").unwrap_or_else(|_| "50051".to_string());
    let grpc_state = state.clone();
    tokio::spawn(async move {
        let addr = format!("{}:{}", grpc_host, grpc_port)
            .parse()
            .expect("Invalid GRPC_HOST/GRPC_PORT");
        let telemetry_service = telemetry::grpc::MyTelemetryIngest::new(grpc_state);

        tracing::info!("Starting gRPC server on {}", addr);

        tonic::transport::Server::builder()
            .add_service(telemetry::grpc::TelemetryIngestServer::new(
                telemetry_service,
            ))
            .serve(addr)
            .await
            .unwrap();
    });

    let http_host = std::env::var("HTTP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let http_port = std::env::var("HTTP_PORT").unwrap_or_else(|_| "8081".to_string());
    let addr: SocketAddr = format!("{}:{}", http_host, http_port)
        .parse()
        .expect("Invalid HTTP_HOST/HTTP_PORT");
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}
