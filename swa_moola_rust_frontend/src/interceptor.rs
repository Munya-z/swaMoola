use reqwest::{ Client};
use reqwest::Method;
use once_cell::sync::Lazy;
use leptos_router::{NavigateOptions};
use reqwest::multipart::Form;
use serde::Serialize;
use leptos::prelude::*;

static CLIENT: Lazy<reqwest::Client> = Lazy::new(Client::new);

fn get_token() -> Option<String> {
    window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|s| s.get_item("auth_token").ok().flatten())
}

pub async fn authenticated_fetch<F, B>(
    method: Method,
    url: &str,
    navigate: F,
    body: Option<B>
) -> Result<reqwest::Response, reqwest::Error>
    where 
        F: Fn(&str, NavigateOptions) + Clone, 
        B: Serialize, 
    {
    let mut request_builder = CLIENT.request(method, url);

    // let navigate = use_navigate();

    if let Some(token) = get_token() {
        request_builder = request_builder.bearer_auth(token);
    }

    if let Some(b) = body {
        request_builder = request_builder.json(&b);
    }

    let response: reqwest::Response  =  request_builder.send().await?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED { 
        let storage = window().local_storage().unwrap().unwrap();
        
        let _ = storage.remove_item("auth_token");
        let _ = storage.remove_item("auth_user");
        navigate("/login", Default::default());
    }

    Ok(response)
}


pub async fn authenticated_multipart_fetch(
    method: Method,
    url: &str,
    navigate: impl Fn(&str, NavigateOptions) + Clone + 'static,
    form_payload: Option<Form>, 
) -> Result<reqwest::Response, reqwest::Error> {

    let mut req = CLIENT.request(method, url);

    if let Some(token) = get_token() {
        req = req.bearer_auth(token);
    }

    if let Some(form) = form_payload {
        req = req.multipart(form);
    }
    
    let response =req.send().await?;
    
    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        let storage = window().local_storage().unwrap().unwrap();
        
        let _ = storage.remove_item("auth_token");
        let _ = storage.remove_item("auth_user");
        navigate("/login", Default::default());
    }

    Ok(response)
}