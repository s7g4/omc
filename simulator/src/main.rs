use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod physics;

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

    loop {
        state.tick();

        tracing::info!(
            "Lat: {:+.4}, Lon: {:+.4} | Alt: {:.1} km | Vel: {:.2} km/s | Battery: {:.1}% ({:+.1}°C) | Solar: {:.1} W",
            state.latitude,
            state.longitude,
            state.altitude,
            state.velocity,
            state.battery_level,
            state.battery_temp,
            state.solar_power
        );

        tokio::time::sleep(tick_duration).await;
    }
}
