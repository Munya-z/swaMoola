use axum::{Json, extract::{ State, Path}, http::StatusCode,response::IntoResponse};
use sqlx::{PgPool};
use uuid::Uuid;
use sha2::{Sha256, Digest};
use std::{env};
use jsonwebtoken::{encode, Header, EncodingKey};
use chrono::{Utc, Duration};
use rand::Rng;
use argon2::{password_hash::{rand_core::OsRng, PasswordHasher, SaltString},Argon2, PasswordHash, PasswordVerifier};

use crate::users::models::{AuthResponse, AuthenticatedUser, LoginRequest, RegisterRequest, User, UserResponse, DiscoverableSearchRequest , DiscoverableSearchResponse};

// Create a user into the database, hashing the password and phone number appropriately
async fn create_user(
    pool: &PgPool,
    name: String,
    phone_number: String,
    password: String,
    x_public: String,
    pq_public: String,
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
    let new_key = generate_discoverable_key();

    let new_user = sqlx::query_as!(
        UserResponse,r#"
        INSERT INTO users (id, name,phone_number_hash , password_hash, trust_score, active_transactions, discoverable_key, x_public, pq_public)

        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, name, trust_score as "trust_score!", active_transactions as "active_transactions!", discoverable_key as "discoverable_key!", x_public as "x_public!", pq_public as "pq_public!"
    "#,
    new_id,
    name,
    phone_number_hash,
    password_hash,
    trust_score ,
    active_transactions,
    new_key,
    x_public,
    pq_public
    )
    .fetch_one(pool) 
    .await?;

    Ok(new_user)
}

// Handler for user registration endpoint
pub async fn register_user(
    State(pool): State<PgPool>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let user = create_user(&pool, payload.name, payload.phone_number, payload.password, payload.x_public, payload.pq_public)
        .await
        .map_err(|e| {
            println!("error from creating uuid : {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok((StatusCode::CREATED, Json(user)))
}

//verify if the user has a account and if the password is correct upon login attempt
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
        User ,"SELECT id, name, phone_number_hash, password_hash ,trust_score as \"trust_score!\", active_transactions as \"active_transactions!\", discoverable_key as \"discoverable_key!\", x_public as \"x_public!\", pq_public as \"pq_public!\"   FROM users WHERE phone_number_hash = $1",
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
        discoverable_key: Some(user.discoverable_key),
        x_public: Some(user.x_public),
        pq_public: Some(user.pq_public),
    })
}

// generate JWT token for authenticated user
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

// handles login requests, verifying credentials and returning a JWT token and user data if successful
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

//  allows users to search fot others by their discoverable key, returning minimal info needed to request a connection if found
pub async fn search_by_discoverable_key(
    State(pool): State<PgPool>,
    Json(payload): Json<DiscoverableSearchRequest>,
) -> Result<Json<DiscoverableSearchResponse>, (StatusCode, String)> {

    if payload.key.len() != 12 {
        return Err((StatusCode::BAD_REQUEST, "Invalid key format".to_string()));
    }

    let result = sqlx::query!(
        "SELECT id, name, x_public, pq_public FROM users WHERE discoverable_key = $1",
        payload.key
    )
    .fetch_optional(&pool)
    .await;

    match result {
        Ok(Some(user)) => {
            Ok(Json(DiscoverableSearchResponse {
                target_user_id: user.id,
                name: user.name,
                x_public: user.x_public,
                pq_public: user.pq_public,
            }))
        }
        Ok(None) => {
            Err((StatusCode::NOT_FOUND, "No user found with that key".to_string()))
        }
        Err(_) => {
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Search failed".to_string()))
        }
    }
}

// generates a new, random 12-character discoverable key using a custom alphabet that avoids easily confused characters, ensuring uniqueness and user-friendliness.
pub fn generate_discoverable_key() -> String {
    const ALLOWED_CHARS: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let mut rng = rand::thread_rng();
    
    let key: String = (0..12)
        .map(|_| {
            let idx = rng.gen_range(0..ALLOWED_CHARS.len());
            ALLOWED_CHARS[idx] as char
        })
        .collect();

    key
}

// used to refresh a user's discoverable key, generating a new one and saving it to the database
pub async fn refresh_user_key(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    
    let new_key = generate_discoverable_key();

    let update_result = sqlx::query!(
        "UPDATE users SET discoverable_key = $1 WHERE id = $2",
        new_key,
        user_id
    )
    .execute(&pool)
    .await;

    match update_result {
        Ok(_) => (StatusCode::OK, new_key),
        Err(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate key".to_string())
        }
    }
}