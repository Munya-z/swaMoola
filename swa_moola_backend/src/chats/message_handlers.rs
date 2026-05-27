use uuid::Uuid;
use sqlx::PgPool;
use axum::{extract::{State, Path}, Json, response::IntoResponse, http::StatusCode, extract::Multipart};

// imported crates from this curent project 
use crate::db::begin_rls_txn;
use crate::chats::models::{Message, AttachmentMetadata};
use crate::chats::conversation_handlers::{add_conversation_participant, find_existing_conversation, add_last_message_to_conversation, create_conversation};

// adds a new message to the database and updates the conversation's last_message_id accordingly.
async fn add_message_to_db(
    executor: &mut sqlx::PgConnection, 
    user_id:Uuid, 
    conv_id: Uuid,
    content: String
) -> anyhow::Result<Message> {

    let db_content = if content.trim().is_empty() {
        None
    } else {
        Some(content)
    };

    let query = sqlx::query_as!(
        Message,r#"
        INSERT INTO messages (conv_id, sender_id, content)

        VALUES ($1, $2, $3)
        RETURNING msg_id as "msg_id!", conv_id as "conv_id!", sender_id as "sender_id?", content as "content?", created_at as "created_at!", '[]'::json as "attachments!: sqlx::types::Json<Vec<AttachmentMetadata>>"
        "#,
        conv_id,
        user_id,
        db_content 
    );


    let new_message =query.fetch_one(&mut *executor)
    .await?;

    let _ = add_last_message_to_conversation(&mut *executor, conv_id, new_message.msg_id).await?;

    Ok(new_message)
    
}

// send message from the frontend,
// if a conversation between the sender and recipient already exists it will add the message to that conversation
// otherwise it will create a new conversation and add the message there
// uses a transaction to ensure that all database operations are atomic and consistent.
// pub async fn send_message(
//     State(pool): State<PgPool>,
//     Path(user_id): Path<Uuid>,
//     Json(payload): Json<MessagePayload>
// ) -> impl IntoResponse {
//     let mut tx = match begin_rls_txn(&pool, user_id).await{
//         Ok(tx) => tx,
//         Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to start transaction: {}", e))).into_response(),
//     };

//     let target_conv_id = match find_existing_conversation(&mut *tx, payload.sender_id, payload.recipient_id).await{
//         Ok(Some(id)) => id ,
//         Ok(None) => {
//             let conv = match create_conversation(&mut *tx).await {
//                 Ok(c) => c ,
//                 Err(e) =>{
//                     return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string())).into_response()
//                 }
//             };
//             let _= add_conversation_participant(&mut *tx, conv.conv_id, payload.sender_id).await;
//             let _= add_conversation_participant(&mut *tx, conv.conv_id, payload.recipient_id).await;
//             conv.conv_id
//             },
//         Err(e) => {
//             return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to check for existing conversation: {}", e))).into_response();
//         },
//     };


//     let message = match add_message_to_db(&mut *tx, user_id, target_conv_id, payload.content).await {
//         Ok(m) => {
//             if let Err(e) = tx .commit().await{
//                 return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to commit transaction: {}", e))).into_response();
//             }else{
//                 (StatusCode::OK, Json(m)).into_response()
//             }
//         },
//         Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to add message: {}", e))).into_response(),
//     };

//     message
// }


pub async fn send_message(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    mut multipart: Multipart, 
) -> impl IntoResponse {
    let mut sender_id = None;
    let mut recipient_id = None;
    let mut content = String::new();
    let mut files: Vec<(String, String, Vec<u8>)> = Vec::new();

    // 3. Extract text fields and files sequentially from the stream
    while let Ok(Some(field)) = multipart.next_field().await {
        if let Some(name) = field.name() {
            match name {
                "sender_id" => {
                    if let Ok(text) = field.text().await {
                        sender_id = Uuid::parse_str(&text).ok();
                    }
                }
                "recipient_id" => {
                    if let Ok(text) = field.text().await {
                        recipient_id = Uuid::parse_str(&text).ok();
                    }
                }
                "content" => {
                    if let Ok(text) = field.text().await {
                        content = text;
                    }
                }
                "files" => {
                    let file_name = field.file_name().unwrap_or("unnamed").to_string();
                    let content_type = field.content_type()
                        .unwrap_or("application/octet-stream")
                        .to_string();
                    if let Ok(bytes) = field.bytes().await {
                        if !bytes.is_empty() {
                            files.push((file_name, content_type, bytes.to_vec()));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // 4. Validate that mandatory IDs were extracted
    let (sender_id, recipient_id) = match (sender_id, recipient_id) {
        (Some(s), Some(r)) => (s, r),
        _ => return (StatusCode::BAD_REQUEST, Json("Missing sender_id or recipient_id")).into_response(),
    };

    // --- YOUR EXISTING TRANSACTION LOGIC (UNCHANGED) ---
    let mut tx = match begin_rls_txn(&pool, user_id).await {
        Ok(tx) => tx,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to start transaction: {}", e))).into_response(),
    };

    let target_conv_id = match find_existing_conversation(&mut *tx, sender_id, recipient_id).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            let conv = match create_conversation(&mut *tx).await {
                Ok(c) => c,
                Err(e) => {return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string())).into_response()}
            };
            let _ = add_conversation_participant(&mut *tx, conv.conv_id, sender_id).await;
            let _ = add_conversation_participant(&mut *tx, conv.conv_id, recipient_id).await;
            conv.conv_id
        },
        Err(e) => {
                println!("❌ TRANSMISSION FAILURE IN find_existing_conversation: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to check for existing conversation: {}", e))).into_response()},
    };

    // 5. Save the message text payload to your database
    let message = match add_message_to_db(&mut *tx, user_id, target_conv_id, content).await {
        Ok(m) => m,
        Err(e) => {
             println!("❌ TRANSMISSION FAILURE IN add_message_to_db: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to add message: {}", e))).into_response()},
    };

    // 6. TODO: Save your file attachment binaries!
    for (file_name, file_type, file_bytes) in files {
        let file_size = file_bytes.len() as i32;

        // Execute the insert into your new message_attachments BYTEA column
        let attachment_result = sqlx::query!(
            "INSERT INTO message_attachments (msg_id, file_name, file_data, file_size, file_type) 
             VALUES ($1, $2, $3, $4, $5)",
            message.msg_id,
            file_name,
            file_bytes, // SQLx automatically maps Vec<u8> into a Postgres BYTEA type
            file_size, 
            file_type
        )
        .execute(&mut *tx)
        .await;

        if let Err(e) = attachment_result {
             println!("❌ TRANSMISSION FAILURE IN message_attachments INSERT: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to save database attachment: {}", e))).into_response();
        }

        println!("Received file: {}, size: {} bytes", file_name, file_bytes.len());
    }

    // 7. Commit database changes
    if let Err(e) = tx.commit().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to commit transaction: {}", e))).into_response();
    }

    (StatusCode::OK, Json(message)).into_response()
}