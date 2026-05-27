use sqlx::{PgPool, Transaction, Postgres};
use uuid::Uuid;
use std::{ error::Error};


pub async fn begin_rls_txn(
    pool: &PgPool,
    user_id: Uuid
)-> Result<Transaction<'_, Postgres>, Box<dyn Error>> {
    // Start the transaction
    let mut tx = pool.begin().await?;

    // Set the RLS variable inside this specific transaction
    sqlx::query("SELECT set_config('app.current_user_id', $1, true)")
        .bind(user_id.to_string())
        .execute(&mut *tx)
        .await?;

    Ok(tx)
}