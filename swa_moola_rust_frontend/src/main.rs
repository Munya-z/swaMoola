use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

mod interceptor;
mod auth;
mod main_page;
mod auth_state; 
mod chats;

use chats::{chats_list::ChatsList, open_chat::OpenChat};
use auth::login_ui::LoginComponent;
use auth::register_ui::RegisterComponent;
use main_page::home::Home;
use auth_state::AuthState; 


#[component]
pub fn App() -> impl IntoView {
    console_error_panic_hook::set_once();

    let initial_token = window()
        .local_storage().ok().flatten()
        .and_then(|storage| storage.get_item("auth_token").ok().flatten());

    let auth_state = RwSignal::new(AuthState { token: initial_token });
    
    // Provide this signal to all child components
    provide_context(auth_state);

    view! {
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
                    // Dynamic parameter example
                    <Route path=path!("/chats/:id") view=OpenChat/>
                    <Route path=path!("/chats/c/:id") view=OpenChat/>
                </Routes>
            </main>
        </Router>
    }
}

fn main() {

    leptos::mount::mount_to_body(App);
}


// CREATE TABLE user_presence (
//     user_id BIGINT PRIMARY KEY,
//     last_seen TIMESTAMP WITH TIME ZONE,
//     is_visible BOOLEAN DEFAULT TRUE -- The user's privacy preference
// );


