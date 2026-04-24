use uuid::Uuid;
use sqlx::FromRow;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub uuid: Uuid,
    pub name: Option<String>,
    pub trust_score: Option<i32>,
    pub active_transactions: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)] 
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub phone_number_hash: String,
    pub password_hash: String,
    pub trust_score: Option<i32>,
    pub active_transactions: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)] 
pub struct RegisterRequest{
    pub name: String,
    pub phone_number: String,
    pub password: String,  
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