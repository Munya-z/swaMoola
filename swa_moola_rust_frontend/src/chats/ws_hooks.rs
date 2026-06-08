use leptos::prelude::*;
use futures_util::StreamExt;
use ws_stream_wasm::WsMeta;
use leptos::task::spawn_local;
use log::{error, info};

pub fn use_websocket_listener(user_id: String) {
    
    Effect::new(move |_| {
        if user_id.is_empty() { return; }
        
        let message_trigger = use_context::<WriteSignal<i32>>()
            .expect("Missing message_trigger WriteSignal context wrapper");
        
        let url = format!("ws://localhost:8000/api/ws/{}", user_id.clone());
        
        spawn_local(async move {
            match WsMeta::connect(&url, None).await {
                Ok((_, mut ws_stream)) => {
     
                    info!("✅ WebSocket connection established successfully!");
                    
                    while let Some(msg) = ws_stream.next().await {
                        info!("Raw WS Frame caught by background worker: {:?}", msg);

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
