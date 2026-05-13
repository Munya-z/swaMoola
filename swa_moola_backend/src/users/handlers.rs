use axum::{Json, extract::{ State}, http::StatusCode,response::IntoResponse};
use sqlx::{PgPool};
use uuid::Uuid;
use sha2::{Sha256, Digest};
use std::{env};
use jsonwebtoken::{encode, Header, EncodingKey};
use chrono::{Utc, Duration};
use argon2::{password_hash::{rand_core::OsRng, PasswordHasher, SaltString},Argon2, PasswordHash, PasswordVerifier};

use crate::users::models::{AuthResponse, AuthenticatedUser, LoginRequest, RegisterRequest, User, UserResponse};

async fn create_user(
    pool: &PgPool,
    name: String,
    phone_number: String,
    password: String,
) -> anyhow::Result<UserResponse> {

    let pepper = env::var("PHONE_PEPPER").expect("PHONE_PEPPER must be set");

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("failed to hash password: {}", e))?
        .to_string();

    let trust_score = 0 ;
    let active_transactions  = 0;

    let mut hasher = Sha256::new();
    hasher.update(phone_number.as_bytes());
    hasher.update(pepper.as_bytes());
    let phone_number_hash = hex::encode(hasher.finalize());

    let new_id = Uuid::new_v4();

    let new_user = sqlx::query_as!(
        UserResponse,r#"
        INSERT INTO users (id, name,phone_number_hash , password_hash, trust_score, active_transactions)

        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, name, trust_score as "trust_score!", active_transactions as "active_transactions!"
    "#,
    new_id,
    name,
    phone_number_hash,
    password_hash,
    trust_score ,
    active_transactions 
    )
    .fetch_one(pool) 
    .await?;

    Ok(new_user)
}

pub async fn register_user(
    State(pool): State<PgPool>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let user = create_user(&pool, payload.name, payload.phone_number, payload.password)
        .await
        .map_err(|e| {
            println!("error from creating uuid : {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok((StatusCode::CREATED, Json(user)))
}

pub async fn verify_user(
    pool: &PgPool,
    phone_number: &str,
    password: &str,
) -> anyhow::Result<AuthenticatedUser> {
    let pepper = env::var("PHONE_PEPPER").expect("PHONE_PEPPER must be set");

    let mut hasher = Sha256::new();
    hasher.update(phone_number.as_bytes());
    hasher.update(pepper.as_bytes());
    let phone_hash_hex = hex::encode(hasher.finalize());

    let user : User = sqlx::query_as!(
        User ,"SELECT id, name, phone_number_hash, password_hash ,trust_score as \"trust_score!\", active_transactions as \"active_transactions!\"  FROM users WHERE phone_number_hash = $1",
        phone_hash_hex
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("not found"))?;

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| anyhow::anyhow!("Invalid password hash format"))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| anyhow::anyhow!("Invalid password"))?;

    Ok(AuthenticatedUser {
        uuid: user.id,
        name: Some(user.name),
        trust_score: user.trust_score,
        active_transactions: user.active_transactions,
    })
}

pub fn generate_token(user: &AuthenticatedUser, secret: &str) -> anyhow::Result<String> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp();

    let claims = serde_json::json!({
        "sub": user.uuid,
        "name": user.name,
        "exp": expiration,
    });

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )?;

    Ok(token)
}

pub async fn login_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<LoginRequest>, 
) -> Result<Json<AuthResponse>, StatusCode> {

    let user = verify_user(&pool, &payload.phone_number, &payload.password)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let secret = std::env::var("JWT_SECRET_KEY").expect("JWT_SECRET_KEY must be set");

    let token = generate_token(&user, &secret)
        .map_err(|e|{
            println!("error creating token key {}" , e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(AuthResponse { token, user }))
}

