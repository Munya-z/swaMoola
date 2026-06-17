use leptos::prelude::*;
use leptos_router::components::*;
use uuid::Uuid;
use reqwest::Method; 
use chrono::{DateTime, Utc, Datelike};
use leptos::{serde_json, html::Ul};
use leptos_router::{NavigateOptions,hooks::{use_navigate, use_params, use_location}};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use crate::chats::models::{SecretInnerPayload};

use crate::auth::create_ecryption_keys::load_private_keys_locally;
use crate::auth::models::AuthenticatedUser; 
use crate::chats::message_input::message_input::MessageInput;
use crate::interceptor::authenticated_fetch; 
use crate::chats::models::{ChatParams, ConversationPayload, SearchResult,ChatPayload, Attachment, InboundMessagePayload};
use crate::chats::chats_list::ChatsList;
use crate::chats::ws_hooks::use_websocket_listener;
use crate::chats::message_bubble::message_viewer::MessageViewer;
use crate::chats::message_decryption::decrypt_message_payload::decrypt_message_with_fallback;
use crate::chats::models::UserPublicKeys;

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

pub fn get_chat_messages(
    navigate: impl Fn(&str, NavigateOptions) + Clone + 'static,
    user_uuid: String,
    current_id_fn: impl Fn() -> String + Send + Sync + 'static, 
    refresh_trigger: Trigger,
    message_trigger_read: ReadSignal<i32>,
) -> LocalResource<Option<Vec<SecretInnerPayload>>> {
    LocalResource::new(move || {
        let navigate = navigate.clone();
        let user_uuid = user_uuid.clone();
        let current_id_str = current_id_fn();
      
        refresh_trigger.track();
        message_trigger_read.track();
        
        let my_keys = load_private_keys_locally()
            .expect("Critical error: Local cryptographic identity state missing from device memory!");

        async move { 
            if current_id_str == "No ID found" || current_id_str.trim().is_empty() {
                return None;
            }

            let conv_id = Uuid::parse_str(&current_id_str).unwrap_or_else(|_| Uuid::nil());
            let url = format!("http://localhost:8000/api/m/{}", user_uuid);
            let payload = ChatPayload { conv_id }; 

            let res = authenticated_fetch(Method::POST, &url, navigate, Some(payload)).await; 
            
            match res { 
                Ok(resp) => {
                    if resp.status().is_success() {
                        let text = resp.text().await.unwrap_or_default();

                        let payloads = match serde_json::from_str::<Vec<InboundMessagePayload>>(&text) {
                            Ok(parsed) => parsed,
                            Err(_e) => {
                                return None;
                            }
                        };

                        let decrypted_msgs: Vec<SecretInnerPayload> = payloads.into_iter().filter_map(|msg_payload| {
                            match decrypt_message_with_fallback(&msg_payload, &my_keys.x25519_private, &my_keys.mlkem_private) {
                                Ok(decrypted) => {
                                    log::info!("decrypted payloag {:?}", &decrypted);
                                    Some(decrypted)
                                },
                                Err(_crypto_err) => {
                                    None 
                                }
                            }
                        }).collect();
                        
                        Some(decrypted_msgs)
                    } else {
                        None                        
                    }
                }, 
                Err(_) => {
                    None
                }, 
            } 
        }
    })
}

