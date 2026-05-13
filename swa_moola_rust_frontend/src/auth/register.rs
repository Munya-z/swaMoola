use leptos_router::{components::*, hooks::use_navigate};
use leptos::{prelude::*,task::spawn_local, ev};

use crate::auth::models::RegisterCredentials;

#[component]
pub fn RegisterComponent() -> impl IntoView {

    let (name, set_name) = signal(String::new());
    let (phone_number, set_phone_number) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (error_msg, set_error_msg) = signal(Option::<String>::None);

    let navigate = use_navigate();

    let on_submit = move |ev: ev::SubmitEvent| {
        ev.prevent_default();
        set_error_msg.set(None);

        let navigate = navigate.clone();
        let name_val = name.get();
        let phone_val = phone_number.get();
        let pass_val = password.get();
        
        spawn_local(async move {
            let client = reqwest::Client::new();
            let res = client
                .post("http://localhost:8000/users/register") 
                .json(&RegisterCredentials { name: &name_val, phone_number: &phone_val, password: &pass_val })
                .send()
                .await;

            match res {
                Ok(response) if response.status().is_success() => {
                    log::info!("Registering successful!");
                    navigate("/login", Default::default());
                }
                Ok(response) => {
                    let msg = format!("Registering failed with status: {}", response.status());
                    log::error!("{}", msg);
                    set_error_msg.set(Some(msg));
                }
                Err(e) => {
                    let msg = format!("Network error: {}", e);
                    log::error!("{}", msg);
                    set_error_msg.set(Some(msg));
                    
                }
            }
        });
    };

    view! {
        <main class="min-h-screen w-full flex items-center justify-center ">
            <section class="p-4 mx-auto w-[90%] max-w-[600px]  flex flex-col items-center justify-center ">
                <h1 class="text-center text-5xl font-bold">Register</h1>
                <form class="p-2 mx-auto w-full flex flex-col items-center justify-center " on:submit=on_submit>
                    <div class="py-5">
                        <label class="my-2" for="name"> "name"</label>
                        <br/>
                        <input 
                        type="text" 
                        name="name"
                        id="name"
                        class="border border-gray-300 rounded-md px-4 py-2 text-gray-700 bg-white shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent" 
                        on:input=move |ev| set_name.set(event_target_value(&ev))
                        />
                    </div>

                    <div class="py-5">
                        <label class="my-2" for="phone"> "Phone number"</label>
                        <br/>
                        <input 
                        type="tel" 
                        pattern=r"\+[0-9]{1,3}\s?[0-9\s]{7,15}"  name="phone"
                        id="phone"
                        class="border border-gray-300 rounded-md px-4 py-2 text-gray-700 bg-white shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent" 
                        on:input=move |ev| set_phone_number.set(event_target_value(&ev))
                        />
                    </div>

                    <div class="py-5 ">
                        <label class="my-2" for="password"> "Password"</label>
                        <br/>
                        <input 
                        type="password" 
                        name="password"
                        id="password"
                        class="border border-gray-300 rounded-md px-4 py-2 text-gray-700 bg-white shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        on:input=move |ev| set_password.set(event_target_value(&ev))
                        />
                    </div>

                    <button type="submit" class="w-[90%] max-w-[200px] bg-blue-600 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded-md transition duration-300 ease-in-out shadow-md hover:shadow-lg active:scale-95">"Register"</button>

                    {move || error_msg.get().map(|msg| view! { 
                        <p class="text-red-500 mt-2 font-bold">{msg}</p> 
                    })}
                    <div class="py-5">
                        <A href="/login">"already have an account? Click here to Login"</A>
                    </div>
                </form>

            </section>
        </main>
    }
}