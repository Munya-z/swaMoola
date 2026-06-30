use leptos::prelude::*;
use futures_util::{StreamExt, SinkExt};
use ws_stream_wasm::WsMeta;
use ws_stream_wasm::WsMessage;
use leptos::task::spawn_local;
use log::{error, info};
use crate::chats::calls::SignalingMessage;
use crate::chats::calls::ActiveCallSession;


#[derive(Clone, Debug, PartialEq)]
pub enum CallState {
    None,
    Incoming(IncomingCallState),
    Connected {
        target_user_id: String,
        sender_name: String,
    },
}



#[derive(Clone, Debug, PartialEq)]
pub struct IncomingCallState {
    pub sender_name: String,
    pub target_user_id: String,
    pub sdp_offer: String,
}

#[derive(Clone)]
pub struct GlobalWebSocketSender(pub Callback<String>);

#[derive(Clone, Copy)]
pub struct IncomingSignalStream(pub RwSignal<Option<SignalingMessage>>);

pub fn use_websocket_listener(user_id: String) {
    let _ = dotenvy::dotenv();

    let message_trigger = use_context::<WriteSignal<i32>>()
        .expect("Missing message_trigger WriteSignal context wrapper");

    let call_state_context = RwSignal::new(None::<CallState>);
    provide_context(call_state_context);

    let (outbound_relay_read, outbound_relay_write) = signal(String::new());

    let incoming_signal_signal = RwSignal::new(None::<SignalingMessage>);
    provide_context(IncomingSignalStream(incoming_signal_signal));

    let call_session = StoredValue::new(None);
    provide_context(ActiveCallSession(call_session));

    // 2. Map this relay writer into your global context layout for child views
    let outbound_callback = Callback::new(move |raw_json_text: String| {
        outbound_relay_write.set(raw_json_text);
    });
    provide_context(GlobalWebSocketSender(outbound_callback));

    Effect::new(move |_| {
        if user_id.is_empty() { return; }

        log::info!("the user in the web socket {:?}", &user_id);
        let base_url = std::env::var("BACKEND_WS_URL")
            .unwrap_or_else(|_| "ws://127.0.0.1:8000".to_string());  
        let url = format!("{base_url}/api/ws/{}", user_id.clone());
        
        spawn_local(async move {
            match WsMeta::connect(&url, None).await {
                Ok((ws_meta, mut ws_stream)) => {
                    info!("✅ WebSocket connection established successfully!");
                    
                    let (mut outbound_sink, mut inbound_stream) = ws_stream.split();

                    let (tx, mut rx) = futures_channel::mpsc::unbounded::<String>();
                    
                    spawn_local(async move {
                        while let Some(text_to_send) = rx.next().await {
                            let ws_frame = WsMessage::Text(text_to_send);
                            let _ = outbound_sink.send(ws_frame).await;
                        }
                    });

                    let _relay_effect = Effect::new(move |_| {
                        let text_to_send = outbound_relay_read.get();
                        if !text_to_send.is_empty() {
                            let _ = tx.unbounded_send(text_to_send);
                        }
                    });
                    
                    // 3. Central incoming message dispatcher loop
                    while let Some(msg) = inbound_stream.next().await {
                        info!("Raw WS Frame caught by background worker: {:?}", msg);

                        if let ws_stream_wasm::WsMessage::Text(text_content) = msg {
                            // Check if this incoming message string is a WebRTC call control signal
                            if let Ok(signal_msg) = leptos::serde_json::from_str::<SignalingMessage>(&text_content) {
                                incoming_signal_signal.set(Some(signal_msg.clone()));

                                match signal_msg {
                                    SignalingMessage::CallRequest { sender_name, target_user_id, sdp_offer  } => {
                                        info!("☎️ Incoming Call request from: {}", sender_name);

                                            call_state_context.set(Some(CallState::Incoming(IncomingCallState {
                                                sender_name,
                                                target_user_id, 
                                                sdp_offer,
                                            })));

                                        
                                        // TODO: Set a global modal signal true here to trigger an "Incoming Call" visual prompt
                                    },
                                    SignalingMessage::CallResponse { .. } | SignalingMessage::IceCandidate { .. } => {
                                        // TODO: Pass these straight down to your active peer_connection reference state handles
                                    },
                                    _ => {}
                                }
                            } else {
                                // 💬 FALLBACK: Not a call control event. Trigger your standard chat database layout refresh!
                                message_trigger.update(|n| *n += 1);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("WS Connection failed: {}", e);
                }
            }
        });
    });
}