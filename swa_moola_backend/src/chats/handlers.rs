use uuid::Uuid;
use sqlx::PgPool;
use axum::{extract::{State, Path}, Json, response::IntoResponse, http::StatusCode};
use crate::chats::models::{Conversation, ConversationParticipant, Message, MessagePayload, GroupPayload, ConversationNamePayload};
use crate::db::begin_rls_txn;

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
        NULL::text as "display_name"
        "#,
        false 
    );
    let new_conversation = query.fetch_one(executor) 
    .await?;

    Ok(new_conversation)
}

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
        name as "display_name"
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

pub async fn add_conversation_participant(
    executor: impl sqlx::PgExecutor<'_>, 
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

pub async fn add_message_to_db(
    executor: impl sqlx::PgExecutor<'_>, 
    user_id:Uuid, 
    conv_id: Uuid,
    content: String
) -> anyhow::Result<Message> {

    let query = sqlx::query_as!(
        Message,r#"
        INSERT INTO messages (conv_id, sender_id, content)

        VALUES ($1, $2, $3)
        RETURNING msg_id as "msg_id!", conv_id as "conv_id!", sender_id as "sender_id", content as "content!", created_at as "created_at!"
        "#,
        conv_id,
        user_id,
        content 
    );

    let new_message = query.fetch_one(executor) 
    .await?;

    Ok(new_message)
    
}

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

#[axum::debug_handler]
pub async fn send_message(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<MessagePayload>
) -> impl IntoResponse {
    let mut tx = match begin_rls_txn(&pool, user_id).await{
        Ok(tx) => tx,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to start transaction: {}", e))).into_response(),
    };

    let target_conv_id = match find_existing_conversation(&mut *tx, payload.sender_id, payload.recipient_id).await{
        Ok(Some(id)) => id ,
        Ok(None) => {
            let conv = match create_conversation(&mut *tx).await {
                Ok(c) => c ,
                Err(e) =>{
                    println!("failed to create new convewrsation {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string())).into_response()
                }
            };
            println!("Created new conversation with ID: {}", conv.conv_id);
            let _= add_conversation_participant(&mut *tx, conv.conv_id, payload.sender_id).await;
            let _= add_conversation_participant(&mut *tx, conv.conv_id, payload.recipient_id).await;
            conv.conv_id
            },
        Err(e) => {
            println!("Failed to check for existing conversation: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to check for existing conversation: {}", e))).into_response();
        },
    };


    let message = match add_message_to_db(&mut *tx, user_id, target_conv_id, payload.content).await {
        Ok(m) => {
            if let Err(e) = tx .commit().await{
                println!("Failed to commit transaction: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to commit transaction: {}", e))).into_response();
            }else{
                (StatusCode::OK, Json(m)).into_response()
            }
        },
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to add message: {}", e))).into_response(),
    };

    message
}

pub async fn get_conversation_header(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<ConversationNamePayload>
) -> impl IntoResponse {

    let mut tx = match begin_rls_txn(&pool, user_id).await{
        Ok(tx) => tx,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let convo_result = sqlx::query_as!(
        Conversation,
        r#"
        SELECT 
            c.conv_id as "conv_id!", 
            c.is_group as "is_group!", 
            c.created_at as "created_at!", 
            COALESCE(c.name, 'Untitled') as "name!", 
            (
                SELECT u.name 
                FROM conversation_participants cp
                JOIN users u ON u.id = cp.user_id
                WHERE cp.conv_id = c.conv_id AND cp.user_id != $2
                LIMIT 1
            ) as "display_name" 
        FROM conversations c
        WHERE c.conv_id = $1
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