use uuid::Uuid;
use sqlx::PgPool;
use axum::{extract::{State, Path}, Json, response::IntoResponse, http::StatusCode};
use crate::chats::models::{Conversation,AddParticipantPayload, ConversationParticipant, GroupPayload, ConversationResult, ConversationIdPayload};
use crate::db::begin_rls_txn;
use crate::chats::models::UserPublicKeys;
use crate::AppState;


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

use serde::Deserialize;

#[derive(Deserialize)]
pub struct NewChatPayload {
    pub recipient_id: Uuid,
}

pub async fn create_a_new_conversation(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>, 
    Json(payload): Json<NewChatPayload>, 
) -> impl IntoResponse {
    

    let pool =state.db;
    let mut tx = match begin_rls_txn(&pool, user_id).await {
        Ok(tx) => tx,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()},
    };

    let recipient_id = payload.recipient_id;

    let check_chat = find_existing_conversation(&mut *tx, user_id, recipient_id).await;

    let target_conv_id = match check_chat {
        Ok(Some(existing_id)) => {
            existing_id
        }
        Ok(None) => {
            let conv = match create_conversation(&mut *tx).await {
                Ok(c) => c,
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()},
            };

            if let Err(e) = add_conversation_participant(&mut *tx, conv.conv_id, user_id).await {
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
            }

            if let Err(e) = add_conversation_participant(&mut *tx, conv.conv_id, recipient_id).await {
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
            }

            conv.conv_id
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Database lookup failure: {}", e)).into_response();
        }
    };

    if let Err(e) = tx.commit().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to commit transaction: {}", e)).into_response();
    }

    (StatusCode::OK, Json(target_conv_id)).into_response()
}

// add a memeber to an existing conversation to make it a group chat
pub async fn make_a_group_conversation(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>, // Current logged-in user
    Json(payload): Json<GroupPayload>
) -> impl IntoResponse {

    let pool = state.db;
    println!("the make group fn is running");
    let mut tx = match begin_rls_txn(&pool, user_id).await {
        Ok(tx) => tx,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    println!("the make group fn is running and passed creating the transaction");
    // 1. Get the existing participants from the old conversation
    // (Assuming your junction table is named 'conversation_participants' with a 'user_id' column)
    let existing_participants_result = sqlx::query!(
        r#"
        SELECT user_id 
        FROM conversation_participants 
        WHERE conv_id = $1
        "#,
        payload.conv_id // The original 1-on-1 conversation ID
    )
    .fetch_all(&mut *tx)
    .await;

    
    let existing_users = match existing_participants_result {
        Ok(users) => users,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get old participants: {}", e)).into_response(),
    };
    println!("the make group fn is running and got these participants {:?}", &existing_users);
    // 2. Create a BRAND NEW conversation row
    let new_convo_id = Uuid::new_v4();
    let group_convo_result = sqlx::query_as!(
        Conversation, r#"
        INSERT INTO conversations (conv_id, name, is_group, created_at)
        VALUES ($1, $2, true, NOW())
        RETURNING name as "name!",
        conv_id as "conv_id!",
        is_group as "is_group!",
        created_at as "created_at!",
        name as "display_name",
        last_message_id as "last_message_id?"
        "#,
        new_convo_id,
        payload.name
    )
    .fetch_one(&mut *tx)
    .await;

    let convo = match group_convo_result {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create new conversation: {}", e)).into_response(),
    };

    // 3. Add the 2 original users to the brand new conversation
    for user in existing_users {
        if let Err(e) = add_conversation_participant(&mut tx, convo.conv_id, user.user_id).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to add original participant: {}", e)).into_response();
        }
    }

    // 4. Add the 3rd person (the new user) to the brand new conversation
    for participant_id in &payload.other_user_ids {
        if let Err(e) = add_conversation_participant(&mut tx, convo.conv_id, *participant_id).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR, 
                format!("Failed to add participant {}: {}", participant_id, e)
            ).into_response();
        }
    }

    match tx.commit().await {
        Ok(_) => {
            (StatusCode::OK, Json(convo)).into_response()
        },
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to commit: {}", e)).into_response()
        }
    }
}

pub async fn get_conversation_header(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<ConversationIdPayload>
) -> impl IntoResponse {
    println!("get conversation header is running");
    let pool = state.db;
    let mut tx = match begin_rls_txn(&pool, user_id).await {
        Ok(tx) => tx,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    // Notice the override syntax on "participant_keys!: Json<Vec<UserPublicKeys>>" 
    // This instructs SQLx compile-time macro how to safely map the JSONB array.
    let row_result = sqlx::query!(  
        r#"
        SELECT 
            c.conv_id as "conv_id!", 
            c.is_group as "is_group!", 
            c.created_at as "created_at!",
            c.last_message_id as "last_message_id?",
            COALESCE(c.name, 'Untitled') as "name!",
            
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

            -- Grabs the single recipient_id if it's 1-on-1 chat, otherwise returns NULL for a group
            (CASE 
                WHEN c.is_group = FALSE THEN (
                    SELECT cp_other.user_id 
                    FROM conversation_participants cp_other
                    WHERE cp_other.conv_id = c.conv_id AND cp_other.user_id != $2
                    LIMIT 1
                )
                ELSE NULL
            END) as "recipient_id?",

            -- Aggregates keys directly into your structure
            COALESCE(
                (SELECT jsonb_agg(
                    jsonb_build_object(
                        'x25519', u_other.x_public, 
                        'mlkem', u_other.pq_public
                    )
                )
                FROM conversation_participants cp_other
                JOIN users u_other ON u_other.id = cp_other.user_id
                WHERE cp_other.conv_id = c.conv_id AND cp_other.user_id != $2
                ), 
                '[]'::jsonb
            ) as "participant_keys!: sqlx::types::Json<Vec<UserPublicKeys>>"

        FROM conversations c
        JOIN conversation_participants cp ON c.conv_id = cp.conv_id
        WHERE c.conv_id = $1 AND cp.user_id = $2
        ORDER BY c.created_at DESC;
        "#,
        payload.conv_id,
        user_id
    )
    .fetch_one(&mut *tx)
    .await;

    let row = match row_result {
        Ok(r) => r,
        Err(e) => return (StatusCode::NOT_FOUND, format!("Conversation not found: {}", e)).into_response(),
    };

    println!("get conversation header is running and got here after database call and got this row : {:?}", &row);

    // Unwraps the typed wrapper structure `.0` directly into your Vec<UserPublicKeys>
    let recipient_keys: Vec<UserPublicKeys> = row.participant_keys.0; 

    println!("this is recipient_keys : {:?}", &recipient_keys);
    println!("get conversation header is running and we attempting to create a convo");

    // Map your custom type safely
    let convo = ConversationResult {
        conv_id: row.conv_id,
        is_group: row.is_group,
        created_at: row.created_at,
        name: row.name,
        display_name: Some(row.display_name),
        recipient_id: row.recipient_id, // Handled automatically by the conditional query block
        recipient_keys,
        last_message_id: row.last_message_id,
    };

    println!("get conversation header is running and convo was created : {:?}", &convo);

    if let Err(e) = tx.commit().await {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
    } else {
        println!("get conversation header is running and is going to end successfully");
        (StatusCode::OK, Json(convo)).into_response()
    }
}

// axum helper function to add a new participant to an existing conversation to make it a group chat.
// used to add someone to an existing conversation from the frontend by providing the conversation ID and the new participant's user ID.
pub async fn add_new_participant_in_conversation(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<AddParticipantPayload>
) -> Result<Json<ConversationParticipant>, StatusCode> {
    let pool = state.db;
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

