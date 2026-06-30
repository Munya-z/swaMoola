use leptos::ev;
use leptos::prelude::*; 
use crate::interceptor::authenticated_fetch; 
use leptos_router::hooks::use_navigate; 
use leptos::serde_json; 
use reqwest::Method;  
use crate::auth::models::AuthenticatedUser; 
use crate::chats::models::{SearchPayload, SearchResult};

#[component]
pub fn SearchUser( 
) -> impl IntoView { 
    let navigate = use_navigate();
    let value_for_search = navigate.clone();

    let user = window() 
        .local_storage() 
        .ok()
        .flatten()
        .and_then(|s| s.get_item("auth_user").ok().flatten()) 
        .and_then(|json| serde_json::from_str::<AuthenticatedUser>(&json).ok()); 

    let user_uuid = user.as_ref().map(|u| u.uuid.to_string()).unwrap_or_default();
    let user_uuid_for_search = user_uuid.clone();

    let (search_result, set_search_result) = signal(Option::<SearchResult>::None);
    let (search_content, set_search_content) = signal(String::new());
    let (_error_msg, set_error_msg) = signal(Option::<String>::None);

    let on_search_submit = move |ev: ev::SubmitEvent| {
        ev.prevent_default();
        set_error_msg.set(None);
        log::info!("im working YeePy ");

        if search_content.get().trim().is_empty() {
            set_error_msg.set(Some("search cannot be empty.".to_string()));
            return;
        }

        let navigate = navigate.clone();
        let content = search_content.get().trim().to_string();
        let payload = SearchPayload {
            key: content.clone(),
        };
        log::info!("im still going forward YeePy ");
        let _ = dotenvy::dotenv();
        let base_url = std::env::var("BACKEND_WS_URL")
        .unwrap_or_else(|_| "http://localhost:8000".to_string());  
        let url = format!("{base_url}/api/uu/sk/{}", user_uuid_for_search);

        
         leptos::task::spawn_local( async move { 
            let res: Result<reqwest::Response, reqwest::Error> = 
                authenticated_fetch(Method::POST, &url, navigate.clone(), Some(payload)).await; 
            log::info!("im still going forward YeePy res was run ");
            
            match res { 
                Ok(resp) => {
                    if resp.status().is_success() {
                        log::info!("im working YeePy , in side res");
                        // Clear the input field on successful transmission
                        let text = resp.text().await.unwrap_or_default();
                        if let Ok(parsed_user) = serde_json::from_str::<SearchResult>(&text) {
                            // Save the profile metadata to state
                            log::info!("im working YeePy , in search {:?}", parsed_user);
                            set_search_result.set(Some(parsed_user)); 
                            set_search_content.set(String::new());
                        }
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
        
        <form class=" flex items-start p-4 mb-4" on:submit=on_search_submit> 
            <input type="text" 
                   id="search"
                   placeholder="Search or start new chat" 
                   prop:value=search_content 
                   on:input=move |ev| set_search_content.set(event_target_value(&ev)) 
                   class="flex-1 px-4 py-2 rounded-full bg-[#f0f2f5] focus:outline-none focus:ring-2 focus:ring-[#00a884]"/>
            <button type="submit" class="ml-2 px-4 py-2 bg-gray-50 text-white rounded-full hover:bg-gray-200 transition-colors">
                <svg fill="#000000" width="1.5rem" height="1.5rem" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                    <path d="M10.035,18.069a7.981,7.981,0,0,0,3.938-1.035l3.332,3.332a2.164,2.164,0,0,0,3.061-3.061l-3.332-3.332A8.032,8.032,0,0,0,4.354,4.354a8.034,8.034,0,0,0,5.681,13.715ZM5.768,5.768A6.033,6.033,0,1,1,4,10.035,5.989,5.989,0,0,1,5.768,5.768Z"/>
                </svg>
            </button>
        </form>

        <Suspense fallback=|| view! { <p>"searched chats would go here..."</p> }> 
            {move || { 
                search_result.get().map(|data: SearchResult| { 
                    let navigate = value_for_search.clone();
                    let target_id = data.target_user_id.clone(); 
                    _=save_search_result(&data);
                    view! { 
                            <ul  class=" p-4 mt-4 w-full bg-gray-50 "> 
                                <li on:click={
                                        
                                        move |_| {
                                            let target_url = format!("/chats/c/{}", target_id);
                                            navigate(&target_url, Default::default());
                                        }
                                    }
                                >{data.name}</li>
                                                         
                            </ul> 
                        }
                    }) 
                } 
            } 
        </Suspense>
    }
}



pub fn save_search_result(result: &SearchResult) -> Result<(), wasm_bindgen::JsValue> {

    // Get session storage
    let storage = web_sys::window()
        .and_then(|w| w.session_storage().ok())
        .flatten()
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("Session storage is not available"))?; 
    
    
    // Turn the struct into a JSON string
    let json_string = serde_json::to_string(result)
        .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))?;
    
    // Save it with the key "search_result"
    storage.set_item("search_result", &json_string)?;
    
    Ok(())
}
