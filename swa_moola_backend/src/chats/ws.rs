use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Path, State},
    response::IntoResponse,
};
use tokio::sync::mpsc::unbounded_channel;
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use sqlx::{postgres::PgListener};
use uuid::Uuid;
use crate::{AppState, models::{SignalingMessage}};


// pub async fn ws_handler(
//     Path(user_id): Path<Uuid>,
//     ws: WebSocketUpgrade,
//     State(state): State<AppState>,
// ) -> impl IntoResponse {
//     let pool = state.db;
//     ws.on_upgrade(move |socket| handle_socket_stream(socket, pool, user_id))
// }



// async fn handle_socket_stream(socket: WebSocket, pool: PgPool, user_id: Uuid) {
//     let (mut sender, mut receiver) = socket.split();

//     let mut listener = match PgListener::connect_with(&pool).await {
//         Ok(l) => l,
//         Err(e) => {
//             eprintln!("Failed to connect PgListener: {e}");
//             return;
//         }
//     };

//     let channel_name = format!("user_updates_{}", user_id);

//     if let Err(e) = listener.listen(&channel_name).await {
//         eprintln!("Failed to listen on pub/sub channel: {e}");
//         return;
//     }

//     let mut db_stream = listener.into_stream();

//     loop {
//         tokio::select! {
//             // Case A: A new notification was dispatched out of PostgreSQL
//             Some(Ok(notification)) = db_stream.next() => {
//                 let json_payload = notification.payload(); 
                
//                 // Axum 0.8: convert your String payload into Utf8Bytes cleanly
//                 let bytes = axum::extract::ws::Utf8Bytes::from(json_payload.to_string());
                
//                 if let Err(e) = sender.send(Message::Text(bytes)).await {
//                     eprintln!("WebSocket write failure (client disconnected): {e}");
//                     break;
//                 }
//             }

//             // Case B: Monitor if the user closed their tab on Leptos
//             Some(client_msg) = receiver.next() => {
//                 match client_msg {
//                     Ok(Message::Close(_)) | Err(_) => {
//                         println!("User {user_id} disconnected their socket interface cleanly.");
//                         break;
//                     }
//                     _ => {} 
//                 }
//             }
//         }
//     }
// }

pub async fn ws_handler(
    Path(user_id): Path<Uuid>,
    ws: WebSocketUpgrade,
    State(app_state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_unified_socket(socket, user_id, app_state))
}

async fn handle_unified_socket(socket: WebSocket, user_id: Uuid, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = unbounded_channel::<Message>();
    let user_id_str = user_id.to_string();

    // 1. Establish the local PostgreSQL database notification channel pipe
    let mut listener = match PgListener::connect_with(&state.db).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to connect PgListener for {user_id}: {e}");
            return;
        }
    };
    let channel_name = format!("user_updates_{}", user_id);
    if let Err(e) = listener.listen(&channel_name).await {
        eprintln!("Failed to listen on pub/sub channel {channel_name}: {e}");
        return;
    }
    let mut db_stream = listener.into_stream();

    // 2. Register this user's outbound memory pipe handle into the global WebRTC switchboard map
    println!("🔌 Unified socket established. Registering switchboard mapping for ID: {user_id_str}");
    if let Ok(mut lock) = state.switchboard.write() {
        lock.connections.insert(user_id_str.clone(), tx);
    }

    // 3. Spawns dedicated background worker task to pipe events down to the client safely
    // This worker acts as a shared outbound multiplexer funnel
    let (outbound_tx, mut outbound_rx) = unbounded_channel::<Message>();
    tokio::spawn(async move {
        while let Some(msg) = outbound_rx.recv().await {
            if sender.send(msg).await.is_err() {
                break; // Tear down if browser connection disconnects
            }
        }
    });

    // Clone transmitter contexts to pass things inside our async framework threads
    let outbound_tx_main = outbound_tx.clone();
    let outbound_tx_for_rtc = outbound_tx.clone();

    // Loop A: Monitors out-of-band WebRTC packets routed from other users' switchboard interactions
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if outbound_tx_for_rtc.send(message).is_err() {
                break;
            }
        }
    });

    // Loop B: Monitors reactive database changes pushing out of your core PostgreSQL layers
    loop{    
        tokio::select! {
            // Branch 1: Monitor reactive database changes pushing out of PostgreSQL layers
            Some(Ok(notification)) = db_stream.next() => {
                let json_payload = notification.payload();
                let bytes = axum::extract::ws::Utf8Bytes::from(json_payload.to_string());
                if outbound_tx_main.send(Message::Text(bytes)).is_err() {
                    break;
                }
            }

            // Branch 2: Core structural inbound listener. Watches for incoming network signals from the client browser
            client_msg_opt = receiver.next() => {
                match client_msg_opt {
                    Some(Ok(client_msg)) => {
                        match client_msg {
                            Message::Text(text) => {
                                if let Ok(signal) = serde_json::from_str::<SignalingMessage>(&text) {
                                    match signal {
                                        SignalingMessage::CallRequest { ref target_user_id, .. } |
                                        SignalingMessage::CallResponse { ref target_user_id, .. } |
                                        SignalingMessage::IceCandidate { ref target_user_id, .. } => {
                                            if let Ok(lock) = state.switchboard.read() {
                                                if let Some(target_tx) = lock.connections.get(target_user_id) {
                                                    let _ = target_tx.send(Message::Text(text.clone()));
                                                } else {
                                                    println!("⚠️ Routing failed: Target Peer ID {} is offline", target_user_id);
                                                }
                                            }
                                        },
                                        _ => {} 
                                    }
                                } else {
                                    println!("💬 Chat text string payload intercept: {}", text);
                                }
                            },
                            Message::Close(_) => break,
                            _ => {}
                        }
                    }
                    _ => break, // Break immediately if the client disconnects or errors out
                }
            }
        }
    }
    // 4. Cleanup allocations when user drops connection loops
    println!("❌ Unified connection terminated for user ID: {user_id_str}");
    if let Ok(mut lock) = state.switchboard.write() {
        lock.connections.remove(&user_id_str);
    }
}
