// src/chats/hooks.rs
use leptos::prelude::*;
use futures_util::StreamExt;
use ws_stream_wasm::WsMeta;
use leptos::task::spawn_local;
use log::{error, info};

pub fn use_websocket_listener(user_id: String) {
    // 1. Grab your state invalidation trigger out of context
    
    // 2. Spawn an effect so this block runs inside the browser environment
    Effect::new(move |_| {
        if user_id.is_empty() { return; }
        
        let message_trigger = use_context::<WriteSignal<i32>>()
            .expect("Missing message_trigger WriteSignal context wrapper");
        
        let url = format!("ws://localhost:8000/api/ws/{}", user_id.clone());
        
        spawn_local(async move {
            match WsMeta::connect(&url, None).await {
                Ok((_, mut ws_stream)) => {
                    // Listen continuously for incoming background socket frames
                    info!("✅ WebSocket connection established successfully!");
                    
                    while let Some(msg) = ws_stream.next().await {
                        info!("Raw WS Frame caught by background worker: {:?}", msg);
                        // 3. Increment the signal. Any LocalResource tracking this 
                        // signal will instantly trigger a network re-fetch.
                        message_trigger.update(|n| *n += 1);
                    }
                }
                Err(e) => {
                    error!("WS Connection failed: {}", e);
                }
            }
        });
    });
}
