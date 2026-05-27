use leptos::prelude::*;
use leptos_router::hooks::use_params;
use leptos_router::hooks::{use_navigate};
use leptos_router::hooks::use_location;
use leptos_router::NavigateOptions;
use leptos::{serde_json, web_sys}; 
use reqwest::Method; 
use crate::auth::models::AuthenticatedUser; 
use crate::chats::message_input::MessageInput;
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

pub fn get_chat_messages(
    navigate: impl Fn(&str, NavigateOptions) + Clone + 'static,
    user_uuid: String,
    current_id_fn: impl Fn() -> String + Send + Sync + 'static, 
    is_recipient_fn: impl Fn() -> bool + Send + Sync + 'static,
    refresh_trigger: Trigger,
    message_trigger_read: ReadSignal<i32>,
) -> LocalResource<Option<Vec<Message>>> {
    LocalResource::new(move || {
        let navigate = navigate.clone();
        let user_uuid = user_uuid.clone();
        let current_id_str = current_id_fn();
        let is_recipient = is_recipient_fn();

        // Track modern Leptos triggers reactively
        refresh_trigger.track();
        message_trigger_read.track();

        async move { 
            if is_recipient || current_id_str == "No ID found" {
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
                        serde_json::from_str::<Vec<Message>>(&text).ok()
                    } else {
                        None                        
                    }
                }, 
                Err(_) => None, 
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
                return None ;
            }

            let route_id = Uuid::parse_str(&current_id_str).unwrap_or_else(|_| Uuid::nil());
            
            // Restored side-effect state update safely inside the async flow
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
    
    // 2. Provide the contexts so child components (like ChatsList) can use .track()
    provide_context(message_trigger_read);
    provide_context(message_trigger_write);
    
    // 4. Start the global background listener task
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
        is_recipient_route,
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
                <div class="flex-1 flex flex-col h-full relative bg-[url(/images/chat_bg.png)] bg-center ">
                <div class="absolute inset-0 bg-black/35 pointer-events-none z-1"></div>
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
                    chat_messages.get().map(|data: Option<Vec<Message>>| { 
                        match data { 
                            Some(msgs) => view! { 
                                <ul node_ref=message_list_ref class="flex-1 overflow-y-auto z-10 px-6 pt-4 pb-24 space-y-2 scrollbar-thin"> 
                                    {msgs.into_iter().map(|msg| {
                                        let is_sender = msg.sender_id == user_uuid_opt;
                                        let formatted_time = format_chat_time(&msg.created_at);

                                        let msg_content = msg.content.clone();
                                        let msg_attachments = msg.attachments.clone();
                                        

                                        if is_sender {
                                            view! { 
                                                <div class="flex justify-end mb-1">
                                                    <div class="bg-[#d9fdd3] text-[#111b21] pl-3 pr-2 py-1.5 rounded-lg rounded-tr-none max-w-[65%] shadow-sm relative group text-[14.2px] leading-relaxed break-words">
                                                        {

                                                            let msg_content_check = msg_content.clone();
                                                            let msg_content_display = msg_content.clone();
                                                            view! {
                                                                <Show when=move || msg_content_check.as_ref().is_some()>
                                                                {
                                                                    let content_str = msg_content_display.clone();
                                                                    view! {
                                                                        <p class="inline-block p-4 mr-12"> 
                                                                            {move || content_str.clone()} 
                                                                        </p>
                                                                    }
                                                                }
                                                                </Show>
                                                            }
                                                        }
                                                        
                                                        {
                                                            let msg_attachments_check = msg_attachments.clone();
                                                            let msg_attachments_display = msg_attachments.clone();
                                                            view! {
                                                                <Show when=move || !msg_attachments_check.is_empty()>
                                                                    <div class="mt-2 space-y-1 ">
                                                                        {msg_attachments_display.clone().into_iter().map(|file| {
                                                                            let media_url = format!("http://localhost:8000/api/m/attachments/{}", file.attachment_id);

                                                                            let file_type_lower = file.file_type.to_lowercase();
                                                                            let file_name_lower = file.file_name.to_lowercase();
                                                                            
                                                                            let is_image = file.file_type.starts_with("image/")
                                                                                || file_type_lower.contains("jpg")
                                                                                || file_type_lower.contains("jpeg")
                                                                                || file_type_lower.contains("png")
                                                                                || file_type_lower.contains("webp")
                                                                                || file_name_lower.ends_with(".jpg")
                                                                                || file_name_lower.ends_with(".jpeg")
                                                                                || file_name_lower.ends_with(".png")
                                                                                || file_name_lower.ends_with(".webp");

                                                                            let url_for_fallback = media_url.clone();
                                                                            let url_for_img = media_url.clone();
                                                                            let name_for_fallback = file.file_name.clone();
                                                                            let name_for_img = file.file_name.clone();
                                                                            
                                                                            view! {
                                                                                <div class="flex flex-col gap-2 shadow-sm">
                                                                                    <Show 
                                                                                        when=move || is_image
                                                                                        fallback=move || {
                                                                                            let url = url_for_fallback.clone();
                                                                                            let name = name_for_fallback.clone();
                                                                                            view! {
                                                                                                <div class="flex items-center gap-2 text-xs">
                                                                                                    <svg class="w-4 h-4 text-blue-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                                                                        <path d="M18.364 18.707m-1.414 1.414a5 5 0 11-7.071-7.071l10.607-10.607a3.5 3.5 0 114.95 4.95L10.607 18.007a2 2 0 11-2.828-2.828l10.607-10.607m-4.243 4.243L8.485 14.586" />
                                                                                                    </svg>
                                                                                                    <a href=url target="_blank" class="text-blue-600 hover:underline font-medium text-xs">
                                                                                                        {name}
                                                                                                    </a>
                                                                                                </div>
                                                                                            }
                                                                                        }
                                                                                    >
                                                                                        <div class="max-w-xs overflow-hidden ">
                                                                                            <SecureImage 
                                                                                                media_url=url_for_img.clone() 
                                                                                                alt_text=name_for_img.clone() 
                                                                                                file_type=file.file_type.clone()
                                                                                            />
                                                                                        </div>
                                                                                    </Show>
                                                                                </div>
                                                                            }
                                                                        }).collect_view()}
                                                                    </div>
                                                                </Show>
                                                            }
                                                        }
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
                                                        
                                                        {

                                                            let msg_content_check = msg_content.clone();
                                                            let msg_content_display = msg_content.clone();
                                                            view!{
                                                                <Show when=move || msg_content_check.as_ref().is_some()>
                                                                {
                                                                    let content_str = msg_content_display.clone();
                                                                    view! {
                                                                        <p class="inline-block p-4 mr-12"> 
                                                                            {move || content_str.clone()} 
                                                                        </p>
                                                                    }
                                                                }
                                                                </Show>
                                                            }
                                                        }
                                                        
                                                        {
                                                            let msg_attachments_check = msg_attachments.clone();
                                                            let msg_attachments_display = msg_attachments.clone();
                                                            view! {
                                                                <Show when=move || !msg_attachments_check.is_empty()>
                                                                    <div class="mt-2 space-y-1 ">
                                                                        {msg_attachments_display.clone().into_iter().map(|file| {
                                                                            let media_url = format!("http://localhost:8000/api/m/attachments/{}", file.attachment_id);
                                                                            let file_type_lower = file.file_type.to_lowercase();
                                                                            let file_name_lower = file.file_name.to_lowercase();

                                                                            let is_image = file.file_type.starts_with("image/")
                                                                                || file_type_lower.contains("jpg")
                                                                                || file_type_lower.contains("jpeg")
                                                                                || file_type_lower.contains("png")
                                                                                || file_type_lower.contains("webp")
                                                                                || file_name_lower.ends_with(".jpg")
                                                                                || file_name_lower.ends_with(".jpeg")
                                                                                || file_name_lower.ends_with(".png")
                                                                                || file_name_lower.ends_with(".webp");

                                                                            let url_for_fallback = media_url.clone();
                                                                            let url_for_img = media_url.clone();
                                                                            let name_for_fallback = file.file_name.clone();
                                                                            let name_for_img = file.file_name.clone();

                                                                            view! {
                                                                                <div class="flex flex-col gap-2 shadow-sm">
                                                                                    <Show 
                                                                                        when=move || is_image
                                                                                        fallback=move || {
                                                                                            let url = url_for_fallback.clone();
                                                                                            let name = name_for_fallback.clone();
                                                                                            view! {
                                                                                                <div class="flex items-center gap-2 text-xs">
                                                                                                    <svg class="w-4 h-4 text-blue-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                                                                        <path d="M18.364 18.707m-1.414 1.414a5 5 0 11-7.071-7.071l10.607-10.607a3.5 3.5 0 114.95 4.95L10.607 18.007a2 2 0 11-2.828-2.828l10.607-10.607m-4.243 4.243L8.485 14.586" />
                                                                                                    </svg>
                                                                                                    <a href=url target="_blank" class="text-blue-600 hover:underline font-medium text-xs">
                                                                                                        {name}
                                                                                                    </a>
                                                                                                </div>
                                                                                            }
                                                                                        }
                                                                                    >
                                                                                        <div class="max-w-xs overflow-hidden ">
                                                                                            <SecureImage 
                                                                                                media_url=url_for_img.clone() 
                                                                                                alt_text=name_for_img.clone() 
                                                                                                file_type=file.file_type.clone()
                                                                                            />
                                                                                        </div>
                                                                                    </Show>
                                                                                </div>
                                                                            }
                                                                        }).collect_view()}
                                                                    </div>
                                                                </Show>
                                                            }
                                                        }
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
                            chat_name_resource.get()
                                .flatten()
                                .and_then(|payload| payload.recipient_id)
                                .unwrap_or_else(|| Uuid::nil())
                        };

                    view! { 
                        <MessageInput 
                            recipient_id=derived_recipient_id 
                            is_recipient=is_recipient_route()
                            on_success=Callback::new(move |_| refresh_trigger.notify())
                        /> 
                    }
                }}

            </div>
            }.into_any()
        }}
    </div>
    }
}

use base64::{Engine as _, engine::general_purpose::STANDARD};

#[component]
fn SecureImage(media_url: String, alt_text: String, file_type: String) -> impl IntoView {

    let navigate = use_navigate();

    let image_src_resource = LocalResource::new(move || {
        let url = media_url.clone();
        let mime_type = file_type.clone();
        let nav = navigate.clone();


        async move {

            let response = authenticated_fetch::<_, String>(
                Method::GET, 
                &url, 
                nav, 
                None
            ).await.ok()?;

            if !response.status().is_success() {
                return None;
            }
            
            // 3. Get the data directly as a byte chunk vector (Vec<u8>) via reqwest
            let bytes = response.bytes().await.ok()?;
            let rust_bytes = bytes.to_vec();

            // 4. Base64 conversion pipeline
            let encoded_bytes = STANDARD.encode(&rust_bytes);
            let mime = if mime_type.is_empty() { "image/jpeg".to_string() } else { mime_type };
            let base64_src = format!("data:{};base64,{}", mime, encoded_bytes);
            
            Some(base64_src)
        }
    });

    view! {
        <Transition fallback=move || view! { <div class="text-xs text-gray-400"> "Loading..." </div> }>
            {move || image_src_resource.get().map(|maybe_src| {
                if let Some(src) = maybe_src {
                    view! {
                        <img src=src alt=alt_text.clone() class="w-full h-auto rounded object-cover" />
                    }.into_any()
                } else {
                    view! {
                         <div class="text-xs text-red-500"> "Failed to display image" </div> 
                    }.into_any()
                }
            })}
        </Transition>
    }
}