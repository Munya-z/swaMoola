use uuid::Uuid;
use sqlx::PgPool;
use axum::{extract::{State, Path}, Json, response::IntoResponse, http::StatusCode};
use crate::chats::models::{Conversation,AddParticipantPayload, ConversationParticipant, GroupPayload, ConversationResult, ConversationIdPayload};
use crate::db::begin_rls_txn;

// Create a new conversation and return its details
pub async fn create_conversation(
    executor: impl sqlx::PgExecutor<'_>, 
) -> anyhow::Result<Conversation> {

    let query = sqlx::query_as!(
        Conversation,r#"
        INSERT INTO conversations (is_group)

        VALUES ($1)
        RETURNING 
        COALESCE(name, 'New Chat') as "name!",
        conv_id as "conv_id!", 
        is_group as "is_group!", 
        created_at as "created_at!",
        NULL::text as "display_name",
        NULL::uuid as "last_message_id"
        "#,
        false 
    );
    let new_conversation = query.fetch_one(executor) 
    .await?;

    Ok(new_conversation)
}

// add a participant to a conversation used when a conversation is created or adding someone to an existing one
pub async fn add_conversation_participant(
    executor: &mut sqlx::PgConnection, 
    conv_id: Uuid,
    user_id: Uuid
) -> anyhow::Result<ConversationParticipant> {

    let query = sqlx::query_as!(
        ConversationParticipant,r#"
        INSERT INTO conversation_participants (conv_id, user_id)

        VALUES ($1, $2)
        RETURNING conv_id as "conv_id!", user_id as "user_id!"
        "#,
        conv_id,
        user_id 
    );
    let new_participant =  query.fetch_one(executor) 
    .await?;


    Ok(new_participant)
}

// Update the conversation's last_message_id when a new message is added
pub async fn add_last_message_to_conversation(
    executor: &mut sqlx::PgConnection, 
    conv_id: Uuid,
    msg_id: Uuid
) -> anyhow::Result<()> {

    
    sqlx::query!(
        r#"
        UPDATE conversations
        SET last_message_id = $2 
        WHERE conv_id = $1
        "#,
        conv_id,
        msg_id
    ).execute(executor)
    .await?;

    Ok(())
    
}

// check if a conversation already exists between two users to avoid creating duplicate conversations for one-on-one chats
pub async fn find_existing_conversation(
    executor: impl sqlx::PgExecutor<'_>, 
    user_a: Uuid,
    user_b: Uuid,
) -> anyhow::Result<Option<Uuid>> {

    let query  = sqlx::query!(
        r#"
        SELECT c.conv_id
        FROM conversations c
        JOIN conversation_participants cp1 ON c.conv_id = cp1.conv_id
        JOIN conversation_participants cp2 ON c.conv_id = cp2.conv_id
        WHERE cp1.user_id = $1 
          AND cp2.user_id = $2 
          AND c.is_group = false
        LIMIT 1
        "#,
        user_a,
        user_b
    );

    let result = query.fetch_optional(executor)
    .await?;

    Ok(result.map(|r| r.conv_id))
}

// add a memeber to an existing conversation to make it a group chat
pub async fn make_a_group_conversation(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<GroupPayload>
)-> impl IntoResponse {

    let mut tx = match begin_rls_txn(&pool, user_id).await{
        Ok(tx) => tx,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let group_convo_result   = sqlx::query_as!(
        Conversation,r#"
        UPDATE conversations
        SET name = $1, is_group = true
        WHERE conv_id = $2
        RETURNING name as "name!",
        conv_id as "conv_id!",
        is_group as "is_group!",
        created_at as "created_at!",
        name as "display_name",
        last_message_id as "last_message_id?"
        "#,
        payload.name,
        payload.conv_id
    )
    .fetch_one(&mut *tx) 
    .await;

    let convo = match group_convo_result {
        Ok(c) => c,
        Err(e) => return (StatusCode::NOT_FOUND, format!("Conversation not found: {}", e)).into_response(),
    };
    
    if let Err(e) = add_conversation_participant(&mut *tx, payload.conv_id, payload.other_user_id).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to add participant: {}", e)).into_response();
    }

    if let Err(e) = tx.commit().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to commit: {}", e)).into_response();
    }

    (StatusCode::OK, Json(convo)).into_response()

}

// get all the details of a conversation including the last message and a dynamic display name based on whether it's a group chat or one-on-one chat
pub async fn get_conversation_header(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<ConversationIdPayload>
) -> impl IntoResponse {

    let mut tx = match begin_rls_txn(&pool, user_id).await{
        Ok(tx) => tx,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let convo_result = sqlx::query_as!(
        ConversationResult,
        r#"
            SELECT 
                c.conv_id as "conv_id!", 
                c.is_group as "is_group!", 
                c.created_at as "created_at!",
                COALESCE(c.name, 'Untitled') as "name!",
                -- Subquery to dynamically set the display name based on participant or group name
                COALESCE(CASE 
                    WHEN c.is_group = TRUE THEN COALESCE(c.name, 'Untitled Group')
                    ELSE (
                        SELECT u.name 
                        FROM conversation_participants cp_other
                        JOIN users u ON u.id = cp_other.user_id
                        WHERE cp_other.conv_id = c.conv_id AND cp_other.user_id != $2
                        LIMIT 1
                    )
                END, 'Untitled')::text AS "display_name!",
                m.content AS "last_msg_content?",
                m.created_at AS "last_msg_date?",
                m.sender_id AS "last_msg_sender?",

                (CASE 
                    WHEN c.is_group = FALSE THEN (
                        SELECT cp_other.user_id 
                        FROM conversation_participants cp_other
                        WHERE cp_other.conv_id = c.conv_id AND cp_other.user_id != $2
                        LIMIT 1
                    )
                    ELSE NULL
                END) as "recipient_id?"

            FROM conversations c
            JOIN conversation_participants cp ON c.conv_id = cp.conv_id
            LEFT JOIN messages m ON c.last_message_id = m.msg_id
            WHERE c.conv_id = $1 AND cp.user_id = $2
            ORDER BY m.created_at DESC NULLS LAST;

        "#,
        payload.conv_id,
        user_id
    )
    .fetch_one(&mut *tx)
    .await;

    match convo_result {
        Ok(convo) => {
            if let Err(e) = tx.commit().await {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
            } else {
                (StatusCode::OK, Json(convo)).into_response()
            }
        }
        Err(e) => (StatusCode::NOT_FOUND, format!("Conversation not found: {}", e)).into_response(),
    }
}

// axum helper function to add a new participant to an existing conversation to make it a group chat.
// used to add someone to an existing conversation from the frontend by providing the conversation ID and the new participant's user ID.
pub async fn add_new_participant_in_conversation(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<AddParticipantPayload>
) -> Result<Json<ConversationParticipant>, StatusCode> {
    let mut tx = begin_rls_txn(&pool, user_id).await.map_err(|e|{   
        println!("Failed to start transaction: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

   
    let new_participant = add_conversation_participant(&mut *tx, payload.conv_id, payload.participant_id).await.map_err(|e| {
        println!("Failed to add participant: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tx.commit().await.map_err(|e: sqlx::Error| {
        println!("Database query error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(new_participant))
}

