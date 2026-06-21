use serde::Serialize;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

mod config;
mod physics;

#[derive(Serialize)]
struct TelemetryPayload {
    satellite_id: Uuid,
    battery_level: f64,
    battery_temp: f64,
    solar_power: f64,
    velocity: f64,
    altitude: f64,
    latitude: f64,
    longitude: f64,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Initializing Satellite Simulator...");

    let config_path = "simulator_config.json";
    let config = match config::Config::load(config_path) {
        Ok(cfg) => {
            tracing::info!("Configuration loaded successfully");
            cfg
        }
        Err(_) => {
            tracing::warn!("Configuration file not found. Generating default...");
            config::Config::save_default(config_path)
                .expect("Critical: Failed to save default configuration")
        }
    };

    tracing::info!("Simulating Satellite ID: {}", config.satellite_id);
    tracing::info!("Targeting API Endpoint: {}", config.backend_url);

    let mut state = physics::SatelliteState::new();
    let tick_duration = Duration::from_millis(config.tick_interval_ms);

    // Reuse a single Client instance for connection pooling
    let http_client = reqwest::Client::new();

    loop {
        state.tick();

        let payload = TelemetryPayload {
            satellite_id: config.satellite_id,
            battery_level: state.battery_level,
            battery_temp: state.battery_temp,
            solar_power: state.solar_power,
            velocity: state.velocity,
            altitude: state.altitude,
            latitude: state.latitude,
            longitude: state.longitude,
        };

        // Send telemetry payload asynchronously
        let request_result = http_client
            .post(&config.backend_url)
            .json(&payload)
            .send()
            .await;

        match request_result {
            Ok(response) => {
                if response.status().is_success() {
                    tracing::info!(
                        "Telemetry sent | Lat: {:+.4}, Lon: {:+.4} | Status: {}",
                        state.latitude,
                        state.longitude,
                        response.status()
                    );
                } else {
                    tracing::warn!(
                        "Failed to send telemetry. Server responded: {}",
                        response.status()
                    );
                }
            }
            Err(e) => {
                tracing::error!("Network error: Failed to connect to telemetry API: {}", e);
            }
        }

        tokio::time::sleep(tick_duration).await;
    }
}
