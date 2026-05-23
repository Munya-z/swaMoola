use leptos::prelude::*; 
use leptos_router::hooks::use_params;
use leptos_router::hooks::{use_navigate};
use leptos_router::hooks::use_location;
use leptos::serde_json; 
use reqwest::Method; 
use crate::auth::models::AuthenticatedUser; 
use crate::chats::chat_box::ChatBox;
use crate::interceptor::authenticated_fetch; 
use leptos_router::components::*;
use crate::chats::models::{ChatParams, ConversationPayload, ChatPayload, Message};
use uuid::Uuid;
use leptos::html::Ul;
use chrono::{DateTime, Utc, Datelike};
use crate::chats::chats_list::ChatsList;
use crate::chats::ws_hooks::use_websocket_listener;

fn format_chat_time(dt: &DateTime<Utc>) -> String {
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
pub fn OpenChat() -> impl IntoView {
    let navigate = use_navigate();
    let location = use_location(); 

    let user = window() 
        .local_storage() 
        .ok()
        .flatten()
        .and_then(|s| s.get_item("auth_user").ok().flatten()) 
        .and_then(|json| serde_json::from_str::<AuthenticatedUser>(&json).ok()); 
    
    let user_uuid_opt = user.as_ref().and_then(|u| Uuid::parse_str(&u.uuid.to_string()).ok()); 
    let user_uuid = user.as_ref().map(|u| u.uuid.to_string()).unwrap_or_default();
    let params = use_params::<ChatParams>();
    let user_uuid_clone = user_uuid.clone();
    
    let chat_id = move || {
        params.with(|p| {
            p.as_ref()
                .map(|p| p.id.clone())
                .unwrap_or_else(|_| "No ID found".to_string())
        })
    };

    let is_recipient_route = move || {
        location.pathname.with(|path| path.contains("/chats/c/"))
    };

    let (_resolved_conv_id, set_resolved_conv_id) = signal::<Option<Uuid>>(None);

    let message_list_ref = NodeRef::<Ul>::new();
    let refresh_trigger = Trigger::new();
    let (message_trigger_read, message_trigger_write) = signal(0);
    
    // 2. Provide the contexts so child components (like ChatsList) can use .track()
    provide_context(message_trigger_read);
    provide_context(message_trigger_write);

    // 4. Start the global background listener task
    use_websocket_listener(user_uuid.clone());


    let navigate_for_resource = navigate.clone();
    let navigate_for_name_resource = navigate.clone();

    let chat_name_resource: LocalResource<Option<ConversationPayload>> = LocalResource::new(move || {
        let navigate = navigate_for_name_resource.clone();
        let user_uuid = user_uuid.clone();
        let current_id_str = chat_id();
        let is_recipient = is_recipient_route();

        async move { 
            
            if is_recipient || current_id_str == "No ID found" {
                        return None ;
                    }

            let route_id = Uuid::parse_str(&current_id_str).unwrap_or_else(|_| Uuid::nil());
            
            set_resolved_conv_id.set(Some(route_id));

            
            let url = format!("http://localhost:8000/api/m/ch/{}", user_uuid); 
            
            let payload = ChatPayload { conv_id: route_id }; 

            let res: Result<reqwest::Response, reqwest::Error> = 
                authenticated_fetch(Method::POST, &url, navigate.clone(), Some(payload)).await; 
        
            match res { 
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        let text = resp.text().await.unwrap_or_default();
                        serde_json::from_str::<ConversationPayload>(&text).ok()
                    } else {
                        None                        
                    }
                }, 
                Err(_) => None, 
            }
        }
    });
    
    let chat_massages = LocalResource::new(move || {
        let navigate = navigate_for_resource.clone();
        let user_uuid = user_uuid_clone.clone();
        refresh_trigger.track();
        message_trigger_read.track();
        let is_recipient = is_recipient_route();
        
        let current_id_str = chat_id(); 
        
        async move { 
            if is_recipient ||current_id_str == "No ID found" {
                return None ;
            }

            let conv_id = Uuid::parse_str(&current_id_str).unwrap_or_else(|_| Uuid::nil());
            
            let url = format!("http://localhost:8000/api/m/{}", user_uuid);

            
            let payload = ChatPayload { conv_id }; 

            let res: Result<reqwest::Response, reqwest::Error> = 
                authenticated_fetch(Method::POST, &url, navigate.clone(), Some(payload)).await; 
            
            match res { 
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        let text = resp.text().await.unwrap_or_default();
                        serde_json::from_str::<Vec<Message>>(&text).ok()
                    } else {
                        None                        
                    }
                }, 
                Err(_) => None, 
            } 
        }
    });

    Effect::new(move |_| {
        if chat_massages.get().is_some() {
            request_animation_frame(move || {
                if let Some(el) = message_list_ref.get_untracked() {
                    el.set_scroll_top(el.scroll_height());
                }
            });
        }
    });


    view! {
        // App container frame mapping WhatsApp Desktop's layout configuration
        <div class="fixed inset-0 top-16 flex bg-[#f0f2f5] font-sans">
            
            // Left Chat Sidebar panel 
            <div class="hidden sm:flex sm:w-[350px] border-r border-[#e9edef] bg-white h-full overflow-y-auto">
                <ChatsList />
            </div>

            {move || match chat_id().as_str()  { 
            // Main Chat Window Space
            "No ID found" => view! {
                        <div class="flex-1 flex flex-col h-full items-center justify-center relative bg-[url(/images/chat_bg.png)] bg-center bg-opacity-40">
                            <div class="bg-white/80 backdrop-blur-sm p-6 rounded-xl shadow-sm text-center max-w-sm mx-4 border border-[#e9edef]">
                                <div class="text-[#00a884] text-4xl mb-2">"💬"</div>
                                <h3 class="text-lg font-medium text-[#111b21] mb-1">"No Active Chat"</h3>
                                <p class="text-sm text-[#667781]">"Select a conversation from the sidebar menu to start messaging."</p>
                            </div>
                        </div>
                    }.into_any(),

            _ => view! {
                <div class="flex-1 flex flex-col h-full relative bg-[url(/images/chat_bg.png)] bg-center bg-black/35 bg-blend-darken">
                
                // Header (Top Chat Info Bar)

                <nav class="h-15 bg-[#ffffff] px-4 py-2 flex items-center justify-between border-b border-[#e9edef] z-50 shrink-0">
                    <div class="flex items-center gap-3.5 min-w-0">
                        <div class="w-10 h-10 rounded-full bg-[#ffffff] flex items-center justify-center text-[#54656f] font-bold shrink-0">
                            "C"
                        </div>
                        <div class="flex flex-col min-w-0">
                            <Suspense fallback=|| view! { <span class="text-sm text-[#667781] animate-pulse">"Loading..."</span> }>
                                {move || {
                                    chat_name_resource.get().map(|data: Option<ConversationPayload>| {
                                        match data {
                                            Some(single_chat) => {
                                                let final_name = single_chat.display_name.unwrap_or_else(|| single_chat.name);
                                                view! { <span class="text-[16px] font-medium text-[#111b21] truncate">{final_name}</span> }.into_any()
                                            },
                                            None => view! { <span class="text-[16px] text-red-500 font-medium">"new conversation"</span> }.into_any(),
                                        }
                                    })
                                }}
                            </Suspense>
                            <span class="text-xs text-[#667781] truncate">"was recently active"</span>
                        </div>
                    </div>

                    <A href="/chats" attr:class="text-sm sm:hidden font-medium text-[#00a884] hover:underline px-3 py-1.5 rounded" >
                        "Back"
                    </A>
                </nav>

                // Message Thread Container
                <Suspense fallback=|| view! { <div class="flex-1 flex items-center justify-center text-[#667781] text-sm">"Loading Messages..."</div> }> 
                {move || { 
                    chat_massages.get().map(|data| { 
                        match data { 
                            Some(msgs) => view! { 
                                <ul node_ref=message_list_ref class="flex-1 overflow-y-auto px-6 pt-4 pb-24 space-y-2 scrollbar-thin"> 
                                    {msgs.into_iter().map(|msg| {
                                        let is_sender = msg.sender_id == user_uuid_opt;
                                        let formatted_time = format_chat_time(&msg.created_at);

                                        if is_sender {
                                            view! { 
                                                <div class="flex justify-end mb-1">
                                                    <div class="bg-[#d9fdd3] text-[#111b21] pl-3 pr-2 py-1.5 rounded-lg rounded-tr-none max-w-[65%] shadow-sm relative group text-[14.2px] leading-relaxed break-words">
                                                        <p class="inline-block p-4 mr-12">{msg.content}</p>
                                                        <span class="text-[11px] text-[#667781] absolute bottom-1 right-2 select-none whitespace-nowrap flex items-center gap-0.5">
                                                            {formatted_time}
                                                            <svg class="w-4 h-4 text-[#53bdeb] inline" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24">
                                                                <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7m-5 6l1 1 4-4"></path>
                                                            </svg>
                                                        </span>
                                                    </div>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { 
                                                <div class="flex justify-start mb-1">
                                                    <div class="bg-white text-[#111b21] pl-3 pr-2 py-1.5 rounded-lg rounded-tl-none max-w-[65%] shadow-sm relative text-[14.2px] leading-relaxed break-words">
                                                        <p class="inline-block p-4 mr-12">{msg.content}</p>
                                                        <span class="text-[11px] text-[#667781] absolute bottom-1 right-2 select-none whitespace-nowrap">
                                                            {formatted_time}
                                                        </span>
                                                    </div>
                                                </div>
                                            }.into_any()
                                        }
                                    }).collect_view()}
                                </ul> 
                            }.into_any(), 
                            None => view! { <div class="flex-1 flex items-center justify-center text-red-500">"Failed to load chat text history."</div> }.into_any(), 
                        } 
                    }) 
                }} 
                </Suspense>

                // Message Box Input Anchor wrapper placeholder matching your components configuration layout
                {move || {
                    let id_str = chat_id();
                    let route_uuid = Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::nil());

                    let derived_recipient_id = if is_recipient_route() {
                            route_uuid
                        } else {
                        chat_massages.get()
                            .flatten() 
                            .and_then(|messages| {
                                messages.iter()
                                    .find(|msg| msg.sender_id != user_uuid_opt) 
                                    .and_then(|msg| msg.sender_id)
                            })
                            .unwrap_or_else(Uuid::nil)
                        };

                    view! { 
                        <ChatBox 
                            recipient_id=derived_recipient_id 
                            is_recipient=is_recipient_route()
                            on_success=move || refresh_trigger.notify()
                        /> 
                    }
                }}

            </div>
            }.into_any()
        }}
    </div>
    }
}

