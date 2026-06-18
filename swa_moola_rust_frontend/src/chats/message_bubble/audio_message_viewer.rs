use leptos::prelude::*;
use leptos::html::Audio;
use web_sys::{Blob, BlobPropertyBag, Url};
use web_sys::HtmlAudioElement;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use leptos::serde_json;
use crate::chats::message_bubble::decrypt_file_data::{decrypt_raw_file_bytes, get_encrypted_bytes_from_server};

fn format_time(seconds: f64) -> String {
    if seconds.is_nan() || seconds.is_infinite() {
        return "0:00".to_string();
    }
    let mins = (seconds / 60.0).floor() as i32;
    let secs = (seconds % 60.0).floor() as i32;
    format!("{}:{:02}", mins, secs)
}

#[component]
pub fn SecureAudio(
    media_url: String ,
    file_type: String ,
    file_key: String,   
    nonce_base: String,
) -> AnyView{

    log::info!("🔊 [SecureAudio] Component initialized for URL: {}, and file type: {}", media_url, file_key);

    let audio_ref = NodeRef::<Audio>::new();
    let (is_playing, set_is_playing) = signal(false);
    let (audio_src, set_audio_src) = signal(None::<String>);

    let tracking_signal = Memo::new(move |_| (media_url.clone(), file_type.clone()));
    let _audio_src_resource = LocalResource::new(move || {
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

            set_audio_src.set(Some(object_url));
            Some(())
        }
    });

    let (current_time, set_current_time) = signal(0.0);
    let (duration, set_duration) = signal(1.0); // Start at 1.0 to avoid divide-by-zero
    let display_current_time = move || format_time(current_time.get());
    let display_duration = move || format_time(duration.get());

    // 2. Calculate the progress percentage reactively
    let progress_percent = move || {
        (current_time.get() / duration.get()) * 100.0
    };

    // 3. Update signals when the audio element fires events
    let on_time_update = move |_| {
        if let Some(audio) = audio_ref.get() {
            set_current_time.set(audio.current_time());
        }
    };

    let on_loaded_metadata = move |_| {
        if let Some(audio) = audio_ref.get() {
            set_duration.set(audio.duration());
        }
    };

    let on_audio_ended = move |_| {
        set_is_playing.set(false);
        set_current_time.set(0.0);
    };

    let toggle_play = move |_| {
        if let Some(audio) = audio_ref.get() {
            let audio_element: &HtmlAudioElement = &audio;
            if is_playing.get() {
                let _ = audio_element.pause();
                set_is_playing.set(false);
            } else {
                let _ = audio_element.play();
                set_is_playing.set(true);
            }
        }
    };

    // Cleanup the Blob URL when the component unmounts to prevent memory leaks
    on_cleanup(move || {
        if let Some(url) = audio_src.get_untracked() {
            let _ = web_sys::Url::revoke_object_url(&url);
        }
    });

    view! {
        <Suspense fallback=move || view! { <div class="text-xs text-gray-400"> "Loading audio..." </div> }.into_any()>
            <>
                <audio 
                    node_ref={audio_ref} 
                    src=move || audio_src.get()
                    on:timeupdate=on_time_update
                    on:loadedmetadata=on_loaded_metadata
                    on:ended=on_audio_ended
                    preload="auto"
                />

                <div class="flex items-center gap-4 bg-transparent p-3  w-72 shadow-lg max-w-full">
                    <button 
                        class="flex items-center justify-center w-10 h-10 rounded-full bg-transparent hover:bg-slate-600 active:scale-95 text-white font-bold text-lg cursor-pointer disabled:bg-slate-800 disabled:text-slate-600 disabled:cursor-not-allowed disabled:scale-100 transition-all" 
                        on:click=toggle_play 
                        disabled={move || audio_src.get().is_none()}
                    >
                        {move || match (audio_src.get(), is_playing.get()) {
                            (None, _) => view! { <span>"⏳"</span> }.into_any(),
                            (Some(_), true) => view! { <span>"⏸"</span> }.into_any(),
                            (Some(_), false) => view!{
                                <svg width="2rem" height="2rem" viewBox="0 0 36 36" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" aria-hidden="true" role="img" class="iconify iconify--twemoji" preserveAspectRatio="xMidYMid meet">
                                    <path fill="#3B88C3" d="M36 32a4 4 0 0 1-4 4H4a4 4 0 0 1-4-4V4a4 4 0 0 1 4-4h28a4 4 0 0 1 4 4v28z"></path>
                                    <path fill="#FFF" d="M8 7l22 11L8 29z"></path>
                                </svg>}.into_any(),
                        }}
                    </button>
                    
                    <div class="flex-1 flex flex-col gap-1 min-w-0">
                        <div class="w-full h-1.5 bg-neutral-700 rounded-full overflow-hidden">
                            <div class="h-full bg-black transition-all duration-100" 
                            style:width=move || format!("{}%", progress_percent())
                            ></div>
                        </div>
                        <div class="flex justify-end items-center text-[10px] font-medium text-slate-400 select-none font-mono">
                            {move || if is_playing.get() {
                                view! { <span> {display_current_time()}</span> }.into_any()
                            }else{
                                view! { <span> {display_duration()}</span> }.into_any()
                            }}
                        </div>
                    </div>
                </div>
            </>    
        </Suspense>
    }.into_any()
}