pub fn get_chat_name(
    navigate: impl Fn(&str, NavigateOptions) + Clone + 'static,
    user_uuid: String,
    current_id_fn: impl Fn() -> String + Send + Sync + 'static,
    is_recipient_fn: impl Fn() -> bool + Send + Sync + 'static,
    set_resolved_conv_id: WriteSignal<Option<Uuid>>,
) -> LocalResource<Option<ConversationPayload>> {
    LocalResource::new(move || {
        let navigate = navigate.clone();
        let user_uuid = user_uuid.clone();
        let current_id_str = current_id_fn();
        let is_recipient = is_recipient_fn();

        async move { 
            if is_recipient || current_id_str == "No ID found" {

                match get_and_clear_search_result(){
                    Ok(Some(data))=>{
                        return Some(ConversationPayload{
                            conv_id: Uuid::nil(),
                            is_group: false,
                            created_at: chrono::Utc::now(),
                            name: String::new(),
                            display_name: Some(data.name),
                            recipient_id: Some(data.target_user_id), 
                            x_public: Some(data.x_public),
                            pq_public: Some(data.pq_public),
                            last_msg_id: None,
                        });
                    }
                    Ok(None)=> {
                        return None;
                    }
                    Err(_e) => {
                        return None;
                    }
                }
            }

            let route_id = Uuid::parse_str(&current_id_str).unwrap_or_else(|_| Uuid::nil());
            
            set_resolved_conv_id.set(Some(route_id));

            let url = format!("http://localhost:8000/api/m/ch/{}", user_uuid); 
            let payload = ChatPayload { conv_id: route_id }; 

            let res = authenticated_fetch(Method::POST, &url, navigate, Some(payload)).await; 
        
            match res { 
                Ok(resp) => {
                    if resp.status().is_success() {
                        let text = resp.text().await.unwrap_or_default();
                        serde_json::from_str::<ConversationPayload>(&text).ok()
                    } else {
                        None                        
                    }
                }, 
                Err(_) => None, 
            }
        }
    })
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

    let user_x_public = user
        .as_ref()
        .and_then(|u| u.x_public.as_ref())
        .and_then(|b64_str| STANDARD.decode(b64_str).ok()) 
        .and_then(|raw_bytes| raw_bytes.try_into().ok())   
        .unwrap_or([0u8; 32]);
        
    let user_pq_public = user
        .as_ref()
        .and_then(|u| u.pq_public.as_ref())
        .and_then(|b64_str| STANDARD.decode(b64_str).ok())
        .and_then(|bytes| bytes.try_into().ok())
        .unwrap_or([0u8; 1184]);

    let user_uuid_opt = user.as_ref().and_then(|u| Uuid::parse_str(&u.uuid.to_string()).ok()); 
    let user_uuid = user.as_ref().map(|u| u.uuid.to_string()).unwrap_or_default();
    let params = use_params::<ChatParams>();
    
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
    
    provide_context(message_trigger_read);
    provide_context(message_trigger_write);
    
    use_websocket_listener(user_uuid.clone());
    
    let navigate_for_name_resource = navigate.clone();
    
    let user_uuid_clone = user_uuid.clone();

    let chat_name_resource: LocalResource<Option<ConversationPayload>> = get_chat_name(
        navigate_for_name_resource,
        user_uuid_clone,
        chat_id,
        is_recipient_route,
        set_resolved_conv_id,
    );

    let chat_messages = get_chat_messages(
        navigate.clone(),
        user_uuid.clone(),
        chat_id, 
        refresh_trigger,
        message_trigger_read,   
    );

    Effect::new(move |_| {
        let has_messages = chat_messages.with(|msgs_res| {
            msgs_res.as_ref()                    
                .and_then(|inner_opt| inner_opt.as_ref()) 
                .map_or(false, |m| !m.is_empty()) 
        });

        if has_messages {

            let list_ref = message_list_ref;
            
            request_animation_frame(move || {
                if let Some(el) = list_ref.get_untracked() {
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
                <div class="flex-1 flex flex-col h-full relative bg-[url(/images/chat_bg.png)] bg-center bg-cover">
                <div class="absolute inset-0 bg-black/35 pointer-events-none z-0"></div>
                
                <nav class="h-15 bg-[#ffffff] px-4 py-2 flex items-center justify-between border-b border-[#e9edef] z-30 shrink-0">
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
                    chat_messages.get().map(|data: Option<Vec<SecretInnerPayload>>| { 
                        match data { 
                            Some(msgs) => view! { 
                                <ul node_ref=message_list_ref class="flex-1 overflow-y-auto z-10 px-6 pt-4 pb-24 space-y-2 scrollbar-thin"> 
                                    {msgs.into_iter().map(|msg| {
                                        let is_sender = Some(msg.sender_id) == user_uuid_opt;
                                        let formatted_time = format_chat_time(&msg.timestamp_ms);

                                        let msg_content = Some(msg.text_message.clone());
                                        let msg_attachments = msg.attachments.into_iter().map(|meta| {
                                            Attachment {
                                                file_name: meta.file_name,
                                                file_type: meta.file_type,
                                                file_size: meta.file_size,
                                                storage_url: meta.storage_url,
                                                file_key: meta.file_key,
                                                nonce_base: meta.nonce_base,
                                            }
                                        }).collect();

                                        if is_sender {
                                            view! { 
                                                <div class="flex justify-end mb-1">
                                                    <div class="bg-[#d9fdd3] text-[#111b21] pl-0 pr-0 py-1.5 rounded-lg rounded-tr-none max-w-[65%] shadow-sm relative group text-[14.2px] leading-relaxed break-words">
                                                        <MessageViewer msg_content=msg_content msg_attachments=msg_attachments /> 
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
                                                    <div class="bg-white text-[#111b21] pl-0 pr-0 py-1.5 rounded-lg rounded-tl-none max-w-[65%] shadow-sm relative text-[14.2px] leading-relaxed break-words">
                                                        <MessageViewer msg_content=msg_content msg_attachments=msg_attachments />
                                                        <span class="text-[11px] text-[#667781] absolute bottom-1 right-2 select-none whitespace-nowrap flex items-center gap-0.5">
                                                            {formatted_time}
                                                            <svg class="w-4 h-4 text-[#53bdeb] inline" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24">
                                                                <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7m-5 6l1 1 4-4"></path>
                                                            </svg>
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
                    
                    // 👇 Read the payload dynamically inside the signal view closure
                    let current_payload = chat_name_resource.get().flatten();

                    let derived_recipient_id = if is_recipient_route() {
                        route_uuid
                    } else {
                        current_payload.as_ref()
                            .and_then(|payload| payload.recipient_id)
                            .unwrap_or_else(|| Uuid::nil())
                    };

                    view! { 
                        <Show
                            when=move || chat_name_resource.get().flatten().is_some()
                            fallback=move || view! { <p>"Loading cryptographic session keys..."</p> }
                        >
                            {
                                let payload = chat_name_resource.get().flatten().unwrap();
                                
                                let recipients: Vec<UserPublicKeys> = vec![
                                    UserPublicKeys::new(
                                        payload.x_public.as_ref(), 
                                        payload.pq_public.as_ref()
                                    )
                                ];

                                view! {
                                    <MessageInput 
                                        recipient_id=derived_recipient_id 
                                        is_recipient=is_recipient_route()
                                        on_success=Callback::new(move |_| refresh_trigger.notify())
                                        recipients
                                        s_x25519=user_x_public
                                        s_mlkem=user_pq_public
                                    /> 
                                }
                            }
                        </Show>
                    }
                }}

            </div>
            }.into_any()
        }}
    </div>
    }
}

pub fn get_and_clear_search_result() -> Result<Option<SearchResult>, wasm_bindgen::JsValue> {
    let storage = web_sys::window()
        .and_then(|w| w.session_storage().ok())
        .flatten()
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("Session storage is not available"))?; 

    if let Some(json_string) = storage.get_item("search_result")? {
        
        storage.remove_item("search_result")?;
        
        let result: SearchResult = serde_json::from_str(&json_string)
            .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))?;
        
        return Ok(Some(result));
    }
    
    Ok(None)
}