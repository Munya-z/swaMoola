use uuid::Uuid;
use sqlx::PgPool;
use axum::{extract::{State, Path}, Json, response::IntoResponse, http::StatusCode, extract::Multipart};
use crate::chats::models::{DbEnvelope, EncryptedMessagePayload};
use crate::db::begin_rls_txn;
use crate::chats::models::{Message};
use crate::chats::conversation_handlers::{add_conversation_participant, find_existing_conversation, add_last_message_to_conversation, create_conversation};

async fn add_message_to_db(
    executor: &mut sqlx::PgConnection, 
    conv_id: Uuid,
    payload: EncryptedMessagePayload,
) -> anyhow::Result<Message> {
 
    let envelopes_json = serde_json::to_value(payload.envelopes)?;
    
    let query = sqlx::query_as!(
        Message,
        r#"
        INSERT INTO messages (conv_id, ciphertext, nonce, envelopes)
        VALUES ($1, $2, $3, $4)
        RETURNING 
            msg_id as "msg_id!", 
            conv_id as "conv_id!", 
            ciphertext as "ciphertext!",
            nonce as "nonce!",
            envelopes as "envelopes!: sqlx::types::Json<Vec<DbEnvelope>>"
        "#,
        conv_id,
        payload.ciphertext,
        payload.nonce,
        envelopes_json
    );

    let new_message = query.fetch_one(&mut *executor).await?;

    let _ = add_last_message_to_conversation(&mut *executor, conv_id, new_message.msg_id).await?;

    Ok(new_message)
    
}

pub async fn send_message(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    mut multipart: Multipart, 
) -> impl IntoResponse {
    let mut recipient_id = None;

    let mut ciphertext = None;
    let mut nonce = None;
    let mut envelopes_str = None;

    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                if let Some(name) = field.name() {
                    match name {
                        "recipient_id" => {
                            if let Ok(text) = field.text().await {
                                recipient_id = Uuid::parse_str(text.trim()).ok();
                            }
                        }
                        "ciphertext" => {
                            if let Ok(text) = field.text().await { ciphertext = Some(text.trim().to_string()); }
                        }
                        "nonce" => {
                            if let Ok(text) = field.text().await { nonce = Some(text.trim().to_string()); }
                        }
                        "envelopes" => {
                            if let Ok(text) = field.text().await { envelopes_str = Some(text.trim().to_string()); }
                        }
                        _ => {}
                    }
                }
            }
            Ok(None) => break,
            Err(e) => {
                return (StatusCode::BAD_REQUEST, Json(format!("Multipart parsing failed: {}", e))).into_response();
            }
        }
    }

    let recipient_id = match recipient_id {
        Some(r) => r,
        None => {
            return (StatusCode::BAD_REQUEST, Json("Missing or invalid recipient_id")).into_response();
        }
    };

    let c_text = match ciphertext {
        Some(c) => c,
        None => {
            return (StatusCode::BAD_REQUEST, Json("Missing ciphertext")).into_response();
        }
    };

    let n_text = match nonce {
        Some(n) => n,
        None => {
            return (StatusCode::BAD_REQUEST, Json("Missing nonce")).into_response();
        }
    };

    let env_raw = match envelopes_str {
        Some(s) => s,
        None => {
            return (StatusCode::BAD_REQUEST, Json("Missing envelope")).into_response();
        }
    };

    // 3. Parse JSON envelopes into your Rust structs
    let envelopes : Vec<DbEnvelope> = match serde_json::from_str(&env_raw) {
        Ok(env) => env,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, Json("Invalid envelope format")).into_response();
        }
    };

    let crypto_payload = EncryptedMessagePayload {
        ciphertext: c_text,
        nonce: n_text,
        envelopes,

    }; 

    let mut tx = match begin_rls_txn(&pool, user_id).await {
        Ok(tx) => tx,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to start transaction: {}", e))).into_response();
        }
    };

    let target_conv_id = match find_existing_conversation(&mut *tx, user_id, recipient_id).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            let conv = match create_conversation(&mut *tx).await {
                Ok(c) => c,
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string())).into_response();
                }
            };
            let _ = add_conversation_participant(&mut *tx, conv.conv_id, user_id).await;
            let _ = add_conversation_participant(&mut *tx, conv.conv_id, recipient_id).await;
            conv.conv_id
        },
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to locate conversation: {}", e))).into_response();
        }
    };

    let message = match add_message_to_db(&mut *tx, target_conv_id, crypto_payload).await {
        Ok(m) => m,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Database transmission failure: {}", e))).into_response();
        }
    };

    if let Err(e) = tx.commit().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to commit transaction: {}", e))).into_response();
    }

    (StatusCode::OK, Json(message)).into_response()
}