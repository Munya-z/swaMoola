use leptos::prelude::*; 
use crate::interceptor::authenticated_multipart_fetch; 
use leptos_router::hooks::use_navigate; 
use reqwest::multipart::{Form, Part};
use wasm_bindgen_futures::JsFuture;
use leptos::serde_json; 
use reqwest::Method;  
use leptos::ev;
use leptos_router::NavigateOptions;
use uuid::Uuid;
use crate::auth::models::AuthenticatedUser; 
use crate::chats::models::{ Message};

pub fn send_message(
    user_uuid: String,
    sender_uuid: Uuid,
    recipient_id: Uuid,
    content: ReadSignal<String>,
    set_content: WriteSignal<String>,
    is_recipient: bool,
    files: ReadSignal<Vec<leptos::web_sys::File>>,     
    set_files: WriteSignal<Vec<leptos::web_sys::File>>, 
    error_msg: WriteSignal<Option<String>>,
    on_success: Callback<()>,
    navigate: impl Fn(&str, NavigateOptions) + Clone + 'static,
){
    error_msg.set(None);

    if content.get().trim().is_empty()  && files.get().is_empty() {
        error_msg.set(Some("Message cannot be empty.".to_string()));
        return;
    }

    let navigate = navigate.clone();
    let text_content = content.get();
    let current_files = files.get();
    
    let url = format!("http://localhost:8000/api/m/sm/{}", user_uuid);

    leptos::task::spawn_local( async move { 

        let mut form = Form::new()
            .text("sender_id", sender_uuid.to_string())
            .text("recipient_id", recipient_id.to_string())
            .text("content", text_content);

        for file in current_files {
            let file_name = file.name();
            
            // Convert web_sys::File into an array buffer promise, then a Rust Result
            let array_buffer_promise = file.array_buffer();
            if let Ok(buffer_value) = JsFuture::from(array_buffer_promise).await {
                let js_array = js_sys::Uint8Array::new(&buffer_value);
                let bytes = js_array.to_vec();
                
                let base_part = Part::bytes(bytes);
                let completed_part = base_part.file_name(file_name);
                form = form.part("files", completed_part);                
            }
        }

        let res: Result<reqwest::Response, reqwest::Error> = 
            authenticated_multipart_fetch(Method::POST, &url, navigate.clone(), Some(form)).await; 
            
        match res { 
            Ok(resp) => {
                if resp.status().is_success() {
                    set_content.set(String::new());
                    set_files.set(Vec::new()); // 6. Clear files upon successful transmission

                    if is_recipient {
                        let text = resp.text().await.unwrap_or_default();
                        
                        if let Ok(data) = serde_json::from_str::<Message>(&text) {
                            let next_url = format!("/chats/{}", data.conv_id);
                            request_animation_frame(move || {
                                navigate(&next_url, Default::default());
                            });
                        } else {
                            on_success.run(());
                        }
                    }
                    on_success.run(());
                } else {
                    error_msg.set(Some(format!("Server returned: {}", resp.status())));
                }
            }, 
            Err(e) => {
                error_msg.set(Some(format!("Network error: {}", e)));
            }, 
        }

    }) 
}

#[component]
pub fn MessageInput(
    recipient_id: Uuid,
    is_recipient: bool,
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

    let (files, set_files) = signal(Vec::<leptos::web_sys::File>::new());

    let on_submit = move |ev: ev::SubmitEvent| {
        ev.prevent_default();
        send_message(
            user_uuid.clone(),
            sender_uuid,
            recipient_id,
            content,
            set_content,
            is_recipient,
            files,      
            set_files,
            set_error_msg,
            on_success,
            navigate.clone(),
        );
    }; 

    view! {


        <div class="p-4 bg-white border-t border-gray-200 shadow-sm w-full relative fixed bottom-0">
            // 1. File Preview List
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
                                        // Remove specific file by index
                                        set_files.update(|list| { list.remove(idx); });
                                    }
                                >
                                    "✕"
                                </button>
                            </div>
                        }
                    }).collect_view()}
                </div>
            </Show>

            // Your existing <form> stays here...
        </div>

        <div class="p-4 bg-white border-t border-gray-200 shadow-sm w-full relative fixed bottom-0">
            <form class="flex items-center gap-3  mx-auto " on:submit=on_submit>
                
                 <div class="flex items-center">
                    <input 
                        type="file" 
                        id="file-upload"
                        multiple=true
                        class="hidden"
                        on:change=move |ev| {
                            let target = event_target::<leptos::web_sys::HtmlInputElement>(&ev);
                            if let Some(file_list) = target.files() {
                                let mut uploaded = Vec::new();
                                for i in 0..file_list.length() {
                                    if let Some(f) = file_list.get(i) {
                                        uploaded.push(f);
                                    }
                                }
                                leptos::logging::log!("Selected {} files", uploaded.len());
                                set_files.set(uploaded);
                            }
                        }
                    />
                    <label 
                        for="file-upload" 
                        class="p-3 bg-gray-100 hover:bg-gray-200 text-gray-600 rounded-xl cursor-pointer transition-all active:scale-95 focus-within:ring-2 focus-within:ring-blue-500 shadow-sm"
                    >
                        // Visual Anchor: Paperclip SVG
                        <svg xmlns="http://w3.org" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-5 h-5">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M18.364 18.707m-1.414 1.414a5 5 0 11-7.071-7.071l10.607-10.607a3.5 3.5 0 114.95 4.95L10.607 18.007a2 2 0 11-2.828-2.828l10.607-10.607m-4.243 4.243L8.485 14.586" />
                        </svg>
                    </label>
                </div>

                // Styled Message Input Field
                <div class="relative flex-1">
                    <input 
                        type="text" 
                        placeholder="Type a message..."
                        prop:value=content 
                        on:input=move |ev| set_content.set(event_target_value(&ev)) 
                        class="w-full pl-4 pr-12 py-3 bg-gray-50 border border-gray-200 text-gray-800 placeholder-gray-400 rounded-2xl focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent focus:bg-white shadow-inner transition-all text-sm"
                    />
                </div>

                // Send Message Button
                <button 
                    type="submit" 
                    class="p-3 bg-blue-600 hover:bg-blue-700 text-white rounded-xl shadow-md hover:shadow-lg active:scale-95 transition-all focus:outline-none"
                >
                    // Visual Anchor: Send Paper Plane SVG
                    <svg xmlns="w3.org" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-5 h-5 transform rotate-45 -translate-x-0.5 translate-y-0.5">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M6 12L3.269 3.126A59.768 59.768 0 0121.485 12 59.77 59.77 0 013.27 20.876L5.999 12zm0 0h7.5" />
                    </svg>
                </button>

            </form>
        </div>
    }


}
 