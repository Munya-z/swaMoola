use leptos::prelude::*;
use uuid::Uuid;
use reqwest::Method;
use serde::{Deserialize, Serialize};  
use leptos::{serde_json, ev, task::spawn_local};
use leptos_router::{NavigateOptions,hooks::{use_navigate}};
use crate::interceptor::authenticated_fetch; 
use crate::chats::models::{ConversationPayload,ChatPayload, ConversationPayloadWithStringKeys, ConversationListPayload};

#[derive(Debug,Clone , Deserialize, Serialize)]
pub struct GroupPayload{
    pub name: String,
    pub conv_id: Uuid,
    pub other_user_ids: Vec<Uuid>
}

pub async fn create_new_group_chat(
    user_uuid: Uuid,
    new_group_name: String,
    conv_id: Uuid,
    new_participants: Vec<Uuid>,
    mut navigate: impl Fn(&str, NavigateOptions) + Clone + 'static,
)-> Result<(), wasm_bindgen::JsValue> {

    
    navigate = navigate.clone();
    let _ = dotenvy::dotenv();
    let base_url = std::env::var("BACKEND_WS_URL")
    .unwrap_or_else(|_| "http://localhost:8000".to_string()); 
    let url = format!("{base_url}/api/m/cg/{}", user_uuid);
    let payload = GroupPayload { 
        name : new_group_name,
        conv_id ,
        other_user_ids: new_participants
    }; 

    let res = authenticated_fetch(Method::POST, &url, navigate.clone(), Some(payload)).await; 
    
    match res { 
        Ok(resp) => {
            log::info!("this is the response status : {:?}", &resp.status());
            if resp.status().is_success() {
                let text = resp.text().await
                    .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))?;

                let data: ConversationPayload = serde_json::from_str(&text)
                    .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))?;

                let storage = web_sys::window()
                    .and_then(|w| w.session_storage().ok())
                    .flatten()
                    .ok_or_else(|| wasm_bindgen::JsValue::from_str("Session storage is not available"))?; 

                let json_string = serde_json::to_string(&data)
                    .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))?;
                
                storage.set_item("new_group", &json_string)?;

                let next_url = format!("/chats/{}", data.conv_id);
                let navigate_clone = navigate.clone();
                request_animation_frame(move || {
                    navigate_clone(&next_url, Default::default());
                });

                Ok(()) 
            } else {
                Err(wasm_bindgen::JsValue::from_str("Server returned an error status code"))                        
            }
           
        }, 
        Err(e) => {
            Err(wasm_bindgen::JsValue::from_str(&format!("Fetch failed: {}", e)))
        }, 
    }
}


