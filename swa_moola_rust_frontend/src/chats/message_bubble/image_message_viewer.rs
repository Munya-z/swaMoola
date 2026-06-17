use leptos::{prelude::*};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use leptos::serde_json;
use crate::chats::message_bubble::decrypt_file_data::{decrypt_raw_file_bytes, get_encrypted_bytes_from_server};

#[component]
pub fn SecureImage(media_url: String, 
    alt_text: String, 
    file_type: String, 
    file_key: String,   
    nonce_base: String,) -> impl IntoView {

    let is_expanded = RwSignal::new(false);
    let alt_text_signal = Memo::new(move |_| alt_text.clone());

    let image_src_resource = LocalResource::new(move || {
        let url = media_url.clone();
        let mime_type = file_type.clone();
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
            
            let encoded_bytes = STANDARD.encode(&decrypted_bytes);
            let mime = if mime_type.is_empty() { "image/jpeg".to_string() } else { mime_type };
            let base64_src = format!("data:{};base64,{}", mime, encoded_bytes);
            Some(base64_src)
        }
    });

    view! {
        <Suspense fallback=move || view! { <div class="text-xs text-gray-400"> "Loading..." </div> }>
            {move || image_src_resource.get().map(|maybe_src| {
                if let Some(src) = maybe_src {

                    let saved_src = StoredValue::new(src);

                    view! {
                        <img 
                            src= move || saved_src.get_value() 
                            alt= move || alt_text_signal.get() 
                            loading="lazy"
                            class="w-full h-auto rounded object-cover" 
                            on:click=move |_| is_expanded.set(true)
                        />

                        <Show
                            when=move || is_expanded.get()
                            fallback=|| view! { <div /> }
                        >
                            <div 
                                class="fixed inset-0 z-[9999] flex items-center justify-center bg-black/90 cursor-zoom-out animate-fade-in"
                                on:click=move |_| is_expanded.set(false)
                            >
                                // Close button in the top right corner
                                <button 
                                    class="absolute top-4 right-4 text-white/70 hover:text-white text-2xl font-bold p-2"
                                    on:click=move |_| is_expanded.set(false)
                                >
                                    "✕"
                                </button>

                                // The large full-page image
                                <img 
                                    src=move || saved_src.get_value() 
                                    alt=move || alt_text_signal.get() 
                                    class="max-w-[95vw] max-h-[95vh] object-contain rounded select-none shadow-2xl"
                                />
                            </div>
                        </Show>
                    }.into_any()
                } else {
                    view! {
                         <div class="text-xs text-red-500"> "Failed to display image" </div> 
                    }.into_any()
                }
            })}
        </Suspense>
    }
}

