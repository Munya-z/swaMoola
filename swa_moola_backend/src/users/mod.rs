use axum::{routing::{post, put}, Router};
use sqlx::{PgPool};

pub mod handlers;
pub mod models;
pub mod services;
use crate::{models::AppState, users::{handlers::{login_handler, refresh_user_key, register_user, search_by_discoverable_key}, services::{update_user_active_transactions, update_user_trust_score}}};

pub fn routes()->Router<AppState>{
    Router::<AppState>::new()
        .route("/register", post(register_user))
        .route("/login",post(login_handler))

}

pub fn protected_routes()->Router<AppState>{
    Router::<AppState>::new()
        .route("/ts/{id}", put(update_user_trust_score))
        .route("/at/{id}", put(update_user_active_transactions))
        .route("/dk/{id}", put(refresh_user_key))
        .route("/sk/{id}", post(search_by_discoverable_key))
}




