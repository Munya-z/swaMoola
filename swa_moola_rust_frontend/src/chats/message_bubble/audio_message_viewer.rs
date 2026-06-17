use leptos::prelude::*;
use web_sys::{Blob, BlobPropertyBag, Url};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use leptos::serde_json;
use crate::chats::message_bubble::decrypt_file_data::{decrypt_raw_file_bytes, get_encrypted_bytes_from_server};

#[component]
pub fn SecureAudio(
    media_url: String ,
    file_type: String ,
    file_key: String,   
    nonce_base: String,
) -> AnyView{

    log::info!("🔊 [SecureAudio] Component initialized for URL: {}, and file type: {}", media_url, file_key);

    let tracking_signal = Memo::new(move |_| (media_url.clone(), file_type.clone()));
    let audio_src_resource = LocalResource::new(move || {
        let (url, mime_type) = tracking_signal.get();
        let key = file_key.clone();
        let nonce_str = nonce_base.clone();

        async move {
           
            let raw_downloaded =get_encrypted_bytes_from_server(url).await?;
            
            let encrypted_bytes = if !raw_downloaded.is_empty() && raw_downloaded[0] == 91 {
                let text_string = String::from_utf8(raw_downloaded.clone()).unwrap_or_default();
                serde_json::from_str::<Vec<u8>>(&text_string).unwrap_or_default()
                } else {
                    raw_downloaded
                };

            let decoded_key_bytes = STANDARD.decode(&key)
                .map_err(|e| format!("Failed to decode key from Base64: {:?}", e)).ok()?;
            let key_array: [u8; 32] = decoded_key_bytes.try_into()
                .map_err(|e|format!("Failed to turn decoded key bytes into array: {:?}", e) ).ok()?;
                      
            let decrypted_bytes = decrypt_raw_file_bytes(
                &encrypted_bytes,
                &nonce_str,
                &key_array
            ).ok()?;


            let js_array = js_sys::Uint8Array::from(&decrypted_bytes[..]);
            let array_parts = js_sys::Array::new();
            array_parts.push(&js_array);

            let options = BlobPropertyBag::new();
            let mime = if mime_type.is_empty() { "audio/mpeg".to_string() } else { mime_type };
            options.set_type(&mime.as_str());

            let blob = Blob::new_with_u8_array_sequence_and_options(&array_parts, &options).ok()?;
            let object_url = Url::create_object_url_with_blob(&blob).ok()?;
            
            Some(object_url)
        }
    });

    // Cleanup the Blob URL when the component unmounts to prevent memory leaks
    on_cleanup(move || {
        if let Some(Some(url)) = audio_src_resource.get_untracked() {
            let _ = Url::revoke_object_url(&url);
        }
    });

    view! {
        <Suspense fallback=move || view! { <div class="text-xs text-gray-400"> "Loading audio..." </div> }.into_any()>
            {move || audio_src_resource.get().map(|maybe_src| {
                if let Some(src) = maybe_src {
                    view! {
                        <audio src=src controls preload="metadata"  class="w-full">
                            "Your browser does not support the audio tag."
                        </audio>
                    }.into_any()
                } else {
                    view! {
                         <div class="text-xs text-red-500"> "Failed to display audio" </div> 
                    }.into_any()
                }
            })}
        </Suspense>
    }.into_any()
}