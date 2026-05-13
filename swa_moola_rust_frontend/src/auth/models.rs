use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct RegisterCredentials<'a>{
    pub name: &'a str,
    pub phone_number: &'a str,
    pub password: &'a str,  
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub uuid: Uuid,
    pub name: Option<String>,
    pub trust_score: Option<i32>,
    pub active_transactions: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LoginResponse {
    pub token: String,
    pub user: AuthenticatedUser, 
}

#[derive(Serialize)]
pub struct LoginCredentials<'a> {
    pub phone_number: &'a str,
    pub password: &'a str,
}
