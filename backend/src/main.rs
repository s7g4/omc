use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod telemetry;

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Fetch database URL from environment
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable must be set");

    // Initialize database pool and run migrations
    let pool = db::init_db(&database_url).await;

    // Build router with shared state (the connection pool)
    let app = Router::new()
        .route("/health", get(health_check))
        .route(
            "/api/v1/telemetry",
            post(telemetry::handlers::ingest_telemetry),
        )
        .with_state(pool);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8081));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}
