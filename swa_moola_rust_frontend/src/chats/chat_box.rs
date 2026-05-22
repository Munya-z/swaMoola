use leptos::prelude::*; 
use crate::interceptor::authenticated_fetch; 
use leptos_router::hooks::use_navigate; 
use leptos::serde_json; 
use reqwest::Method;  
use leptos::ev;
use uuid::Uuid;
use crate::auth::models::AuthenticatedUser; 
use crate::chats::models::{ MessagePayload};

#[component]
pub fn ChatBox(recipient_id: Uuid,
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

    let on_submit = move |ev: ev::SubmitEvent| {
        ev.prevent_default();
        set_error_msg.set(None);

        if content.get().trim().is_empty() {
            set_error_msg.set(Some("Message cannot be empty.".to_string()));
            return;
        }

        let navigate = navigate.clone();
        let content = content.get();
        let payload = MessagePayload {
            sender_id: sender_uuid,
            recipient_id,
            content: content.clone(),
        };
        let url = format!("http://localhost:8000/api/m/sm/{}", user_uuid);

         leptos::task::spawn_local( async move { 
            let res: Result<reqwest::Response, reqwest::Error> = 
                authenticated_fetch(Method::POST, &url, navigate.clone(), Some(payload)).await; 
            
            match res { 
                Ok(resp) => {
                    if resp.status().is_success() {
                        // Clear the input field on successful transmission
                        set_content.set(String::new());
                         on_success.run(());
                    } else {
                        set_error_msg.set(Some(format!("Server returned: {}", resp.status())));
                    }
                }, 
                Err(e) => {
                    set_error_msg.set(Some(format!("Network error: {}", e)));
                }, 
            }
        }) 
    }; 

    view! {
        <div class="p-4 bg-white border-t border-gray-200 shadow-sm w-full relative fixed bottom-0">
            <form class="flex items-center gap-3  mx-auto " on:submit=on_submit>
                

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
 