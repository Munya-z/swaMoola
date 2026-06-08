use uuid::Uuid;
use sqlx::PgPool;
use axum::{extract::{State, Path}, Json, response::IntoResponse, http::StatusCode, extract::Multipart};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Envelope {
    pub ephemeral_x25519: String,
    pub pq_ciphertext: String,
    pub encrypted_master_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedMessagePayload {
    pub ciphertext: String,
    pub nonce: String,
    pub s_envelope: Envelope,
    pub r_envelope: Envelope,
}

// imported crates from this curent project 
use crate::db::begin_rls_txn;
use crate::chats::models::{Message};
use crate::chats::conversation_handlers::{add_conversation_participant, find_existing_conversation, add_last_message_to_conversation, create_conversation};

// adds a new message to the database and updates the conversation's last_message_id accordingly.
async fn add_message_to_db(
    executor: &mut sqlx::PgConnection, 
    conv_id: Uuid,
    payload: EncryptedMessagePayload,
) -> anyhow::Result<Message> {
   
    // Convert the envelope structs into serde_json values for the JSONB columns
    let s_envelope_json = serde_json::to_value(payload.s_envelope)?;
    let r_envelope_json = serde_json::to_value(payload.r_envelope)?;

    // Update query to target the new secure columns
    let query = sqlx::query_as!(
        Message,
        r#"
        INSERT INTO messages (conv_id, ciphertext, nonce, s_envelope, r_envelope)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING 
            msg_id as "msg_id!", 
            conv_id as "conv_id!", 
            ciphertext as "ciphertext!",
            nonce as "nonce!",
            s_envelope as "s_envelope!: sqlx::types::Json<serde_json::Value>",
            r_envelope as "r_envelope!: sqlx::types::Json<serde_json::Value>"
        "#,
        conv_id,
        payload.ciphertext,
        payload.nonce,
        s_envelope_json,
        r_envelope_json
    );

    let new_message = query.fetch_one(&mut *executor).await?;

    // Update the conversation's last message tracker
    let _ = add_last_message_to_conversation(&mut *executor, conv_id, new_message.msg_id).await?;

    Ok(new_message)
    
}

pub async fn send_message(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    mut multipart: Multipart, 
) -> impl IntoResponse {
    let mut recipient_id = None;
    
    // Extracted payload fields
    let mut ciphertext = None;
    let mut nonce = None;
    let mut s_envelope_str = None;
    let mut r_envelope_str = None;
       

    // 1. Extract cryptographic payload and files from multipart stream
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
                        "s_envelope" => {
                            if let Ok(text) = field.text().await { s_envelope_str = Some(text.trim().to_string()); }
                        }
                        "r_envelope" => {
                            if let Ok(text) = field.text().await { r_envelope_str = Some(text.trim().to_string()); }
                        }
                        _ => {}
                    }
                }
            }
            Ok(None) => break, // Stream completed successfully
            Err(e) => {
                return (StatusCode::BAD_REQUEST, Json(format!("Multipart parsing failed: {}", e))).into_response();
            }
        }
    }

    // // 2. Validate mandatory IDs and crypto structures

    let recipient_id = match recipient_id {
        Some(r) => r,
        None => {
            println!("❌ [Validation Failure]: 'recipient_id' parameter is missing, mismatched, or invalid UUID formatting!");
            return (StatusCode::BAD_REQUEST, Json("Missing or invalid recipient_id")).into_response();
        }
    };

    let c_text = match ciphertext {
        Some(c) => c,
        None => {
            println!("❌ [Validation Failure]: 'ciphertext' field is missing or empty!");
            return (StatusCode::BAD_REQUEST, Json("Missing ciphertext")).into_response();
        }
    };

    let n_text = match nonce {
        Some(n) => n,
        None => {
            println!("❌ [Validation Failure]: 'nonce' field is missing or empty!");
            return (StatusCode::BAD_REQUEST, Json("Missing nonce")).into_response();
        }
    };

    let s_env_raw = match s_envelope_str {
        Some(s) => s,
        None => {
            println!("❌ [Validation Failure]: 's_envelope' configuration is missing!");
            return (StatusCode::BAD_REQUEST, Json("Missing s_envelope")).into_response();
        }
    };

    let r_env_raw = match r_envelope_str {
        Some(r) => r,
        None => {
            println!("❌ [Validation Failure]: 'r_envelope' configuration is missing!");
            return (StatusCode::BAD_REQUEST, Json("Missing r_envelope")).into_response();
        }
    };

    // 3. Parse JSON envelopes into your Rust structs
    let s_envelope: Envelope = match serde_json::from_str(&s_env_raw) {
        Ok(env) => env,
        Err(e) => {
            println!("❌ [s_envelope JSON FAILURE]: Deserialization crashed! Error detail: {}", e);
            println!("Raw text received was: {}", s_env_raw);
            return (StatusCode::BAD_REQUEST, Json("Invalid s_envelope format")).into_response();
        }
    };

    let r_envelope: Envelope = match serde_json::from_str(&r_env_raw) {
        Ok(env) => env,
        Err(e) => {
            println!("❌ [r_envelope JSON FAILURE]: Deserialization crashed! Error detail: {}", e);
            println!("Raw text received was: {}", r_env_raw);
            return (StatusCode::BAD_REQUEST, Json("Invalid r_envelope format")).into_response();
        }
    };

    let crypto_payload = EncryptedMessagePayload {
        ciphertext: c_text,
        nonce: n_text,
        s_envelope,
        r_envelope,
    };

    // 4. Start Transaction
    let mut tx = match begin_rls_txn(&pool, user_id).await {
        Ok(tx) => tx,
        Err(e) => {
            println!("❌ [send_message 500 CRASH]: begin_rls_txn failed: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to start transaction: {}", e))).into_response();
        }
    };

    // 5. Manage Conversations
    let target_conv_id = match find_existing_conversation(&mut *tx, user_id, recipient_id).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            let conv = match create_conversation(&mut *tx).await {
                Ok(c) => c,
                Err(e) => {
                    println!("❌ [send_message 500 CRASH]: create_conversation failed: {:?}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string())).into_response();
                }
            };
            let _ = add_conversation_participant(&mut *tx, conv.conv_id, user_id).await;
            let _ = add_conversation_participant(&mut *tx, conv.conv_id, recipient_id).await;
            conv.conv_id
        },
        Err(e) => {
            println!("❌ [send_message 500 CRASH]: find_existing_conversation failed: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to locate conversation: {}", e))).into_response();
        }
    };

    // 6. Save Secure Payload to DB
    let message = match add_message_to_db(&mut *tx, target_conv_id, crypto_payload).await {
        Ok(m) => m,
        Err(e) => {
            println!("❌ [send_message 500 CRASH]: add_message_to_db failed: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Database transmission failure: {}", e))).into_response();
        }
    };

   
    if let Err(e) = tx.commit().await {
        println!("❌ [send_message 500 CRASH]: Failed to commit transaction: {:?}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to commit transaction: {}", e))).into_response();
    }

    (StatusCode::OK, Json(message)).into_response()
}