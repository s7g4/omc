use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct SimulatorControlMsg {
    satellite_id: Uuid,
    command: String,
    fault: Option<String>,
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

    // Shared thread-safe variable to hold current active fault overlay
    let active_fault = Arc::new(tokio::sync::Mutex::new(None::<String>));

    // Connect to Redis for control command subscription
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6380".to_string());
    let redis_client = redis::Client::open(redis_url).expect("Failed to initialize Redis client");

    let active_fault_clone = Arc::clone(&active_fault);
    let satellite_id = config.satellite_id;

    tokio::spawn(async move {
        loop {
            tracing::info!("Connecting to Redis control stream...");
            match redis_client.get_async_pubsub().await {
                Ok(mut pubsub) => {
                    if let Err(e) = pubsub.subscribe("simulator:control").await {
                        tracing::error!("Failed to subscribe to Redis control channel: {:?}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                    tracing::info!("Subscribed to control channel 'simulator:control'");

                    let mut pubsub_stream = pubsub.into_on_message();
                    while let Some(msg) = pubsub_stream.next().await {
                        let payload: String = match msg.get_payload() {
                            Ok(p) => p,
                            Err(e) => {
                                tracing::error!("Failed to read control payload: {:?}", e);
                                continue;
                            }
                        };

                        if let Ok(cmd) = serde_json::from_str::<SimulatorControlMsg>(&payload) {
                            if cmd.satellite_id == satellite_id {
                                let mut lock = active_fault_clone.lock().await;
                                *lock = cmd.fault.clone();
                                tracing::warn!("System Override register set: {:?}", *lock);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Redis command link failed: {:?}", e);
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    let mut state = physics::SatelliteState::new();
    let tick_duration = Duration::from_millis(config.tick_interval_ms);

    // Reuse a single Client instance for connection pooling
    let http_client = reqwest::Client::new();

    loop {
        // Read current active fault from control link state
        let current_fault = {
            let lock = active_fault.lock().await;
            lock.clone()
        };

        state.tick(current_fault.as_deref());

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
                        "Telemetry sent | Lat: {:+.4}, Lon: {:+.4} | Overrides: {:?} | Status: {}",
                        state.latitude,
                        state.longitude,
                        current_fault,
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
