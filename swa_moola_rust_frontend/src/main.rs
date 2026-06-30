use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use crate::chats::ws_hooks::CallState;

mod interceptor;
mod auth;
mod main_page;
mod auth_state; 
mod chats;
use crate::chats::calls::SignalingMessage;
use chats::{chats_list::ChatsList, open_chat::OpenChat, calls::WebRtcCall};
use auth::login_ui::LoginComponent;
use auth::register_ui::RegisterComponent;
use main_page::home::Home;
use auth_state::AuthState; 
use leptos::serde_json;
use crate::auth::models::AuthenticatedUser;
use crate::chats::ws_hooks::use_websocket_listener;
use crate::chats::calls::IncomingCallModal;


#[derive(Clone)]
pub struct IncomingSignalStream(pub ReadSignal<Option<SignalingMessage>>);


#[component]
pub fn App() -> impl IntoView {
    console_error_panic_hook::set_once();

    let initial_token = window()
        .local_storage().ok().flatten()
        .and_then(|storage| storage.get_item("auth_token").ok().flatten());

    let auth_state = RwSignal::new(AuthState { token: initial_token });
    provide_context(auth_state);

    let (global_message_read, global_message_write) = signal(0);
    provide_context(global_message_write);
    
    let user =  window() 
        .local_storage() 
        .ok()
        .flatten()
        .and_then(|s| s.get_item("auth_user").ok().flatten()) 
        .and_then(|json| serde_json::from_str::<AuthenticatedUser>(&json).ok());

    let user_id = user.as_ref().map(|u| u.uuid.to_string()).unwrap_or_default();
    use_websocket_listener(user_id);
    
    view! {

        <IncomingCallModal />

        <Router>
            <nav class="px-10 fixed top-0 left-0 w-full h-16 bg-white shadow-md z-50 flex items-center justify-end gap-2 ">
                <A href="/" exact=true attr:class="aria-[current=page]:underline" >"Home"</A>
                {move || match auth_state.get().token{
                    Some(_) => view! {
                        <div class="sm:hidden">
                        <A href="/chats" attr:class="aria-[current=page]:underline " >"Chats"</A> 
                        </div>
                        <div class="hidden sm:block">
                        <A href="/chats/c" attr:class="aria-[current=page]:underline " >"Chats"</A> 
                        </div>
                    }.into_any(),
                    None => view! {
                        <A href="/login" attr:class="aria-[current=page]:underline " >"Login"</A>
                        }.into_any(),
                }}
            </nav>
            <div class="h-16"></div>
            <main>
                <Routes fallback=|| "Page not found.">
                    <Route path=path!("/") view=Home/>
                    <Route path=path!("/login") view=LoginComponent/>
                    <Route path=path!("/register") view=RegisterComponent/>
                    <Route path=path!("/chats") view=ChatsList/>
                    <Route path=path!("/chats/c") view=OpenChat/>
                    // Dynamic parameters
                    <Route path=path!("/chats/:id") view=OpenChat/>
                    <Route path=path!("/chats/c/:id") view=OpenChat/>
                    <Route path=path!("/make_call/:id") view=WebRtcCall/>

                </Routes>
            </main>
        </Router>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    leptos::mount::mount_to_body(App);
   
}




