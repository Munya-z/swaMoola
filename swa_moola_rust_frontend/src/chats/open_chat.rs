use leptos::prelude::*;
use leptos_router::components::*;
use uuid::Uuid;
use reqwest::Method;  
use chrono::{DateTime, Utc, Datelike};
use leptos::{serde_json, html::Ul};
use leptos_router::{NavigateOptions,hooks::{use_navigate, use_params, use_location}};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use crate::chats::models::{SecretInnerPayload};
use crate::chats::make_new_group_chat::MakeGroupChat;

use crate::auth::create_ecryption_keys::load_private_keys_locally;
use crate::auth::models::AuthenticatedUser; 
use crate::chats::message_input::message_input::MessageInput;
use crate::interceptor::authenticated_fetch; 
use crate::chats::models::{ChatParams, ConversationPayload, SearchResult,ChatPayload, Attachment, InboundMessagePayload};
use crate::chats::chats_list::ChatsList;
use crate::chats::ws_hooks::use_websocket_listener;
use crate::chats::message_bubble::message_viewer::MessageViewer;
use crate::chats::message_decryption::decrypt_message_payload::decrypt_message_with_fallback;
use crate::chats::models::{UserPublicKeys, ConversationPayloadWithStringKeys};

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
                        let converted_keys: Vec<UserPublicKeys> = data.recipient_keys
                        .iter()
                        .map(|key_string| {
                            UserPublicKeys::new(
                                Some(&key_string.x25519), 
                                Some(&key_string.mlkem)
                            )
                        })
                        .collect();

                        return Some(ConversationPayload{
                            conv_id: Uuid::nil(),
                            is_group: false,
                            created_at: chrono::Utc::now(),
                            name: String::new(),
                            display_name: Some(data.name),
                            recipient_id: Some(data.target_user_id), 
                            recipient_keys: converted_keys,
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

                        if let Ok(data) = serde_json::from_str::<ConversationPayloadWithStringKeys>(&text) {
                            
                            // 2. Convert string keys into your target binary byte-key types
                            let converted_keys: Vec<UserPublicKeys> = data.recipient_keys
                                .iter()
                                .map(|key_string| {
                                    UserPublicKeys::new(
                                        Some(&key_string.x25519), 
                                        Some(&key_string.mlkem)
                                    )
                                })
                                .collect();

                            let final_payload = ConversationPayload {
                                conv_id: data.conv_id,
                                is_group: data.is_group,
                                created_at: data.created_at,
                                name: data.name,
                                display_name: data.display_name,
                                recipient_id: data.recipient_id,
                                recipient_keys: converted_keys, // Swap in the binary keys here
                                last_msg_id: data.last_msg_id,
                            };

                            return Some(final_payload) 
                        }
                        None
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

    let (resolved_conv_id, set_resolved_conv_id) = signal::<Option<Uuid>>(None);

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

    let user_uuid_clone_group = user_uuid.clone();
    let conv_id_for_group = move || resolved_conv_id.get();

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

            _ =>{

                let user_uuid_for_this_render = user_uuid_clone_group.clone();
                let parsed_user_uuid = Uuid::parse_str(&user_uuid_for_this_render)
                    .unwrap_or_else(|_| Uuid::nil());

                view! {
                <div class="flex-1 flex flex-col h-full relative bg-[url(/images/chat_bg.png)] bg-center bg-cover">
                <div class="absolute inset-0 bg-black/35 pointer-events-none z-0"></div>
                    <div 
                    id="my-huge-popover" 
                    popover="auto" 
                    class="w-[80vw] h-[80vh] bg-white rounded-xl shadow-2xl p-6 m-auto border-0 backdrop:bg-black/50 backdrop:backdrop-blur-sm">
                    
                        <div class="w-full h-full flex flex-col p-6">
                            <div class="flex items-center justify-between border-b pb-4">
                                <h3 class="text-xl font-bold text-gray-900">"Create New Group"</h3>
                                <button 
                                popovertarget="my-huge-popover" 
                                popovertargetaction="hide" 
                                class="px-4 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg font-medium">"Close"</button>
                            </div>
                        
                            <div class="flex-1 overflow-y-auto py-4 text-gray-600">
                            {move || match conv_id_for_group(){
                                    Some(actual_conv_id) => {
                                        log::info!("the conv_id {:?}", &actual_conv_id);
                                        log::info!("the user_uuid {:?}", &parsed_user_uuid);

                                        view! {
                                            <MakeGroupChat 
                                                conv_id=actual_conv_id 
                                                user_uuid=parsed_user_uuid
                                            />
                                        }.into_any()
                                    },
                                    None => view! { 
                                        <div>"Loading conversation details..."</div> 
                                    }.into_any()
                                }}

                            </div>
                            
                            <div class="border-t pt-4 flex justify-end gap-3">
                                "you can add more participants later"
                            </div>
                        </div>
                    </div>
                        
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

                    <div class="flex items-center justify-between">
                        <button popovertarget="my-huge-popover"  class="text-sm  font-medium text-[#00a884] hover:underline px-3 py-1.5 rounded" >
                            <svg fill="#000000" width="1.5rem" height="1.5rem" viewBox="0 0 56 56" xmlns="http://www.w3.org/2000/svg">
                                <path d="M 33.7169 50.6051 C 45.9141 50.6051 56.0000 40.4968 56.0000 28.2994 C 56.0000 16.1245 45.8920 5.9937 33.6944 5.9937 C 22.4180 5.9937 12.9611 14.6419 11.5909 25.5365 C 12.1749 25.5365 12.7365 25.5814 13.2981 25.6712 C 13.9944 25.7611 14.6908 25.9183 15.3646 26.1205 C 16.4204 16.9332 24.1926 9.8124 33.6944 9.8124 C 43.9598 9.8124 52.1812 18.0563 52.2037 28.2994 C 52.2037 33.0840 50.4294 37.3969 47.5090 40.6765 C 44.1171 37.8461 39.0406 35.9593 33.6944 35.9593 C 31.1785 35.9593 28.3258 36.4984 25.6751 37.4418 C 25.8324 38.2954 25.9222 39.1715 25.9222 40.0475 C 25.9222 42.9902 25.0012 45.7531 23.4513 48.0668 C 26.5287 49.6616 30.0329 50.6051 33.7169 50.6051 Z M 33.6944 32.0956 C 38.0073 32.0956 41.2644 28.3668 41.2644 23.6720 C 41.2644 19.2469 37.9399 15.4057 33.6944 15.4057 C 29.4714 15.4057 26.1244 19.2469 26.1244 23.6720 C 26.1244 28.3668 29.4040 32.0956 33.6944 32.0956 Z M 11.4112 51.4587 C 17.6110 51.4587 22.8224 46.2922 22.8224 40.0475 C 22.8224 33.8028 17.6783 28.6363 11.4112 28.6363 C 5.1665 28.6363 0 33.8028 0 40.0475 C 0 46.3372 5.1665 51.4587 11.4112 51.4587 Z M 11.4336 47.4603 C 10.6474 47.4603 9.9511 46.9212 9.9511 46.0676 L 9.9511 41.4178 L 5.6607 41.4178 C 4.8969 41.4178 4.2679 40.7888 4.2679 40.0475 C 4.2679 39.2838 4.8969 38.6548 5.6607 38.6548 L 9.9511 38.6548 L 9.9511 34.0050 C 9.9511 33.1739 10.6474 32.6347 11.4336 32.6347 C 12.1974 32.6347 12.8937 33.1739 12.8937 34.0050 L 12.8937 38.6548 L 17.1841 38.6548 C 17.9479 38.6548 18.5544 39.2838 18.5544 40.0475 C 18.5544 40.7888 17.9479 41.4178 17.1841 41.4178 L 12.8937 41.4178 L 12.8937 46.0676 C 12.8937 46.9212 12.1974 47.4603 11.4336 47.4603 Z"/>
                            </svg>
                        </button>

                        <A href="/chats" attr:class="text-sm sm:hidden font-medium text-[#00a884] hover:underline px-3 py-1.5 rounded" >
                            <svg fill="#000000" version="1.1" id="Capa_1" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" 
                                    width="1.5rem" height="1.5rem" viewBox="0 0 869.959 869.958"
                                    xml:space="preserve">
                                <g>
                                    <path d="M146.838,484.584c10.271,10.395,23.804,15.6,37.347,15.6c13.329,0,26.667-5.046,36.897-15.155
                                        c20.625-20.379,20.825-53.62,0.445-74.245l-41.688-42.191h423.78c88.963,0,161.34,72.376,161.34,161.339v4.32
                                        c0,43.096-16.782,83.61-47.255,114.084c-20.503,20.502-20.503,53.744,0,74.246c10.251,10.251,23.688,15.377,37.123,15.377
                                        c13.435,0,26.872-5.125,37.123-15.377c50.305-50.306,78.009-117.188,78.009-188.331v-4.32c0-71.142-27.704-138.026-78.009-188.331
                                        c-50.306-50.305-117.189-78.009-188.331-78.009h-424.99l42.25-41.747c20.625-20.379,20.825-53.62,0.445-74.245
                                        c-20.376-20.624-53.618-20.825-74.244-0.445L15.601,277.068c-9.905,9.787-15.517,23.107-15.6,37.03
                                        c-0.084,13.924,5.367,27.31,15.154,37.215L146.838,484.584z"/>
                                </g>
                            </svg>
                        </A>
                    </div>
                </nav>

                // Message Thread Container
                <Suspense fallback=|| view! { <div class="flex-1 flex items-center justify-center text-[#667781] text-sm">"Loading Messages..."</div> }> 
                {move || { 
                    chat_messages.get().map(|data: Option<Vec<SecretInnerPayload>>| { 
                        match data { 
                            Some(msgs) => view! { 
                                <ul node_ref=message_list_ref class="flex-1 overflow-y-auto w-full z-10 px-6 pt-4 pb-24 space-y-2 scrollbar-thin"> 
                                    {msgs.into_iter().map(|msg| {
                                        let is_sender = Some(msg.sender_id) == user_uuid_opt;
                                        let formatted_time = format_chat_time(&msg.timestamp_ms);

                                        let sender_name_str = msg.sender_name;
                                        log::info!("sender name str: {:?}", &sender_name_str);
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
                                                    <p>{sender_name_str}</p>
                                                    <div class="bg-[#d8eef3] overflow-hidden text-[#111b21] pl-0 pr-0 rounded-lg rounded-tr-none max-w-[75%] shadow-sm relative group text-[14.2px] leading-relaxed break-words">
                                                        <MessageViewer msg_content=msg_content msg_attachments=msg_attachments /> 
                                                        <span class="text-[11px] text-[#667781] absolute bottom-1 right-2 select-none whitespace-nowrap flex items-center gap-0.5">
                                                            {formatted_time}
                                                            <svg fill="#000000" width=".5rem" height=".5rem" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                                                                <path d="M2.305,11.235a1,1,0,0,1,1.414.024l3.206,3.319L14.3,7.289A1,1,0,0,1,15.7,8.711l-8.091,8a1,1,0,0,1-.7.289H6.9a1,1,0,0,1-.708-.3L2.281,12.649A1,1,0,0,1,2.305,11.235ZM20.3,7.289l-7.372,7.289-.263-.273a1,1,0,1,0-1.438,1.39l.966,1a1,1,0,0,0,.708.3h.011a1,1,0,0,0,.7-.289l8.091-8A1,1,0,0,0,20.3,7.289Z"/>
                                                            </svg>
                                                        </span>
                                                    </div>  
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { 
                                                <div class="flex justify-start mb-1">
                                                    <p>{sender_name_str}</p>
                                                    <div class="bg-white overflow-hidden text-[#111b21] pl-0 pr-0 rounded-lg rounded-tl-none max-w-[75%] shadow-sm relative text-[14.2px] leading-relaxed break-words">
                                                        <MessageViewer msg_content=msg_content msg_attachments=msg_attachments />
                                                        <span class="text-[11px] text-[#667781] absolute bottom-1 right-2 select-none whitespace-nowrap flex items-center gap-0.5">
                                                            {formatted_time}
                                                            <svg fill="#000000" width=".5rem" height=".5rem" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                                                                <path d="M2.305,11.235a1,1,0,0,1,1.414.024l3.206,3.319L14.3,7.289A1,1,0,0,1,15.7,8.711l-8.091,8a1,1,0,0,1-.7.289H6.9a1,1,0,0,1-.708-.3L2.281,12.649A1,1,0,0,1,2.305,11.235ZM20.3,7.289l-7.372,7.289-.263-.273a1,1,0,1,0-1.438,1.39l.966,1a1,1,0,0,0,.708.3h.011a1,1,0,0,0,.7-.289l8.091-8A1,1,0,0,0,20.3,7.289Z"/>
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

                    view! { 
                        <Show
                            when=move || chat_name_resource.get().flatten().is_some()
                            fallback=move || view! { <p>"Loading cryptographic session keys..."</p> }
                        >
                            {
                                let payload = chat_name_resource.get().flatten().unwrap();
                                
                                let recipients: Vec<UserPublicKeys> = payload.recipient_keys;

                                view! {
                                    <MessageInput 
                                        target_id=route_uuid 
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
        }
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
        
        let result: SearchResult = serde_json::from_str(&json_string)
            .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))?;
        
        return Ok(Some(result));
    }
    
    Ok(None)
}


