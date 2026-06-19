use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Satellite simulator initializing...");

    loop {
        tracing::info!("Telemetry tick...");
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
