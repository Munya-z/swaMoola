use leptos::{prelude::*,task::spawn_local};
use leptos_router::NavigateOptions;
use crate::auth::create_ecryption_keys::generate_hybrid_identities;
use crate::auth::models::RegisterCredentials;


pub fn register_handler(
    name_signal: ReadSignal<String>,
    phone_number: ReadSignal<String>,
    password_signal: ReadSignal<String>,
    error_msg: WriteSignal<Option<String>>,
    navigate: impl Fn(&str, NavigateOptions) + Clone + 'static,
) {
    error_msg.set(None);

    let navigate = navigate.clone();
    let name_val = name_signal.get();
    let pass_val = password_signal.get();
    let phone_val = phone_number.get()
    .chars()
    .filter(|c| !c.is_whitespace())
    .collect::<String>();
    let (x_pub, pq_pub) = generate_hybrid_identities();

    let _ = dotenvy::dotenv();
    let base_url = std::env::var("BACKEND_WS_URL")
    .unwrap_or_else(|_| "http://localhost:8000".to_string());
    let api_url = format!("{base_url}/users/register");   

    spawn_local(async move {
        let client = reqwest::Client::new();
        let res = client
            .post(api_url) 
            .json(&RegisterCredentials { name: &name_val, phone_number: &phone_val, password: &pass_val , x_public: &x_pub, pq_public: &pq_pub })
            .send()
            .await;

        match res {
            Ok(response) if response.status().is_success() => {
                log::info!("Registering successful!");
                navigate("/login", Default::default());
            }
            Ok(response) => {
                let msg = format!("Registering failed with status: {}", response.status());
                log::error!("{}", msg);
                error_msg.set(Some(msg));
            }
            Err(e) => {
                let msg = format!("Network error: {}", e);
                log::error!("{}", msg);
                error_msg.set(Some(msg));
                
            }
        }
    });
}


