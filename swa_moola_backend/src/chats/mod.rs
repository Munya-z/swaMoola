use axum::{routing::{post, get}, Router};

pub mod conversation_handlers;
pub mod message_handlers;
pub mod ws;
pub mod models;
pub mod services;
use crate::AppState;
use crate::chats::{services::{get_conversation_messages, get_user_conversations,get_conversation_participants, download_attachment}, conversation_handlers::{get_conversation_header, make_a_group_conversation, add_new_participant_in_conversation, create_a_new_conversation}, message_handlers::{send_message}};

pub fn routes()->Router<AppState>{
    Router::<AppState>::new()
        .route("/{id}", post(get_conversation_messages))
        .route("/sm/{id}",post(send_message).put(make_a_group_conversation))
        .route("/cg/{id}",post(make_a_group_conversation))
        .route("/conversations/{id}", get(get_user_conversations).post(add_new_participant_in_conversation))
        .route("/participants/{id}", get(get_conversation_participants))
        .route("/ch/{id}", post(get_conversation_header))
        .route("/nch/{id}", post(create_a_new_conversation))
        .route("/attachments/{id}", get(download_attachment))
        .layer(axum::extract::DefaultBodyLimit::max(70 * 1024 * 1024))
}
