use uuid::Uuid;
use sqlx::PgPool;
use axum::{extract::{State,Path}, Json, http::{StatusCode, header, HeaderMap}};
use crate::chats::models::{Conversation, ConversationParticipant, Message, ConversationIdPayload, AttachmentMetadata };
use axum::response::IntoResponse;
use crate::db::begin_rls_txn;


// get all the messages belonging to a conversation, given the conversation id and the user id of the requester.
// This will be used to display the messages in a conversation when a user opens it.
pub async fn get_conversation_messages(
    State(pool): State<PgPool>, 
    Path(user_id): Path<Uuid>,
    Json(payload): Json<ConversationIdPayload>
)-> Result<Json<Vec<Message>>, StatusCode>{
    let mut tx =begin_rls_txn(&pool, user_id).await.map_err(|_|
    StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let query = sqlx::query_as!(
        Message,r#"
        SELECT 
            m.msg_id as "msg_id!", 
            m.conv_id as "conv_id!", 
            m.sender_id as "sender_id?", 
            m.content as "content?", 
            m.created_at as "created_at!",
            COALESCE(
                json_agg(
                    json_build_object(
                        'attachment_id', a.attachment_id,
                        'file_name', a.file_name,
                        'file_size', a.file_size,
                        'file_type', a.file_type
                    )
                ) FILTER (WHERE a.attachment_id IS NOT NULL),
                '[]'
            ) as "attachments!: _"
        FROM messages m
        LEFT JOIN message_attachments a ON m.msg_id = a.msg_id
        WHERE m.conv_id = $1
        GROUP BY m.msg_id
        ORDER BY m.created_at ASC
        "#,
        payload.conv_id,
    );
    let messages = query.fetch_all(&mut *tx) 
    .await.map_err(|e: sqlx::Error| {
        println!("Database query error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tx.commit().await.map_err(|e: sqlx::Error| {
        println!("Database query error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(messages))
    
}

//get all the conversations that a user is a part of, given the user id of the requester. This will be used to display the list of conversations in the front 
pub async fn get_user_conversations(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>
)-> Result<Json<Vec<Conversation>>, StatusCode>{
    let mut tx = begin_rls_txn(&pool, user_id).await.map_err(|e|{   
        println!("Failed to start transaction: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let query = sqlx::query_as!(
        Conversation,r#"
            SELECT c.conv_id as "conv_id!", 
            c.is_group as "is_group!", 
            COALESCE(c.name, 'Untitled') as "name!",
            c.name as "display_name", 
            c.created_at as "created_at!",
            c.last_message_id as "last_message_id?"
            FROM Conversations c
            JOIN Conversation_participants cp ON c.conv_id = cp.conv_id
            WHERE cp.user_id = $1
        "#,
        user_id,
    );
    let conversations = query.fetch_all(&mut *tx) 
    .await.map_err(|e: sqlx::Error| {
        println!("Database query error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tx.commit().await.map_err(|e: sqlx::Error| {
        println!("Database query error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(conversations))
    
}

// get all the participants of a conversation, given the conversation id and the user id of the requester.
pub async fn get_conversation_participants(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<ConversationIdPayload>
)-> Result<Json<Vec<ConversationParticipant>>, StatusCode>{
    let mut tx = begin_rls_txn(&pool, user_id).await.map_err(|e|{   
        println!("Failed to start transaction: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let query = sqlx::query_as!(
        ConversationParticipant,r#"
            SELECT conv_id as "conv_id!", 
            user_id as "user_id!" FROM Conversation_participants WHERE conv_id = $1
        "#,
        payload.conv_id,
    );
    let participants = query.fetch_all(&mut *tx) 
    .await.map_err(|e: sqlx::Error| {
        println!("Database query error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tx.commit().await.map_err(|e: sqlx::Error| {
        println!("Database query error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(participants))
    
}

pub async fn download_attachment(
    State(pool): State<PgPool>,
    Path(attachment_id): Path<Uuid>,
) -> impl IntoResponse {
   
    let record = sqlx::query!(
        "SELECT file_name, file_data, file_type FROM message_attachments WHERE attachment_id = $1",
        attachment_id
    )
    .fetch_optional(&pool)
    .await;

    match record {
        Ok(Some(row)) => {
            
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", row.file_name).parse().unwrap(),
            );
            if let Ok(content_type) = row.file_type.parse() {
                headers.insert(header::CONTENT_TYPE, content_type);
            } else {
                headers.insert(header::CONTENT_TYPE, "application/octet-stream".parse().unwrap());
            }

            
            (StatusCode::OK, headers, row.file_data).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}