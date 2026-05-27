use axum::{routing::{post, get}, Router};
use sqlx::{PgPool};

pub mod conversation_handlers;
pub mod message_handlers;
pub mod ws;
pub mod models;
pub mod services;
use crate::chats::{services::{get_conversation_messages, get_user_conversations,get_conversation_participants, download_attachment}, conversation_handlers::{get_conversation_header, make_a_group_conversation, add_new_participant_in_conversation}, message_handlers::{send_message}};

pub fn routes()->Router<PgPool>{
    Router::new()
        .route("/{id}", post(get_conversation_messages))
        .route("/sm/{id}",post(send_message).put(make_a_group_conversation))
        .route("/conversations/{id}", get(get_user_conversations).post(add_new_participant_in_conversation))
        .route("/participants/{id}", get(get_conversation_participants))
        .route("/ch/{id}", post(get_conversation_header))
        .route("/attachments/{id}", get(download_attachment))
}
