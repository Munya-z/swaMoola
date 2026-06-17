use uuid::Uuid;
use sqlx::PgPool;
use axum::{extract::{State,Path}, Json, http::{StatusCode, header, HeaderMap}};
use axum::{body::Body,response::IntoResponse};
use tokio_util::io::ReaderStream;
use crate::chats::models::{Conversation, ConversationParticipant, MessageReturn, ConversationIdPayload, DbEnvelope};
use crate::db::begin_rls_txn;


// get all the messages belonging to a conversation, given the conversation id and the user id of the requester.
// This will be used to display the messages in a conversation when a user opens it.
pub async fn get_conversation_messages(
    State(pool): State<PgPool>, 
    Path(user_id): Path<Uuid>,
    Json(payload): Json<ConversationIdPayload>
)-> Result<Json<Vec<MessageReturn>>, StatusCode>{
    let mut tx =begin_rls_txn(&pool, user_id).await.map_err(|_|
    StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let query = sqlx::query_as!(
        MessageReturn, r#"
        SELECT 
            m.msg_id as "msg_id!", 
            m.conv_id as "conv_id!",   
            m.created_at as "created_at!",
            m.ciphertext as "ciphertext!",
            m.nonce as "nonce!",
            m.envelopes as "envelopes!: sqlx::types::Json<Vec<DbEnvelope>>"
        FROM messages m
        WHERE m.conv_id = $1
        ORDER BY m.created_at ASC
        "#,
        payload.conv_id,
    );

    let messages: Vec<MessageReturn> = query.fetch_all(&mut *tx) 
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
    let conversations: Vec<Conversation> = query.fetch_all(&mut *tx) 
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
    let participants: Vec<ConversationParticipant> = query.fetch_all(&mut *tx) 
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
        "SELECT file_name, storage_url, file_type FROM message_attachments WHERE attachment_id = $1",
        attachment_id
    )
    .fetch_optional(&pool)
    .await;

    match record {
        Ok(Some(row)) => {
            
            let safe_filename = row.file_name.replace("..", "").replace("/", "").replace("\\", "");
            let file_path = format!("./local_cloud_storage/{}", safe_filename);

            // 2. Try to open the file from disk
            let file = match tokio::fs::File::open(&file_path).await {
                Ok(file) => file,
                Err(_) => return StatusCode::NOT_FOUND.into_response(),
            };

            // 3. Convert the file into a stream body
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);

            // 4. Set the headers
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", row.file_name).parse().unwrap(),
            );
            
            let content_type = row.file_type
                .parse()
                .unwrap_or_else(|_| "application/octet-stream".parse().unwrap());
            headers.insert(header::CONTENT_TYPE, content_type);

            // 5. Return the actual file stream
            (StatusCode::OK, headers, body).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}