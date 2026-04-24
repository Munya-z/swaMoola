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
    let secret = std::env::var("JWT_SECRET_KEY").expect("JWT_SECRET_KEY must be set");

     match decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::new(Algorithm::HS256),
    ) {
        Ok(token_data) => Ok(token_data.claims.sub),
        Err(e) => {
            if let jsonwebtoken::errors::ErrorKind::ExpiredSignature = e.kind() {
                // Return a specific error message your middleware can catch
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

     // 1. LET OPTIONS REQUESTS PASS THROUGH
    // The CorsLayer handles these, but it needs the request to reach it!
    if req.method() == axum::http::Method::OPTIONS {
        return Ok(next.run(req).await);
    }

     println!("Step 0: Middleware hit");
     println!("All headers: {:?}", &req.headers());
    // 1. Extract Authorization header
    let auth_header = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| &h[7..]); // Skip "Bearer "

    println!("Step 1: auth header :{:?}", &auth_header);

    let token = auth_header.ok_or(StatusCode::UNAUTHORIZED)?;

    println!("Step 2: auth header :{}", &token);

    // 2. Validate JWT and extract user_id (pseudo-code)
    let user_id = validate_token_and_get_id(token).map_err(|e| {
        let err_msg = e.to_string();
        println!("Error validating token: {}", err_msg);

        if err_msg == "TOKEN_EXPIRED" {
            // You could return a custom status or handle this in your frontend
            // Many APIs use 401 for all, but you can log specifically here
            return StatusCode::UNAUTHORIZED; 
        }
        
        StatusCode::UNAUTHORIZED
    })?;

    // 3. Insert user info into Request Extensions
    req.extensions_mut().insert(AuthenticatedUser { uuid: user_id , name: None, trust_score: None , active_transactions:  None});

    // 4. Proceed to the next handler
    Ok(next.run(req).await)
}
