
use leptos::prelude::*; 
use crate::interceptor::authenticated_fetch; 
use leptos_router::hooks::use_navigate; 
use leptos::serde_json; 
use reqwest::Method;  
use uuid::Uuid;
use crate::auth::models::AuthenticatedUser; 
use crate::chats::models::{Chat, ChatPayload };

#[component]
fn ChatItem(chat: Chat, user_uuid: String) -> impl IntoView {
    let navigate = use_navigate();
    let conv_id_str = chat.conv_id.to_string();
    let redirect_conv_id = chat.conv_id;

    let navigate_for_resource = navigate.clone();
    
    let chat_name_resource = LocalResource::new(move || {
        let navigate = navigate_for_resource.clone();
        let user_uuid = user_uuid.clone();
        let conv_id = Uuid::parse_str(&conv_id_str.clone()).unwrap_or_else(|_| Uuid::nil());
        let url = format!("http://localhost:8000/api/m/ch/{}", user_uuid); 
        
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
                        serde_json::from_str::<Chat>(&text).ok()
                    } else {
                        None                        
                    }
                }, 
                Err(_) => None, 
                } 
        }
    });

    view! {
        <li on:click={
                let navigate = navigate.clone();
                move |_| {
                    let target_url = format!("/chats/{}", redirect_conv_id);
                    navigate(&target_url, Default::default());
                }
            }
            style="cursor: pointer; padding: 8px; margin: 4px 0; transition: background 0.2s;">
            <Suspense fallback=|| view! { <span>"Loading name..."</span> }>
                {move || {
                    chat_name_resource.get().map(|data| {
                        match data {
                            Some(single_chat) => {
                                view! { <span>{single_chat.display_name}</span> }.into_any()
                            },
                            None => view! { <span>"Error loading name"</span> }.into_any(),
                        }
                        
                    })
                }}
            </Suspense>
           
        </li>
    }
}

pub fn ChatsList() -> impl IntoView { 
    let navigate = use_navigate(); 
    let user = window() 
        .local_storage() 
        .ok()
        .flatten()
        .and_then(|s| s.get_item("auth_user").ok().flatten()) 
        .and_then(|json| serde_json::from_str::<AuthenticatedUser>(&json).ok()); 
    
    let user_uuid = user.as_ref().map(|u| u.uuid.to_string()).unwrap_or_default();
    let user_uuid_clone = user_uuid.clone();

    let chats_resource = LocalResource::new(move || { 
        let navigate = navigate.clone(); 
        let url = format!("http://localhost:8000/api/m/conversations/{}", user_uuid); 
        
        async move { 
            let res: Result<reqwest::Response, reqwest::Error> = 
                authenticated_fetch(Method::GET, &url, navigate.clone(), None::<()>).await; 
            
            match res { 
                Ok(resp) => resp.json::<Vec<Chat>>().await.ok(), 
                Err(_) => None, 
            } 
        } 
    }); 

    view! { 
        <h3>"chats"</h3> 
        <Suspense fallback=|| view! { <p>"Loading chats..."</p> }> 
            {move || { 
                chats_resource.get().map(|data| { 
                    match data { 
                        Some(chats) => view! { 
                            <ul> 
                                {chats.into_iter().map(|chat| { 
                                    view! { 
                                        <ChatItem chat=chat user_uuid=user_uuid_clone.clone() /> 
                                    } 
                                }).collect_view()} 
                            </ul> 
                        }.into_any(), 
                        None => view! { <p>"No chats found or error loading."</p> }.into_any(), 
                    } 
                }) 
            }} 
        </Suspense> 
    } 
}
