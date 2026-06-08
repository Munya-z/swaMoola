use leptos::prelude::*; 
use leptos_router::hooks::use_navigate; 
use leptos::serde_json; 
use leptos::ev;
use uuid::Uuid;
use crate::chats::message_input::send_message::send_message;
use crate::auth::models::AuthenticatedUser; 

#[component]
pub fn MessageInput(
    recipient_id: Uuid,
    is_recipient: bool,
    r_x25519:[u8; 32],
    r_mlkem:[u8; 1184],
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
            s_x25519,
            s_mlkem,
            r_x25519,
            r_mlkem,
        );
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
                                "✕"
                            </button>
                        </div>
                    }
                }).collect_view()}
            </div>
        </Show>

        <div class="p-4 bg-white border-t border-gray-200 shadow-sm w-full relative fixed bottom-0">
            <form class="flex items-center gap-3  mx-auto " on:submit=on_submit>
                
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
                        <svg xmlns="http://w3.org" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-5 h-5">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M18.364 18.707m-1.414 1.414a5 5 0 11-7.071-7.071l10.607-10.607a3.5 3.5 0 114.95 4.95L10.607 18.007a2 2 0 11-2.828-2.828l10.607-10.607m-4.243 4.243L8.485 14.586" />
                        </svg>
                    </label>
                </div>

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
                    class="p-3 bg-blue-600 hover:bg-blue-700 text-white rounded-xl shadow-md hover:shadow-lg active:scale-95 transition-all focus:outline-none"
                >
                    <svg xmlns="w3.org" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-5 h-5 transform rotate-45 -translate-x-0.5 translate-y-0.5">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M6 12L3.269 3.126A59.768 59.768 0 0121.485 12 59.77 59.77 0 013.27 20.876L5.999 12zm0 0h7.5" />
                    </svg>
                </button>

            </form>
        </div>
    }


}










