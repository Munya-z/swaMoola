use leptos::prelude::*; 
use crate::interceptor::authenticated_fetch; 
use leptos_router::hooks::use_navigate; 
use leptos::serde_json; 
use reqwest::Method;  
use uuid::Uuid;
use crate::auth::models::AuthenticatedUser; 
use crate::chats::search_user::SearchUser;
use crate::chats::models::{ ConversationPayloadWithStringKeys, ChatPayload, ConversationListPayload};
use chrono::{DateTime, Utc, Datelike};

fn _format_chat_time(dt: &DateTime<Utc>) -> String {
    let local_time = dt.with_timezone(&chrono::Local);
    let today = chrono::Local::now();

    if local_time.year() == today.year() 
        && local_time.month() == today.month() 
        && local_time.day() == today.day() 
    {
        local_time.format("%H:%M").to_string()
    } else {
        local_time.format("%d-%m-%y %H:%M").to_string()
    }
}

#[component]
fn ChatItem(chat: ConversationListPayload, user_uuid: String) -> impl IntoView {
    let navigate = use_navigate();
    let conv_id_str = chat.conv_id.to_string();
    let redirect_conv_id = chat.conv_id;

    let navigate_for_resource = navigate.clone();

    let chat_name_resource = LocalResource::new(move || {
        let navigate = navigate_for_resource.clone();
        let user_uuid = user_uuid.clone();
        let conv_id = Uuid::parse_str(&conv_id_str.clone()).unwrap_or_else(|_| Uuid::nil());
        let base_url = option_env!("BACKEND_URL").unwrap_or("http://localhost:8000");   
        let url = format!("{base_url}/api/m/ch/{}", user_uuid); 
        
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
                       let text = resp.text().await.unwrap_or_default();

                        if let Ok(data) = serde_json::from_str::<ConversationPayloadWithStringKeys>(&text) {

                            return Some(data); 
                        }
                        None
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
                                view! {
                                    <div class="flex items-center cursor-pointer select-none">
                                        <img class="w-12 h-12 rounded-full object-cover mr-4" src="/images/chat_bg.png" alt="Profile"/>
                                        <div class="flex-1 min-w-0 border-b border-[#f0f2f5] pb-2.5">
                                        <div class="flex justify-between items-baseline mb-1">
                                            <h3 class="text-[16px] font-medium text-[#111b21] truncate">{single_chat.display_name}</h3>
                                            // <span class="text-xs text-[#00a884] font-medium whitespace-nowrap ml-1.5">{format_chat_time(&single_chat.last_msg_date.unwrap_or_else(chrono::Utc::now))}</span>
                                        </div>
                                        <div class="flex justify-between items-center">
                                            // <p class="text-sm text-[#667781] truncate flex-1">{single_chat.last_msg_content.as_deref().unwrap_or_else(|| "No messages yet")}</p>
                                            <span class="bg-[#00a884] text-white text-xs font-semibold min-w-[20px] h-5 rounded-full flex items-center justify-center px-1 ml-2">""</span>
                                        </div>
                                        </div>
                                    </div> 
                                }.into_any()
                            },
                            None => view! { <span>"Error loading name"</span> }.into_any(),
                        }
                        
                    })
                }}
            </Suspense>
           
        </li>
    }
}

#[component]
pub fn ChatsList( 
) -> impl IntoView { 
    let navigate = use_navigate(); 
    let user = window() 
        .local_storage() 
        .ok()
        .flatten()
        .and_then(|s| s.get_item("auth_user").ok().flatten()) 
        .and_then(|json| serde_json::from_str::<AuthenticatedUser>(&json).ok()); 
    
    let user_uuid = user.as_ref().map(|u| u.uuid.to_string()).unwrap_or_default();
    let user_uuid_clone = user_uuid.clone();

    let value = navigate.clone();

    let chats_resource : LocalResource<Option<Vec<ConversationListPayload>>> = LocalResource::new(move || { 
        let navigate = value.clone(); 
        let _ = dotenvy::dotenv();
        let base_url = std::env::var("BACKEND_WS_URL")
        .unwrap_or_else(|_| "http://localhost:8000".to_string());   
        let url = format!("{base_url}/api/m/conversations/{}", user_uuid); 
        
        async move { 
            let res: Result<reqwest::Response, reqwest::Error> = 
                authenticated_fetch(Method::GET, &url, navigate.clone(), None::<()>).await; 
                
                
                match res { 
                    Ok(resp) =>{
                    if resp.status().is_success() {
                        let text = resp.text().await.unwrap_or_default();
         
                        match serde_json::from_str::<Vec<ConversationListPayload>>(&text) {
                            Ok(conversations) => Some(conversations),
                            Err(parse_err) => {
                                None
                            }
                        }
                    }else {
                        None
                    }
                }
                Err(e) => {
                    None
                } 
            }
                
        } 
    }); 

    view! { 
        <div class="max-w-[600px] w-full p-4"> 
        
        <SearchUser/>
                
        <Suspense fallback=|| view! { <p>"Loading chats..."</p> }> 
            {move || { 
                chats_resource.get().map(|data: Option<Vec<ConversationListPayload>>| { 

                    match data { 
                        Some(chats) => view! { 
                            <ul  class="max-w-[600px] w-full bg-white font-sans "> 
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
    </div>
    } 
}



