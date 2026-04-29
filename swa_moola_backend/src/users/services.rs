use axum::{Json, extract::{ State, Path}, http::StatusCode};
use serde::{Serialize, Deserialize};
use sqlx::{PgPool, Type};
use uuid::Uuid;

use crate::db::begin_rls_txn;
use crate::users::models::{ UserResponse};

#[derive(Deserialize)]
pub struct ScoreUpdate {
    pub score: i32,
}

#[derive(Deserialize)]
pub struct TransactionUpdate {
    pub option: TransactionUpdateOption,
}

#[derive(Serialize, Deserialize, Debug, Clone,Type)]
#[sqlx(type_name = "TEXT")] 
#[sqlx(rename_all = "lowercase")] 
pub enum TransactionUpdateOption {
    Start,
    End,
}

pub async fn update_user_trust_score(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<ScoreUpdate>
) -> Result<(StatusCode, Json<UserResponse>), StatusCode>{
    
    let mut tx = begin_rls_txn(&pool, user_id)
    .await
    .map_err(|e| {
        println!("error from beginning transaction in update user trust score : {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let updated_user: UserResponse = sqlx::query_as::<_, UserResponse>(  r#"
            UPDATE users SET trust_score = trust_score + $1 WHERE id = $2 
            RETURNING id, name, trust_score , active_transactions 
        "#)
        .bind(payload.score)
        .bind(user_id)
        .fetch_one(tx.as_mut())
        .await
        .map_err(|e|{
            println!("error from updating user trust score : {}", e);
            StatusCode::INTERNAL_SERVER_ERROR   
        })?;

    tx.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(( StatusCode::OK, Json(updated_user)))
} 

pub async fn update_user_active_transactions(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    Json(payload) : Json<TransactionUpdate>
) -> Result<(StatusCode, Json<UserResponse>), StatusCode>{
    
    let mut tx = begin_rls_txn(&pool, user_id)
    .await
    .map_err(|e| {
        println!("error from beginning transaction in update user active transactions : {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let active_transaction_change = match payload.option {
        TransactionUpdateOption::Start => 1,
        TransactionUpdateOption::End => -1,
    };

    let updated_user: UserResponse = sqlx::query_as::<_, UserResponse>(r#"
            UPDATE users SET active_transactions = active_transactions + $1 WHERE id = $2 
            RETURNING id, name, trust_score, active_transactions
        "#)
        .bind(active_transaction_change)
        .bind(user_id)
        .fetch_one(tx.as_mut())
        .await
        .map_err(|e|{
            println!("error from updating active transactions : {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(( StatusCode::OK, Json(updated_user)))
}