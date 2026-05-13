use axum::{routing::{post, get}, Router};
use sqlx::{PgPool};

pub mod handlers;
pub mod models;
pub mod services;
use crate::chats::{services::{get_conversation_messages, get_user_conversations,get_conversation_participants, add_new_participant_in_conversation}, handlers::{send_message, get_conversation_header, make_a_group_conversation}};

pub fn routes()->Router<PgPool>{
    Router::new()
        .route("/{id}", post(get_conversation_messages))
        .route("/sm/{id}",post(send_message).put(make_a_group_conversation))
        .route("/conversations/{id}", get(get_user_conversations).post(add_new_participant_in_conversation))
        .route("/participants/{id}", get(get_conversation_participants))
        .route("/ch/{id}", post(get_conversation_header))
}
