use uuid::Uuid;
use sqlx::PgPool;
use axum::{extract::{State,Path}, Json, http::StatusCode};
use crate::chats::models::{Conversation, ConversationParticipant, Message, AddParticipantPayload, CocoPayload};
use crate::db::begin_rls_txn;



pub async fn get_conversation_messages(
    State(pool): State<PgPool>, 
    Path(user_id): Path<Uuid>,
    Json(payload): Json<CocoPayload>
)-> Result<Json<Vec<Message>>, StatusCode>{
    let mut tx =begin_rls_txn(&pool, user_id).await.map_err(|_|
    StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let query = sqlx::query_as!(
        Message,r#"
            SELECT  msg_id as "msg_id!", conv_id as "conv_id!", sender_id as "sender_id!", content as "content!", created_at as "created_at!" FROM messages WHERE conv_id = $1
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
            c.created_at as "created_at!" FROM Conversations c
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


pub async fn get_conversation_participants(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<CocoPayload>
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


pub async fn add_new_participant_in_conversation(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<AddParticipantPayload>
) -> Result<Json<ConversationParticipant>, StatusCode> {
    let mut tx = begin_rls_txn(&pool, user_id).await.map_err(|e|{   
        println!("Failed to start transaction: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let query = sqlx::query_as!(
        ConversationParticipant,r#"
        INSERT INTO conversation_participants (conv_id, user_id)

        VALUES ($1, $2)
        RETURNING *
        "#,
        payload.conv_id,
        payload.participant_id 
    );
     let new_participant = query.fetch_one(&mut *tx) 
    .await.map_err(|e: sqlx::Error| {
        println!("Database query error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tx.commit().await.map_err(|e: sqlx::Error| {
        println!("Database query error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(new_participant))
}