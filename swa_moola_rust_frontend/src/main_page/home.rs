use leptos::prelude::*;
use crate::auth_state::AuthState;

#[component]
pub fn Home() -> impl IntoView {
    
    let auth = use_context::<RwSignal<AuthState>>()
        .expect("AuthState should be provided in context");

    view! {
        {move || match auth.get().token{
            Some(_) => view! { <LoggedInHome /> }.into_any(),
            None => view! { <p>"You need to login first."</p> }.into_any(),
        }}
           
    }
    
}

#[component]
fn LoggedInHome() -> impl IntoView {

    let auth = use_context::<RwSignal<AuthState>>()
        .expect("AuthState should be provided");

    view! {
        <div class="flex flex-col items-center justify-center min-h-screen">
            <h1 class="text-3xl font-bold">"Welcome to SwaMoola!"</h1>
            
            // Access the token or user data reactively
            {move || {
                if let Some(token) = auth.get().token {
                    let display_token = token.chars().take(8).collect::<String>();
                    view! { <p class="mt-4">"You are logged in with token: " {display_token}"..."</p> }.into_any()
                } else {
                    "".into_any()
                }
            }}

            <button 
                class="mt-6 bg-red-500 text-white px-4 py-2 rounded"
                on:click=move |_| {
                    // Logout logic
                    let _ = window().local_storage().ok().flatten().map(|s| s.remove_item("auth_token"));
                    auth.update(|state| state.token = None);
                }
            >
                "Log Out"
            </button>
        </div>
    }
}