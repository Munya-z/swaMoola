use uuid::Uuid;
use sqlx::FromRow;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub uuid: Uuid,
    pub name: Option<String>,
    pub trust_score: Option<i32>,
    pub active_transactions: Option<i32>,
    pub discoverable_key: Option<String>,
    pub x_public: Option<String>,
    pub pq_public: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)] 
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub phone_number_hash: String,
    pub password_hash: String,
    pub trust_score: Option<i32>,
    pub active_transactions: Option<i32>,
    pub discoverable_key: String,
    pub x_public: String,
    pub pq_public: String,
}

#[derive(Serialize, sqlx::FromRow)] // Added FromRow so SQLx can map to it
pub struct UserResponse {
    pub id: Uuid,
    pub name: String,
    pub trust_score: i32,
    pub active_transactions: i32,
    pub discoverable_key: String,
    pub x_public: String,
    pub pq_public: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)] 
pub struct RegisterRequest{
    pub name: String,
    pub phone_number: String,
    pub password: String, 
    pub x_public: String,
    pub pq_public: String, 
}

#[derive(Debug, Clone, Serialize, Deserialize)] 
pub struct LoginRequest{
    pub phone_number: String,
    pub password: String,  
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: AuthenticatedUser, 
}

#[derive(Deserialize)]
pub struct DiscoverableSearchRequest {
    pub key: String,
}

#[derive(Serialize)]
pub struct DiscoverableSearchResponse {
    pub target_user_id: Uuid,
    pub name: String,
    pub x_public: String,
    pub pq_public: String,
}
