use crate::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};

#[derive(serde::Deserialize)]
pub struct WsParams {
    pub satellite_id: Option<uuid::Uuid>,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<WsParams>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, params.satellite_id))
}

async fn handle_socket(socket: WebSocket, state: AppState, satellite_id: Option<uuid::Uuid>) {
    // 1. Get async Pub/Sub connection to Redis (for events)
    let mut redis_pubsub = match state.redis.get_async_pubsub().await {
        Ok(ps) => ps,
        Err(e) => {
            tracing::error!("Failed to obtain Redis Pub/Sub connection: {:?}", e);
            return;
        }
    };

    // Subscribe to events channel on Redis
    if let Err(e) = redis_pubsub.subscribe("events").await {
        tracing::error!("Failed to subscribe to Redis events channel: {:?}", e);
        return;
    }

    let mut redis_stream = redis_pubsub.into_on_message();

    // 2. Setup NATS JetStream Consumer (for telemetry replay & stream)
    let jetstream = async_nats::jetstream::new(state.nats.clone());
    let stream = match jetstream.get_stream("TELEMETRY_STREAM").await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to get NATS stream: {:?}", e);
            return;
        }
    };

    let filter_subject = if let Some(sat_id) = satellite_id {
        format!("telemetry.{}", sat_id)
    } else {
        "telemetry.>".to_string()
    };

    let consumer = match stream
        .create_consumer(async_nats::jetstream::consumer::pull::Config {
            durable_name: None,
            deliver_policy: async_nats::jetstream::consumer::DeliverPolicy::All,
            filter_subject,
            ..Default::default()
        })
        .await
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to create NATS JetStream consumer: {:?}", e);
            return;
        }
    };

    let mut nats_stream = match consumer.messages().await {
        Ok(ms) => ms,
        Err(e) => {
            tracing::error!("Failed to open NATS JetStream consumer messages: {:?}", e);
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = socket.split();
    tracing::info!("New WebSocket connection established");

    // 3. Concurrent Select Loop
    loop {
        tokio::select! {
            // A. Listen for new messages from NATS JetStream (Telemetry replay and live stream)
            Some(Ok(nats_msg)) = nats_stream.next() => {
                let _ = nats_msg.ack().await;
                let payload = String::from_utf8_lossy(&nats_msg.message.payload).into_owned();

                if let Err(e) = ws_sender.send(Message::Text(payload)).await {
                    tracing::info!("WebSocket client disconnected during NATS delivery: {:?}", e);
                    break;
                }
            }

            // B. Listen for events from Redis Pub/Sub
            Some(redis_msg) = redis_stream.next() => {
                if let Ok(payload) = redis_msg.get_payload::<String>() {
                    if let Err(e) = ws_sender.send(Message::Text(payload)).await {
                        tracing::info!("WebSocket client disconnected during Redis delivery: {:?}", e);
                        break;
                    }
                }
            }

            // C. Listen for client closures or network drops
            Some(ws_msg) = ws_receiver.next() => {
                match ws_msg {
                    Ok(Message::Close(_)) | Err(_) => {
                        tracing::info!("WebSocket client closed connection");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}