#[component]
pub fn MakeGroupChat(conv_id:Uuid, user_uuid: Uuid) -> impl IntoView {

    let (group_name, set_group_name) = signal(String::new());
    let (new_participants, set_new_participants) = signal(Vec::<Uuid>::new());
    let (error_msg, set_error_msg) = signal(Option::<String>::None);

    let navigate = use_navigate();
    let navigate_resource = navigate.clone();

    let chats_resource : LocalResource<Option<Vec<ConversationPayloadWithStringKeys>>> = LocalResource::new(move || { 
        let navigate = navigate_resource.clone(); 
        let base_url = option_env!("BACKEND_URL").unwrap_or("http://localhost:8000");   
        let url = format!("{base_url}/api/m/conversations/{}", user_uuid); 
        let ch_url = format!("{base_url}/api/m/ch/{}", user_uuid);
        
        async move { 
            let res: Result<reqwest::Response, reqwest::Error> = 
                authenticated_fetch(Method::GET, &url, navigate.clone(), None::<()>).await; 
                
                match res { 
                    Ok(resp) =>{
                    if resp.status().is_success() {
                        let text = resp.text().await.unwrap_or_default();

                        if let Ok(mut conversations) = serde_json::from_str::<Vec<ConversationListPayload>>(&text) {
                            
                            conversations.retain(|chat| {
                                let is_current_conv = chat.conv_id == conv_id; 
                                let is_group_chat = chat.is_group;        
                                
                                !is_current_conv && !is_group_chat
                            });
                            let mut fully_populated_chats = Vec::new();

                            for conversation in conversations{
                                let payload = ChatPayload { 
                                    conv_id: conversation.conv_id 
                                };
                                 
                                let details_res = authenticated_fetch(
                                    Method::POST, 
                                    &ch_url, 
                                    navigate.clone(), 
                                    Some(payload)
                                ).await; 

                                if let Ok(details_resp) = details_res {
                                    if details_resp.status().is_success() {
                                        let details_text = details_resp.text().await.unwrap_or_default();
                                        
                                        if let Ok(data) = serde_json::from_str::<ConversationPayloadWithStringKeys>(&details_text) {
                                            fully_populated_chats.push(data);
                                        }
                                    }
                                }
                            }

                            Some(fully_populated_chats)
                        } else { None }
                    }else { None }
                }
                Err(_) => {
                    None
                } 
            } 
        } 
    }); 


    let on_submit = move |ev: ev::SubmitEvent| {
        ev.prevent_default();

        let name = group_name.get_untracked();
        let participants_list = new_participants.get_untracked();
        let nav = navigate.clone();
        set_error_msg.set(None);

        spawn_local(async move {
            let _ =create_new_group_chat(
                user_uuid,
                name,
                conv_id,
                participants_list,
                nav
            ).await;
        });
    };

    view! {
        <main class="w-full flex  justify-center ">
            <section class="p-4 mx-auto w-full items-center justify-center ">
                <form class="p-2 mx-auto w-full flex flex-col items-center justify-center " on:submit=on_submit>
                    <div class="py-5">
                        <label class="my-2" for="phone"> "group_name"</label>
                        <br/>
                        <input 
                        type="text" 
                        name="group_name"
                        id="group_name"
                        class="border border-gray-300 rounded-md px-4 py-2 text-gray-700 bg-white shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent" 
                        on:input=move |ev| set_group_name.set(event_target_value(&ev))
                        />
                    </div>

                    <Suspense fallback=|| view! { <p>"Loading chats..."</p> }> 
                        {move || { 
                            chats_resource.get().map(|data: Option<Vec<ConversationPayloadWithStringKeys>>| { 
                                log::info!("chat resource data : {:?}", &data);
                                match data { 
                                    Some(chats) => view! { 
                                        <ul class="max-w-[600px] w-full bg-white font-sans flex flex-col gap-2"> 
                                            {chats.into_iter().map(|chat| { 
                                                let Some(recipient_id) = chat.recipient_id else {
                                                    return view! { <div></div> }.into_any(); 
                                                };
                                                
                                                let is_added = move || new_participants.with_untracked(|list: &Vec<Uuid>| list.contains(&recipient_id));

                                                view! { 
                                                    <div class="flex flex-row justify-between items-center p-2 hover:bg-gray-50 rounded-lg">
                                                        <li style="cursor: pointer; padding: 8px; margin: 4px 0; transition: background 0.2s;">
                                          
                                                            <div class="flex items-center cursor-pointer select-none">
                                                                <img class="w-12 h-12 rounded-full object-cover mr-4" src="/images/chat_bg.png" alt="Profile"/>
                                                                <div class="flex-1 min-w-0 border-b border-[#f0f2f5] pb-2.5">
                                                                <div class="flex justify-between items-baseline mb-1">
                                                                    <h3 class="text-[16px] font-medium text-[#111b21] truncate">{chat.display_name}</h3>
                                                                    // <span class="text-xs text-[#00a884] font-medium whitespace-nowrap ml-1.5">{format_chat_time(&single_chat.last_msg_date.unwrap_or_else(chrono::Utc::now))}</span>
                                                                </div>
                                                                <div class="flex justify-between items-center">
                                                                    // <p class="text-sm text-[#667781] truncate flex-1">{single_chat.last_msg_content.as_deref().unwrap_or_else(|| "No messages yet")}</p>
                                                                    <span class="bg-[#00a884] text-white text-xs font-semibold min-w-[20px] h-5 rounded-full flex items-center justify-center px-1 ml-2">""</span>
                                                                </div>
                                                                </div>
                                                            </div> 

                                                            {move || if !is_added() {
                                                                view! {
                                                                    <button
                                                                        type="button" 
                                                                        // :prevent_default
                                                                        on:click=move |_| {
                                                                            set_new_participants.update(|list: &mut Vec<Uuid>| {
                                                                                list.push(recipient_id);
                                                                            });
                                                                        }
                                                                        class="px-3 py-1.5 bg-[#00a884] text-white text-sm font-medium rounded-md hover:bg-[#008f70]">
                                                                        "Add"
                                                                    </button>
                                                                }.into_any()
                                                            } else {
                                                                view! {
                                                                    <button 
                                                                        type="button"
                                                                        on:click=move |_| {
                                                                            set_new_participants.update(|list: &mut Vec<Uuid>| {
                                                                                list.retain(|id| *id != recipient_id);
                                                                            });
                                                                        }
                                                                        class="px-3 py-1.5 bg-red-100 text-red-600 text-sm font-medium rounded-md hover:bg-red-200">
                                                                        "Remove"
                                                                    </button>
                                                                }.into_any()
                                                            }}

                                                        </li>
                                                    </div>    
                                                }.into_any() 
                                            }).collect_view()} 
                                        </ul> 
                                    }.into_any(), 
                                    None => view! { <p>"No chats found or error loading."</p> }.into_any(), 
                                } 
                            }) 
                        }} 
                    </Suspense> 

                    <button type="submit" class="w-[90%] max-w-[200px] bg-blue-600 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded-md transition duration-300 ease-in-out shadow-md hover:shadow-lg active:scale-95">"create group"</button>

                    {move || error_msg.get().map(|msg| view! { 
                        <p class="text-red-500 mt-2 font-bold">{msg}</p> 
                    })}

                </form>

            </section>
        </main>
    }
}