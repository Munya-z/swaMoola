use leptos::prelude::*; 
use crate::chats::models::UserPublicKeys;
use crate::interceptor::authenticated_multipart_fetch; 
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::Utc;
use reqwest::multipart::{Form};
use rand::RngCore;
use wasm_bindgen_futures::JsFuture;
use leptos::serde_json; 
use reqwest::Method;  
use leptos_router::NavigateOptions;
use uuid::Uuid;
use crate::chats::message_encryption::encrypt_files::encrypt_raw_file_bytes;
use crate::chats::message_encryption::encrypt_files::upload_encrypted_file_to_storage;
use crate::chats::message_encryption::encrypt_message_payload::prepare_full_payload;
use crate::chats::models::{ SecretInnerPayload, AttachmentMeta};
use crate::chats::models::InboundMessagePayload;
use crate::chats::message_input::compresion::{convert_to_clean_webp_bytes, convert_to_compressed_audio_bytes};


pub fn generate_random_32_bytes() -> [u8; 32] {
    let mut rng = rand::thread_rng();
    let mut key = [0u8; 32];
    rng.fill_bytes(&mut key);
    key
}

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
    s_x25519: [u8; 32], 
    s_mlkem: [u8; 1184],
    recipients: Vec<UserPublicKeys>,
){
    error_msg.set(None);
    let raw_content = content.get_untracked();
    let raw_files = files.get_untracked();

    if raw_content.trim().is_empty() && raw_files.is_empty() {
        error_msg.set(Some("Message cannot be empty.".to_string()));
        return;
    }

    let text_content = if raw_content.trim().is_empty() {
        if !raw_files.is_empty() {
            " ".to_string() 
        } else {
            "".to_string()
        }
    } else {
        content.get()
    };

    let navigate = navigate.clone();
    
    let url = format!("http://localhost:8000/api/m/sm/{}", user_uuid);

    leptos::task::spawn_local(async move { 

        let mut attachments = Vec::new();

        for file in raw_files {
            let file_name = file.name();
            let file_type = file.type_();
            let file_size = file.size() as i32;
            let navigate_clone = navigate.clone();
                     
            let (raw_file_bytes, final_file_type) = if file_type.starts_with("audio/") {
                match convert_to_compressed_audio_bytes(file.clone()).await {
                    Ok(bytes) => (bytes, "audio/wav".to_string()),
                    Err(_) => {
                        let array_buffer = JsFuture::from(file.array_buffer()).await.unwrap();
                        (js_sys::Uint8Array::new(&array_buffer).to_vec(), file_type.clone())
                    }
                }
            } else if file_type.starts_with("image/") {
                match convert_to_clean_webp_bytes(file.clone()).await {
                    Ok(bytes) => (bytes, "image/webp".to_string()),
                    Err(_) => {
                        let array_buffer = JsFuture::from(file.array_buffer()).await.unwrap();
                        (js_sys::Uint8Array::new(&array_buffer).to_vec(), file_type.clone())
                    }
                }
            } else {
                let array_buffer_promise = file.array_buffer();
                if let Ok(buffer_value) = JsFuture::from(array_buffer_promise).await {
                    (js_sys::Uint8Array::new(&buffer_value).to_vec(), file_type.clone())
                } else {
                    continue;
                }
            };
            
            let file_secret_key = generate_random_32_bytes();
            
            let (encrypted_file_bytes, nonce_base) = encrypt_raw_file_bytes(&raw_file_bytes, &file_secret_key).expect("Symmetric file encryption operation failed unexpectedly");
            
            let storage_url = upload_encrypted_file_to_storage(navigate_clone, encrypted_file_bytes).await.unwrap();
            attachments.push(AttachmentMeta {
                file_name,
                file_type : final_file_type,
                file_size,
                storage_url :  storage_url.clone(),
                file_key: STANDARD.encode(file_secret_key),
                nonce_base: STANDARD.encode(nonce_base),
            });
        }
    

        let inner_data = SecretInnerPayload {
            sender_id: sender_uuid,
            timestamp_ms: Utc::now(),
            text_message: text_content,
            attachments,
        };

        let final_payload = prepare_full_payload(
            inner_data,
            s_x25519, 
            s_mlkem,
            recipients,
            recipient_id
        );

        let form = Form::new()
            .text("recipient_id", recipient_id.to_string())    
            .text("ciphertext", final_payload.ciphertext)
            .text("nonce", final_payload.nonce)
            .text("envelopes", serde_json::to_string(&final_payload.envelopes).unwrap());
 

        let res: Result<reqwest::Response, reqwest::Error> = 
            authenticated_multipart_fetch(Method::POST, &url, navigate.clone(), Some(form)).await; 
            
        match res { 
            Ok(resp) => {
                if resp.status().is_success() {
                    set_content.set(String::new());
                    set_files.set(Vec::new());

                    if is_recipient {
                        let text = resp.text().await.unwrap_or_default();
                        if let Ok(data) = serde_json::from_str::<InboundMessagePayload>(&text) {
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
                    error_msg.set(Some(format!("Server returned error status: {}", resp.status())));
                }
            }, 
            Err(e) => {
                error_msg.set(Some(format!("Network layer failure: {}", e)));
            }, 
        }
    }) 
}