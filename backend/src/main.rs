use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod telemetry;
mod websockets;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub redis: redis::Client,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable must be set");

    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6380".to_string());

    // Initialize database pool
    let pool = db::init_db(&database_url).await;

    // Initialize Redis client
    let redis_client = redis::Client::open(redis_url).expect("Failed to initialize Redis client");

    // Bundle into shared state
    let state = AppState {
        db: pool,
        redis: redis_client,
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route(
            "/api/v1/telemetry",
            post(telemetry::handlers::ingest_telemetry),
        )
        .route("/api/v1/telemetry/ws", get(websockets::handler::ws_handler))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8081));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}
