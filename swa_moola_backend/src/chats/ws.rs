// src/chats/ws.rs
use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Path, State},
    response::IntoResponse,
};
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use sqlx::{postgres::PgListener, PgPool};
use uuid::Uuid;

pub async fn ws_handler(
    Path(user_id): Path<Uuid>,
    ws: WebSocketUpgrade,
    State(pool): State<PgPool>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket_stream(socket, pool, user_id))
}

async fn handle_socket_stream(socket: WebSocket, pool: PgPool, user_id: Uuid) {
    let (mut sender, mut receiver) = socket.split();

    let mut listener = match PgListener::connect_with(&pool).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to connect PgListener: {e}");
            return;
        }
    };

    let channel_name = format!("user_updates_{}", user_id);

    if let Err(e) = listener.listen(&channel_name).await {
        eprintln!("Failed to listen on pub/sub channel: {e}");
        return;
    }

    let mut db_stream = listener.into_stream();

    loop {
        tokio::select! {
            // Case A: A new notification was dispatched out of PostgreSQL
            Some(Ok(notification)) = db_stream.next() => {
                let json_payload = notification.payload(); 
                
                // Axum 0.8: convert your String payload into Utf8Bytes cleanly
                let bytes = axum::extract::ws::Utf8Bytes::from(json_payload.to_string());
                
                if let Err(e) = sender.send(Message::Text(bytes)).await {
                    eprintln!("WebSocket write failure (client disconnected): {e}");
                    break;
                }
            }

            // Case B: Monitor if the user closed their tab on Leptos
            Some(client_msg) = receiver.next() => {
                match client_msg {
                    Ok(Message::Close(_)) | Err(_) => {
                        println!("User {user_id} disconnected their socket interface cleanly.");
                        break;
                    }
                    _ => {} 
                }
            }
        }
    }
}
