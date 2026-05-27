use std::env;
use sqlx::{postgres::{PgPoolOptions, PgPool} };
use axum::{middleware as axum_middleware};
use axum::{http::{Method}, Router,routing::get};
use tower_http::cors::{CorsLayer};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE, UPGRADE, CONNECTION};
use crate::chats::ws::ws_handler;

pub mod db; 
mod users;
mod chats;
mod middleware;
use middleware::auth_middleware;


#[tokio::main]
async fn main() {
    
        dotenvy::dotenv().ok();

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
        // .fallback_service(ServeDir::new("dist/your-angular-project/browser"))
        .route("/api/ws/{id}", get(ws_handler))
        .layer(cors)
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    println!("server running on port 8000");
    axum::serve(listener, app).await.unwrap();

}

async fn root()->&'static str{
    "welcome to Swa Moola Api"
}
