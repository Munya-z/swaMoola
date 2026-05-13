
use leptos::prelude::*; 
use leptos_router::hooks::use_params;
use leptos_router::hooks::use_navigate;
use leptos::serde_json; 
use reqwest::Method; 
use crate::auth::models::AuthenticatedUser; 
use crate::interceptor::authenticated_fetch; 
use crate::chats::models::{ChatParams, ChatPayload, Chat, Message};
use leptos_router::params::Params;
use uuid::Uuid;


#[component]
pub fn OpenChat() -> impl IntoView {
    let navigate = use_navigate();
    let user = window() 
        .local_storage() 
        .ok()
        .flatten()
        .and_then(|s| s.get_item("auth_user").ok().flatten()) 
        .and_then(|json| serde_json::from_str::<AuthenticatedUser>(&json).ok()); 
    
    let user_uuid = user.as_ref().map(|u| u.uuid.to_string()).unwrap_or_default();
    let user_uuid_clone = user_uuid.clone();
    let params = use_params::<ChatParams>();
    
    // Derived signal that extracts the active chat ID string
    let chat_id = move || {
        params.with(|p| {
            p.as_ref()
                .map(|p| p.id.clone())
                .unwrap_or_else(|_| "No ID found".to_string())
        })
    };


    let navigate_for_resource = navigate.clone();
    
    let chat_massages = LocalResource::new(move || {
        let navigate = navigate_for_resource.clone();
        let user_uuid = user_uuid.clone();
        
        let current_id_str = chat_id(); 
        let conv_id = Uuid::parse_str(&current_id_str).unwrap_or_else(|_| Uuid::nil());
        
        let url = format!("http://localhost:8000/api/m/{}", user_uuid);
        
        let payload = ChatPayload { 
            conv_id
        }; 

        async move { 
            let res: Result<reqwest::Response, reqwest::Error> = 
                authenticated_fetch(Method::POST, &url, navigate.clone(), Some(payload)).await; 
            
            match res { 
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        let text =  resp.text().await.unwrap_or_default();
                        serde_json::from_str::<Vec<Message>>(&text).ok()
                    } else {
                        None                        
                    }
                }, 
                Err(_) => None, 
                } 
        }
    });


    view! {
        <div class="chat-room">
            <h2>"Active Conversation"</h2>
            <p>"Loading data for Chat ID: " {chat_id}</p>
            
            <Suspense fallback=|| view! { <p>"Loading chats..."</p> }> 
            {move || { 
                chat_massages.get().map(|data| { 
                    match data { 
                        Some(msgs) => view! { 
                            <ul> 
                                {msgs.into_iter().map(|msg| { 
                                    view! { 
                                        <li><p>{msg.content}</p></li> 
                                    } 
                                }).collect_view()} 
                            </ul> 
                        }.into_any(), 
                        None => view! { <p>"No chats found or error loading."</p> }.into_any(), 
                    } 
                }) 
            }} 
        </Suspense> 
        </div>
    }
}