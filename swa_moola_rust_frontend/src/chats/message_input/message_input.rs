use leptos::*;
use leptos::prelude::*; 
use leptos_router::hooks::use_navigate; 
use serde::{Serialize};
use js_sys::Array;
use leptos::serde_json; 
use uuid::Uuid;
use reqwest::Method;
use web_sys::MediaRecorder;
use crate::interceptor::authenticated_fetch;
use leptos_router::NavigateOptions;
use crate::chats::message_input::send_message::send_message;
use crate::chats::message_input::voice_message_input::{reset_voice_recorder, start_voice_recorder, stop_voice_recorder, RecordState};
use crate::auth::models::AuthenticatedUser; 
use crate::chats::models::{UserPublicKeys, ChatTarget};


#[component]
pub fn MessageInput(
    target_id: Uuid,
    is_recipient: bool,
    recipients: Vec<UserPublicKeys>,
    s_x25519:[u8; 32],
    s_mlkem:[u8; 1184],
    #[prop(into)] on_success: Callback<()>
) -> impl IntoView { 
    let navigate = use_navigate(); 
    let user = window() 
        .local_storage() 
        .ok()
        .flatten()
        .and_then(|s| s.get_item("auth_user").ok().flatten()) 
        .and_then(|json| serde_json::from_str::<AuthenticatedUser>(&json).ok()); 

    let (content, set_content) = signal(String::new());
    let (_error_msg, set_error_msg) = signal(Option::<String>::None);
    
    let user_uuid = user.as_ref().map(|u| u.uuid.to_string()).unwrap_or_default();
    let sender_uuid = user.as_ref().map(|u| u.uuid).unwrap_or_else(Uuid::nil);
    let sender_name =user
        .as_ref()
        .and_then(|u| u.name.as_deref())
        .unwrap_or("Unknown User")
        .to_string();
    let (files, set_files) = signal(Vec::<leptos::web_sys::File>::new());

    let state = RwSignal::new(RecordState::Idle);

    let media_recorder: StoredValue<Option<MediaRecorder>> = StoredValue::new(None);
    let audio_chunks: StoredValue<Array> = StoredValue::new(Array::new());

    let start_recording = move |_| {
        start_voice_recorder(state, media_recorder, audio_chunks, set_files);
    };

    let stop_recording = move |_| stop_voice_recorder(media_recorder);
    let reset_recorder = move |_| reset_voice_recorder(state, set_files);

    let on_submit = move |ev: ev::SubmitEvent| {
        ev.prevent_default();
        let recipients_clone = recipients.clone();
        let navigate_clone = navigate.clone();
        let user_uuid_clone =  user_uuid.clone();
        let current_sender_name = sender_name.clone(); 

        let target= if is_recipient{
            ChatTarget::NewChat { recipient_id : target_id }
        }else{
            ChatTarget::ExistingChat { conv_id: target_id }
        };
        
        leptos::task::spawn_local(async move {
            match target {
                ChatTarget::NewChat { recipient_id} => {
                    let recipient_str = recipient_id.to_string();

                    let new_room_result = create_new_conv_id(
                    recipient_str, 
                    navigate_clone.clone(), 
                    user_uuid_clone.clone()
                    ).await;
                
                    match new_room_result {
                        Some(new_conv_id) => {
                            send_message(
                                user_uuid_clone.clone(),
                                current_sender_name,
                                sender_uuid,
                                new_conv_id,
                                content,
                                set_content,
                                is_recipient,
                                files,      
                                set_files,
                                set_error_msg,
                                on_success,
                                navigate_clone,
                                s_x25519,
                                s_mlkem,
                                recipients_clone
                            );
                        }
                        None => {
                            set_error_msg.set(Some(format!("Failed to initialize chat",)));
                        }
                    }
                
                },
                ChatTarget::ExistingChat { conv_id } => {
                    send_message(
                        user_uuid_clone.clone(),
                        current_sender_name,
                        sender_uuid,
                        conv_id,
                        content,
                        set_content,
                        is_recipient,
                        files,      
                        set_files,
                        set_error_msg,
                        on_success,
                        navigate_clone,
                        s_x25519,
                        s_mlkem,
                        recipients_clone
                    );   
                }
            }
        });
        state.set(RecordState::Idle);
    }; 

    view! {            
        <Show when=move || !files.get().is_empty()>
            <div class="flex flex-wrap gap-2 mb-3 p-2 bg-gray-50 rounded-xl border border-gray-100">
                {move || files.get().into_iter().enumerate().map(|(idx, file)| {
                    let file_name = file.name();
                    view! {
                        <div class="flex items-center gap-1.5 bg-white border border-gray-200 pl-3 pr-1.5 py-1 rounded-lg text-xs font-medium text-gray-700 shadow-sm">
                            <span>{file_name}</span>
                            <button 
                                type="button"
                                class="p-1 text-gray-400 hover:text-red-500 rounded-md hover:bg-gray-100 transition-colors"
                                on:click=move |_| {
                                    set_files.update(|list| { list.remove(idx); });
                                }
                            >
                                <svg width="1.5rem" height="1.5rem" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
                                    <path fill="#000000" d="M352 192V95.936a32 32 0 0 1 32-32h256a32 32 0 0 1 32 32V192h256a32 32 0 1 1 0 64H96a32 32 0 0 1 0-64h256zm64 0h192v-64H416v64zM192 960a32 32 0 0 1-32-32V256h704v672a32 32 0 0 1-32 32H192zm224-192a32 32 0 0 0 32-32V416a32 32 0 0 0-64 0v320a32 32 0 0 0 32 32zm192 0a32 32 0 0 0 32-32V416a32 32 0 0 0-64 0v320a32 32 0 0 0 32 32z"/>
                                </svg>
                            </button>
                        </div>
                    }
                }).collect_view()}
            </div>
        </Show>
              
        <div class="p-4 bg-white border-t border-gray-200 shadow-sm w-full relative fixed bottom-0">
            <form class="flex items-center gap-3  mx-auto " on:submit=on_submit>
                <div class="flex items-center gap-2 w-full">
                    {move || match state.get() {
                        RecordState::Idle => view! {
                            <div class="flex items-center">
                                <input 
                                    type="file" 
                                    id="file-upload"
                                    multiple=true
                                    accept="image/*,video/*,audio/*,application/pdf,application/zip,application/x-rar-compressed"
                                    class="hidden"
                                    on:change=move |ev| {
                                        let target = event_target::<leptos::web_sys::HtmlInputElement>(&ev);
                                        if let Some(file_list) = target.files() {
                                            let mut uploaded = Vec::new();
                                            for i in 0..file_list.length() {
                                                if let Some(f) = file_list.get(i) {
                                                    log::info!("the file : {:?}", &f);
                                                    uploaded.push(f);
                                                }
                                            }
                                            set_files.set(uploaded);
                                        }
                                    }
                                />
                                <label 
                                    for="file-upload" 
                                    class="p-3 bg-gray-100 hover:bg-gray-200 text-gray-600 rounded-xl cursor-pointer transition-all active:scale-95 focus-within:ring-2 focus-within:ring-blue-500 shadow-sm"
                                >
                                    <svg width="1.5rem" height="1.5rem" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
                                        <path fill-rule="evenodd" clip-rule="evenodd" d="M9 0H2V16H14V5L9 0ZM7 6V8H5V10H7V12H9V10H11V8H9V6H7Z" fill="#000000"/>
                                    </svg>
                                </label>
                            </div>
                            <button 
                                type="button"
                                on:click=start_recording
                                class="flex items-center gap-1.5 px-3 py-3 bg-gray-100 hover:bg-blue-50 text-white rounded-lg text-xs font-medium transition-colors"
                            >
                                <svg class="animate-pulse" fill="#000000" height="1.5rem" width="1.5rem" version="1.1" id="Layer_1" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" 
                                    viewBox="0 0 512 512" enable-background="new 0 0 512 512" xml:space="preserve">
                                    <path d="M155.7,320.3c2.8,23.3,24.9,42.4,49,42.4h87.8c24.1,0,46.2-19.1,49-42.4l11.7-96.6c2.8-23.3,2.8-61.4,0-84.8l-11.7-96.6
                                        C338.6,19,316.5,0,292.4,0h-87.8c-24.1,0-46.2,19.1-49,42.4L144,138.9c-2.8,23.3-2.8,61.5,0,84.8L155.7,320.3z M419.1,170.7h-42.6
                                        c0.4,19.5-0.3,40.2-2.2,55.6l-11.7,96.6c-4.1,34.3-34.9,61.1-70.2,61.1h-87.8c-35.2,0-66-26.9-70.2-61.1l-11.7-96.6
                                        c-1.9-15.4-2.6-36.1-2.2-55.6H77.9c-0.4,21.3,0.4,43.6,2.5,60.7L92.1,328c6.7,55.3,56.1,98.6,112.5,98.6h22.6v42.7h-64V512h170.7
                                        v-42.7h-64v-42.7h22.6c56.5,0,105.9-43.4,112.5-98.7l11.7-96.7C418.7,214.2,419.5,191.9,419.1,170.7z"/>
                                </svg>
                            </button>
                            <div class="relative flex-1">
                                <input 
                                    type="text" 
                                    placeholder="Type a message..."
                                    prop:value=content 
                                    on:input=move |ev| set_content.set(event_target_value(&ev)) 
                                    class="w-full pl-4 pr-12 py-3 bg-gray-50 border border-gray-200 text-gray-800 placeholder-gray-400 rounded-2xl focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent focus:bg-white shadow-inner transition-all text-sm"
                                />
                            </div>
                            
                            <button 
                                type="submit" 
                                class="p-3 bg-gray-100 hover:bg-blue-50 text-white rounded-xl shadow-md hover:shadow-lg active:scale-95 transition-all focus:outline-none"
                            >
                                <svg fill="#000000" width="1.5rem" height="1.5rem" viewBox="0 0 32 32" style="fill-rule:evenodd;clip-rule:evenodd;stroke-linejoin:round;stroke-miterlimit:2;" version="1.1" xml:space="preserve" xmlns="http://www.w3.org/2000/svg" xmlns:serif="http://www.serif.com/" xmlns:xlink="http://www.w3.org/1999/xlink">
                                    <path d="M11.499,19.173l5.801,-5.849c0.389,-0.392 1.022,-0.394 1.414,-0.006c0.392,0.389 0.395,1.022 0.006,1.414l-5.798,5.847l5.306,8.002c0.207,0.313 0.572,0.483 0.945,0.441c0.373,-0.042 0.691,-0.289 0.824,-0.64l9.024,-23.904c0.138,-0.366 0.05,-0.78 -0.226,-1.058c-0.276,-0.278 -0.689,-0.369 -1.057,-0.233l-24.004,8.892c-0.353,0.13 -0.602,0.448 -0.646,0.821c-0.044,0.373 0.125,0.74 0.438,0.948l7.973,5.325Z"/>
                                    <g id="Icon"/>
                                </svg>
                            </button>
                        }.into_any(),         
                    
                        RecordState::Recording => view! {
                            <button 
                                on:click=stop_recording
                                class="flex justify-end gap-1.5 px-3 py-2 bg-red-300 hover:bg-red-700 text-white rounded-lg text-xs font-medium transition-colors"
                            >
                                <span class="w-2 h-2 rounded-full bg-white animate-ping" />
                                "Stop"
                            </button>
                        }.into_any(),

                        RecordState::Finished(url) => view! {
                            <div class="flex flex-row gap-2 w-full">
                                
                                <audio controls src=url class="w-full min-w-400 h-8" />
                                
                                <div class="flex gap-2 justify-end">
                                    <button 
                                        on:click=reset_recorder
                                        class="px-2.5 py-1 text-gray-500 hover:bg-red-300 text-xs font-medium transition-colors"
                                    >
                                        <svg width="1.5rem" height="1.5rem" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
                                            <path fill="#000000" d="M352 192V95.936a32 32 0 0 1 32-32h256a32 32 0 0 1 32 32V192h256a32 32 0 1 1 0 64H96a32 32 0 0 1 0-64h256zm64 0h192v-64H416v64zM192 960a32 32 0 0 1-32-32V256h704v672a32 32 0 0 1-32 32H192zm224-192a32 32 0 0 0 32-32V416a32 32 0 0 0-64 0v320a32 32 0 0 0 32 32zm192 0a32 32 0 0 0 32-32V416a32 32 0 0 0-64 0v320a32 32 0 0 0 32 32z"/>
                                        </svg>
                                    </button>
                                    
                                </div>
                                <button 
                                    type="submit" 
                                    class="p-3 hover:bg-blue-50 text-white rounded-xl shadow-md hover:shadow-lg active:scale-95 transition-all focus:outline-none"
                                >
                                    <svg fill="#000000" width="1.5rem" height="1.5rem" viewBox="0 0 32 32" style="fill-rule:evenodd;clip-rule:evenodd;stroke-linejoin:round;stroke-miterlimit:2;" version="1.1" xml:space="preserve" xmlns="http://www.w3.org/2000/svg" xmlns:serif="http://www.serif.com/" xmlns:xlink="http://www.w3.org/1999/xlink">
                                        <path d="M11.499,19.173l5.801,-5.849c0.389,-0.392 1.022,-0.394 1.414,-0.006c0.392,0.389 0.395,1.022 0.006,1.414l-5.798,5.847l5.306,8.002c0.207,0.313 0.572,0.483 0.945,0.441c0.373,-0.042 0.691,-0.289 0.824,-0.64l9.024,-23.904c0.138,-0.366 0.05,-0.78 -0.226,-1.058c-0.276,-0.278 -0.689,-0.369 -1.057,-0.233l-24.004,8.892c-0.353,0.13 -0.602,0.448 -0.646,0.821c-0.044,0.373 0.125,0.74 0.438,0.948l7.973,5.325Z"/>
                                        <g id="Icon"/>
                                    </svg>
                                </button>
                            </div>
                        }.into_any(),
                    }}
                </div>
            </form>
        </div>
    }
}


#[derive(Serialize)]
pub struct CPayload { 
    pub recipient_id : Uuid, 
}


async fn create_new_conv_id(
    recipient_id: String,
    navigate: impl Fn(&str, NavigateOptions) + Clone + 'static,
    user_uuid: String,
 )-> Option<Uuid>{
 
        let navigate = navigate.clone();
 
        let url = format!("http://localhost:8000/api/m/nch/{}", user_uuid);
        let parsed_uuid = Uuid::parse_str(&recipient_id).unwrap_or_else(|_| Uuid::nil());
        let payload = CPayload { recipient_id: parsed_uuid }; 

        let res = authenticated_fetch(Method::POST, &url, navigate, Some(payload)).await; 
        
        match res { 
            Ok(resp) => {
                if resp.status().is_success() {
                    let text = resp.text().await.unwrap_or_default();

                    let new_conv_id = match serde_json::from_str::<Uuid>(&text) {
                        Ok(parsed) => parsed,
                        Err(_e) => {
                            return None;
                        }
                    };
                    Some(new_conv_id)
                } else {
                    None                        
                }
            }, 
            Err(_) => {
                None
            }, 
        }
     
}








