use axum::{
    http::{Request, StatusCode, header},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use crate::users::models::AuthenticatedUser;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct Claims {
    sub: Uuid,
    exp: usize,
}

pub fn validate_token_and_get_id(token: &str) -> anyhow::Result<Uuid> {
    // let secret = std::env::var("JWT_SECRET_KEY").expect("JWT_SECRET_KEY must be set");
    let secret = std::env::var("JWT_SECRET_KEY")
        .map_err(|_| anyhow::anyhow!("JWT_SECRET_KEY environment variable is missing"))?;


     match decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::new(Algorithm::HS256),
    ) {
        Ok(token_data) => Ok(token_data.claims.sub),
        Err(e) => {
            if let jsonwebtoken::errors::ErrorKind::ExpiredSignature = e.kind() {
                return Err(anyhow::anyhow!("TOKEN_EXPIRED"));
            }
            Err(anyhow::anyhow!("Invalid token: {}", e))
        }
    }
}


pub async fn auth_middleware(
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    println!("Auth middleware intercepted request to: {}", req.uri());

    if req.method() == axum::http::Method::OPTIONS {
        return Ok(next.run(req).await);
    }
    println!("after i do request method of Options");

    let auth_header = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| &h[7..]); 
    println!("after auth header ");

    let token = auth_header.ok_or(StatusCode::UNAUTHORIZED)?;

    let user_id = validate_token_and_get_id(token).map_err(|e| {
        let err_msg = e.to_string();
        println!("Error validating token: {}", err_msg);

        if err_msg == "TOKEN_EXPIRED" {
            println!("error from err_msg = token expired");
            return StatusCode::UNAUTHORIZED; 
        }

        if err_msg.contains("JWT_SECRET_KEY") {
            println!("error from err_msg.contains(JWT_SECRET_KEY)");
            return StatusCode::INTERNAL_SERVER_ERROR; // Tells you the server configuration is broken
        }

        println!("error validate_token_and_get_id");    
        StatusCode::UNAUTHORIZED
    })?;

    req.extensions_mut().insert(AuthenticatedUser { uuid: user_id , name: None, trust_score: None , active_transactions:  None, discoverable_key: None, x_public: None, pq_public: None });

    let response = next.run(req).await;
    println!("Auth middleware response status: {}", response.status());
    Ok(response)
}
