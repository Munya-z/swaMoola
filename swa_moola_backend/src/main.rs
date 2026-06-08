use std::env;
use sqlx::{postgres::{PgPoolOptions, PgPool} };
use axum::{middleware as axum_middleware};
use axum::{http::{Method}, Router,routing::get};
use std::fs;
use tower_http::services::ServeDir;
use tower_http::cors::{CorsLayer};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE, UPGRADE, CONNECTION};
use axum::{extract::{Path}, http::StatusCode,body::{Body}, routing::post};
use axum::response::IntoResponse;
use crate::chats::ws::ws_handler;



pub mod db; 
mod users;
mod chats;
mod middleware;
use middleware::auth_middleware;


#[tokio::main]
async fn main() {
    
    dotenvy::dotenv().ok();

    let upload_dir = "./local_cloud_storage";
    fs::create_dir_all(upload_dir).unwrap();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL MUST BE SET");

    let pool: PgPool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await
        .expect("fail to connect");

    sqlx::migrate!().run(&pool).await.expect("migration failed");

    let public_routes = Router::new()
        .route("/", get(root))
        .nest("/users", users::routes());

    let protected_routes = Router::new()
        .nest("/uu", users::protected_routes())
        .nest("/m", chats:: routes())  
        .route("/upload/{filename}", post(handle_upload))
        .layer(axum_middleware::from_fn(auth_middleware));
    
    let cors = CorsLayer::new()
    .allow_origin([
        "http://localhost:8080".parse().unwrap(),
        "http://127.0.0.1:8080".parse().unwrap()
    ])
    .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE, UPGRADE, CONNECTION])
    .allow_credentials(true);

    let app = Router::new()
        .merge(public_routes)
        .nest("/api",protected_routes)
        .route("/api/ws/{id}", get(ws_handler))
        .nest_service("/view-files", ServeDir::new("./local_cloud_storage"))
        .layer(cors)
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    println!("server running on port 8000");
    axum::serve(listener, app).await.unwrap();

}

async fn root()->&'static str{
    "welcome to Swa Moola Api"
}

async fn handle_upload(
    Path(filename): Path<String>,
    body: Body,
) -> impl IntoResponse {
    // Sanitize the filename to prevent directory traversal vulnerabilities
    let safe_filename = filename.replace("..", "").replace("/", "").replace("\\", "");
    let file_path = format!("./local_cloud_storage/{}", safe_filename);

    let bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("Failed to read body: {}", e)).into_response(),
    };

    // Save the raw encrypted binary bytes directly to your disk folder
    _ = tokio::fs::write(&file_path, bytes)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));

    println!("Successfully saved encrypted file to: {}", file_path);
    let storage_url = format!("/view-files/{}", safe_filename);
    (StatusCode::OK, storage_url).into_response()
}