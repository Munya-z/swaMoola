use axum::{routing::{post, put}, Router};
use sqlx::{PgPool};

pub mod handlers;
pub mod models;
pub mod services;
use crate::users::{handlers::{register_user, login_handler, refresh_user_key,search_by_discoverable_key}, services::{update_user_trust_score, update_user_active_transactions}};

pub fn routes()->Router<PgPool>{
    Router::new()
        .route("/register", post(register_user))
        .route("/login",post(login_handler))

}

pub fn protected_routes()->Router<PgPool>{
    Router::new()
        .route("/ts/{id}", put(update_user_trust_score))
        .route("/at/{id}", put(update_user_active_transactions))
        .route("/dk/{id}", put(refresh_user_key))
        .route("/sk/{id}", post(search_by_discoverable_key))
}




