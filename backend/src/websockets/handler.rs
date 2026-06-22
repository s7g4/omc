use crate::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    // 1. Get async Pub/Sub connection to Redis
    let mut pubsub = match state.redis.get_async_pubsub().await {
        Ok(ps) => ps,
        Err(e) => {
            tracing::error!("Failed to obtain Redis Pub/Sub connection: {:?}", e);
            return;
        }
    };

    // 2. Subscribe to "telemetry" channel
    if let Err(e) = pubsub.subscribe("telemetry").await {
        tracing::error!("Failed to subscribe to Redis telemetry channel: {:?}", e);
        return;
    }

    let mut pubsub_stream = pubsub.into_on_message();
    let (mut ws_sender, mut ws_receiver) = socket.split();

    tracing::info!("New WebSocket connection established");

    // 3. Concurrent Event Loop
    loop {
        tokio::select! {
            // A. Listen for new messages from Redis Pub/Sub
            Some(redis_msg) = pubsub_stream.next() => {
                let payload: String = match redis_msg.get_payload() {
                    Ok(val) => val,
                    Err(e) => {
                        tracing::error!("Failed to read Redis message payload: {:?}", e);
                        continue;
                    }
                };

                // Push payload to WebSocket client
                if let Err(e) = ws_sender.send(Message::Text(payload)).await {
                    tracing::info!("WebSocket client disconnected during send: {:?}", e);
                    break;
                }
            }

            // B. Listen for client closures or network drops
            Some(ws_msg) = ws_receiver.next() => {
                match ws_msg {
                    Ok(Message::Close(_)) | Err(_) => {
                        tracing::info!("WebSocket client closed connection");
                        break;
                    }
                    _ => {} // Ignore text/binary frames sent by the client
                }
            }
        }
    }
}
