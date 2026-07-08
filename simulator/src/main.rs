use futures_util::StreamExt;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

mod communication;
mod config;
mod physics;

// Include generated code
pub mod telemetry_proto {
    tonic::include_proto!("telemetry");
}

use telemetry_proto::telemetry_ingest_client::TelemetryIngestClient;
use telemetry_proto::TelemetryRequest;

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
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
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
    let mut comms = communication::channel::CommsChannel::new();
    let tick_duration = Duration::from_millis(config.tick_interval_ms);

    let grpc_addr = std::env::var("GRPC_URL").unwrap_or_else(|_| "http://[::1]:50051".to_string());
    tracing::info!("Targeting gRPC Endpoint: {}", grpc_addr);

    let mut grpc_client = None;

    loop {
        // Read current active fault from control link state
        let current_fault = {
            let lock = active_fault.lock().await;
            lock.clone()
        };

        state.tick(current_fault.as_deref());
        comms.tick(state.altitude, &mut rand::thread_rng());

        // Establish / retry gRPC client connection
        if grpc_client.is_none() {
            match TelemetryIngestClient::connect(grpc_addr.clone()).await {
                Ok(client) => {
                    tracing::info!("Successfully connected to gRPC server");
                    grpc_client = Some(client);
                }
                Err(e) => {
                    tracing::error!("Failed to connect to gRPC server: {:?}. Will retry...", e);
                }
            }
        }

        if comms.should_drop_packet(&mut rand::thread_rng()) {
            tracing::warn!(
                "Simulated packet drop (SNR {:.1}dB) | Lat: {:+.4}, Lon: {:+.4}",
                comms.snr_db,
                state.latitude,
                state.longitude
            );
            tokio::time::sleep(tick_duration).await;
            continue;
        }

        if let Some(ref mut client) = grpc_client {
            let (tx_lat, tx_lon) = comms.apply_gps_drift(state.latitude, state.longitude);
            let request = tonic::Request::new(TelemetryRequest {
                satellite_id: config.satellite_id.to_string(),
                battery_level: state.battery_level,
                battery_temp: state.battery_temp,
                solar_power: state.solar_power,
                velocity: state.velocity,
                altitude: state.altitude,
                latitude: tx_lat,
                longitude: tx_lon,
            });

            match client.ingest_telemetry(request).await {
                Ok(response) => {
                    let resp = response.into_inner();
                    tracing::info!(
                        "Telemetry sent over gRPC | Lat: {:+.4}, Lon: {:+.4} | SNR: {:.1}dB | Overrides: {:?} | Status: {} {}",
                        tx_lat,
                        tx_lon,
                        comms.snr_db,
                        current_fault,
                        resp.status,
                        resp.message
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "gRPC Ingestion request failed: {:?}. Resetting client...",
                        e
                    );
                    grpc_client = None; // Reset on failure to trigger reconnect on next loop
                }
            }
        }

        tokio::time::sleep(tick_duration).await;
    }
}
