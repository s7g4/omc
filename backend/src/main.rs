use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod auth;
mod db;
mod metrics;
mod missions;
mod telemetry;
mod websockets;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub redis: redis::Client,
    pub nats: async_nats::Client,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

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

    // Configure NATS JetStream
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    jetstream
        .get_or_create_stream(async_nats::jetstream::stream::Config {
            name: "TELEMETRY_STREAM".to_string(),
            subjects: vec!["telemetry.>".to_string()],
            ..Default::default()
        })
        .await
        .expect("Failed to create NATS JetStream stream");

    // Bundle into shared state
    let state = AppState {
        db: pool,
        redis: redis_client,
        nats: nats_client,
    };

    let api_routes = Router::new()
        .route("/telemetry", post(telemetry::handlers::ingest_telemetry))
        .route(
            "/telemetry/:id/history",
            get(telemetry::handlers::get_history),
        )
        .route("/telemetry/ws", get(websockets::handler::ws_handler))
        .route("/auth/register", post(auth::handlers::register_user))
        .route("/auth/login", post(auth::handlers::login_user))
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
        .route_layer(axum::middleware::from_fn(metrics::track_metrics));

    let cors = tower_http::cors::CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
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
        .route("/metrics", get(metrics::metrics_handler))
        .nest("/api/v1", api_routes)
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8081));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}
