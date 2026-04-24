use axum::{ routing::{post}, Router, Json, extract::{ State}, http::StatusCode,response::IntoResponse};
use sqlx::{PgPool, Transaction, Postgres};
use uuid::Uuid;
use sha2::{Sha256, Digest};
use std::{env, error::Error};
use jsonwebtoken::{encode, Header, EncodingKey};
use chrono::{Utc, Duration};
use argon2::{password_hash::{rand_core::OsRng, PasswordHasher, SaltString},Argon2, PasswordHash, PasswordVerifier};
use crate::users::models::{AuthenticatedUser, RegisterRequest, LoginRequest, AuthResponse, User};
pub mod models;

pub fn routes()->Router<PgPool>{
    Router::new()
        .route("/register", post(register_user))
        .route("/login",post(login_handler))
}

pub async fn begin_rls_txn(pool: &PgPool, user_id: Uuid)-> Result<Transaction<'_, Postgres>, Box<dyn Error>> {
    // 1. Start the transaction
    let mut tx = pool.begin().await?;

    // 2. Set the RLS variable inside this specific transaction
    // 'app.current_user_id' must match what you wrote in your SQL Policy
    sqlx::query("SELECT set_config('app.current_user_id', $1, true)")
        .bind(user_id.to_string())
        .execute(&mut *tx)
        .await?;

    Ok(tx)
}

async fn create_user(
    pool: &PgPool,
    name: String,
    phone_number: String,
    password: String,
) -> anyhow::Result<User> {

    let pepper = env::var("PHONE_PEPPER").expect("PHONE_PEPPER must be set");
    // 1. Hash the password
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
        User,r#"
        INSERT INTO users (id, name,phone_number_hash , password_hash, trust_score, active_transactions)

        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, name, trust_score as "trust_score!", active_transactions as "active_transactions!", phone_number_hash as "phone_number_hash!", password_hash as "password_hash!"
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
    let user_uuid = create_user(&pool, payload.name, payload.phone_number, payload.password)
        .await
        .map_err(|e| {
            println!("error from creating uuid : {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok((StatusCode::CREATED, Json(user_uuid)))
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
    
    // 1. Fetch user by phone number
    let user : User = sqlx::query_as!(
        User ,"SELECT id, name, phone_number_hash, password_hash ,trust_score as \"trust_score!\", active_transactions as \"active_transactions!\"  FROM users WHERE phone_number_hash = $1",
        phone_hash_hex
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("not found"))?;

    // 2. Parse the stored hash and verify
    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| anyhow::anyhow!("Invalid password hash format"))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| anyhow::anyhow!("Invalid password"))?;

    // 3. Return session data (excluding password)
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
    Json(payload): Json<LoginRequest>, // Contains phone_number and password
) -> Result<Json<AuthResponse>, StatusCode> {
    
    // 1. CALL THE VERIFY FUNCTION
    let user = verify_user(&pool, &payload.phone_number, &payload.password)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let secret = std::env::var("JWT_SECRET_KEY").expect("JWT_SECRET_KEY must be set");

    // 2. GENERATE THE TOKEN (The "Key")
    let token = generate_token(&user, &secret)
        .map_err(|e|{
            println!("error creating token key {}" , e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // 3. RETURN TO CLIENT
    Ok(Json(AuthResponse { token, user }))
}
