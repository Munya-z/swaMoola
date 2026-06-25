use leptos::prelude::*;
use crate::auth_state::AuthState;
use crate::auth::models::AuthenticatedUser;
use crate::interceptor::authenticated_fetch;
use leptos_router::hooks::use_navigate;
use reqwest::Method;
use leptos::serde_json;

fn get_local_user() -> Option<AuthenticatedUser> {
    window() 
        .local_storage() 
        .ok()
        .flatten()
        .and_then(|s| s.get_item("auth_user").ok().flatten()) 
        .and_then(|json| serde_json::from_str::<AuthenticatedUser>(&json).ok())
}

pub fn change_discoverable_key(on_success: impl Fn() + 'static){
    let navigate = use_navigate();
    let (_error_msg, set_error_msg) = signal(Option::<String>::None);

    log::info!("this funtion is working and has started");

    let user = get_local_user();

    let user_uuid = user.as_ref().map(|u| u.uuid.to_string()).unwrap_or_default();
    let url :String =  format!("http://localhost:8000/api/uu/dk/{}", user_uuid );
    let navigate_for_key = navigate.clone();

    leptos::task::spawn_local(async move { 

        let res: Result<reqwest::Response, reqwest::Error> = 
            authenticated_fetch(Method::PUT, &url, navigate_for_key, Option::<()>::None).await; 

        match res { 
            Ok(resp) => {
                if resp.status().is_success() {
                    let data :String = resp.json::<String>().await.unwrap_or_default();
                    log::info!("the data from the server in discoverable key {:?}", &data);

                    if let Some(mut u) = user {
                        u.discoverable_key = Some(data); 
                        
                        
                        if let Ok(Some(storage)) = window().local_storage() {
                            if let Ok(json) = serde_json::to_string(&u) {
                                log::info!("the storage will now be updated with this data {}", &json);
                                let _ = storage.set_item("auth_user", &json);
                                log::info!("the storage has been updated");
                                on_success();
                            }
                        }
                    }

                } else {
                    log::info!("OK Response failled ");
                    set_error_msg.set(Some(format!("Server returned error status: {}", resp.status())));
                }
            }, 
            Err(e) => {
                log::info!("server failled ");
                set_error_msg.set(Some(format!("failed to change discoverable key: {}", e)));
            }, 
        }
    })

}

#[component]
pub fn Home() -> impl IntoView {
    
    let auth = use_context::<RwSignal<AuthState>>()
        .expect("AuthState should be provided in context");

    view! {
        {move || match auth.get().token{
            Some(_) => view! { <LoggedInHome /> }.into_any(),
            None => view! {
                <div class="flex flex-col items-center justify-center min-h-screen">
                    <h1 class="text-3xl font-bold">"Welcome to SwaMoola!"</h1>
                    <p class="mt-4">"You need to login first."</p>
                </div> 
                }.into_any(),
        }}
           
    }
    
}

#[component]
fn LoggedInHome() -> impl IntoView {
    let auth = use_context::<RwSignal<AuthState>>()
        .expect("AuthState should be provided in context");

    let (user, set_user) = signal(get_local_user());

    let name = move || user.get().and_then(|u| u.name.clone()).unwrap_or_default();
    let disc_key = move || user.get().and_then(|u| u.discoverable_key.clone()).unwrap_or_default();
    let trust_score = move || user.get().and_then(|u| u.trust_score.map(|t| t.to_string())).unwrap_or_default();

    view! {
        <div class="flex flex-col items-center justify-center min-h-screen">
            <h1 class="text-3xl font-bold">"Welcome to SwaMoola!"</h1>
            
            <div class="max-w-xl mx-auto my-8 bg-white border border-slate-200 rounded-2xl shadow-sm overflow-hidden font-sans">
               
                <div class="bg-slate-50 px-6 py-4 border-b border-slate-100 flex items-center justify-between">
                    <div class="flex items-center space-x-2">
                        <span class="h-2 w-2 rounded-full bg-green-500 animate-pulse"></span>
                        <h2 class="text-sm font-medium text-slate-600 tracking-wide uppercase">
                            "Logged in as: "<span class="text-slate-900 font-semibold normal-case">{name}</span>
                        </h2>
                    </div>
                </div>

                <div class="p-6 space-y-6">
                    <div class="bg-amber-50 border border-amber-200 rounded-xl p-4 space-y-3">
                        <div class="flex items-center justify-between">
                            <span class="text-sm font-medium text-amber-800">"Your Trust Score"</span>
                            <span class="text-lg font-bold text-amber-900 bg-white px-3 py-1 rounded-md shadow-sm border border-amber-100">
                                {trust_score}
                            </span>
                        </div>
                        <p class="text-xs text-amber-700 leading-relaxed">"
                            Trust can only be broken, and once it is broken, it can never be rebuilt. Use the app in line with the community guidelines. If your trust score reaches zero, your account will be terminated and your number will be permanently blocked."
                        </p>
                    </div>
                    
                    <div class="space-y-4">
                        <div class="bg-blue-50 border border-blue-100 rounded-xl p-4 space-y-3">
                            <p class="text-sm font-medium text-blue-900">
                                "Share this key with family and friends you want to contact you:"
                            </p>
                            <div class="bg-white border border-blue-200 rounded-lg p-3 flex items-center justify-between">
                                <span class="text-xs font-mono text-slate-500 uppercase tracking-wider">"Key"</span>
                                <code class="text-sm font-mono font-bold text-blue-600 tracking-md">{disc_key}</code>
                            </div>
                        </div>
                        
                        <p class="text-xs text-slate-500 leading-relaxed pl-1">
                            "You can change this key at any point if bad actors share it with people you do not want to talk to. Once changed, your current chats will remain intact, but the old key can no longer be used to find you."
                        </p>

                        <div class="bg-blue-50 border border-blue-100 rounded-xl p-4 space-y-3">
                            <p class="text-sm font-medium text-blue-900">
                                "Click here to change the discoverable key"
                            </p>
                            <button 
                                class="mt-6 bg-blue-500 text-white px-4 py-2 rounded"
                                on:click=move |_| {
                                    change_discoverable_key(move || {
                                        set_user.set(get_local_user());
                                    });
                                }
                            >
                                "change discoverable key"
                            </button>

                        </div>
                    </div>
                </div>
                
                <div class="flex justify-center w-full pb-6">
                    <button 
                    class="mt-6 bg-red-500 text-white px-4 py-2 rounded"
                    on:click=move |_| {
                        // Logout logic
                        let _ = window().local_storage().ok().flatten().map(|s| s.remove_item("auth_token"));
                        let _ = window().local_storage().ok().flatten().map(|s| s.remove_item("auth_user"));
                        auth.update(|state| state.token = None);
                    }
                    >
                    "Log Out"
                    </button>
                </div>
            </div>
        </div>
    }
}