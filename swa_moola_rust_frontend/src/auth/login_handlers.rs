use leptos::{prelude::*,task::spawn_local, serde_json};
use leptos_router::NavigateOptions;
use crate::auth_state::AuthState;
use crate::auth::models::{ LoginResponse, LoginCredentials };

pub fn login_handler(
    phone_number: ReadSignal<String>,
    password_signal: ReadSignal<String>,
    error_msg: WriteSignal<Option<String>>,
    auth: RwSignal<AuthState>,
    navigate: impl Fn(&str, NavigateOptions) + Clone + 'static,
) {
 
        error_msg.set(None);

        let navigate = navigate.clone(); 
        let phone_val = phone_number.get()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();
        let pass_val = password_signal.get();

        let _ = dotenvy::dotenv();
        let base_url = std::env::var("BACKEND_WS_URL")
            .unwrap_or_else(|_| "http://localhost:8000".to_string());
        let api_url = format!("{base_url}/users/login");
        
        
        spawn_local(async move {
            let client = reqwest::Client::new();
            let res : Result<reqwest::Response, reqwest::Error> = client
                .post(api_url) 
                .json(&LoginCredentials { phone_number: phone_val, password: pass_val })
                .send()
                .await;

            match res {
                Ok(response) if response.status().is_success() => {
                    let data : LoginResponse = response.json::<LoginResponse>().await.unwrap();
                    let storage = window().local_storage().unwrap().unwrap();

                    let _ = storage.set_item("auth_token", &data.token);
                    let user_json = serde_json::to_string(&data.user).unwrap();
                    let _ = storage.set_item("auth_user", &user_json);

                    log::info!("Login successful!");
                    auth.update(|state| state.token = Some(data.token));

                    navigate("/", Default::default());
                }
                Ok(response) => {
                    let msg = format!("Login failed with status: {}", response.status());
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




