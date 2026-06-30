// use leptos::prelude::*;
// use wasm_bindgen::prelude::*;
// use wasm_bindgen::JsCast;
// use uuid::Uuid;
// use leptos::{serde_json, html};
// use crate::auth::models::AuthenticatedUser; 
// use web_sys::{MediaStream, RtcConfiguration, RtcPeerConnection};
// use leptos_router::params::Params;
// use leptos_router::hooks::use_params;

// #[derive(Params, PartialEq, Clone, Debug)]
// struct CallParams {
//     id: String, // Matches the :id segment in your router path configuration
// }


// #[component]
// pub fn WebRtcCall() -> impl IntoView {
//     let params = use_params::<CallParams>();

//     let user = window() 
//         .local_storage() 
//         .ok()
//         .flatten()
//         .and_then(|s| s.get_item("auth_user").ok().flatten()) 
//         .and_then(|json| serde_json::from_str::<AuthenticatedUser>(&json).ok());

//     let user_uuid_str = user.as_ref().and_then(|u| Uuid::parse_str(&u.uuid.to_string()).ok()); 
//     let my_name =user.as_ref().and_then(|u| u.name.as_deref()).unwrap_or("Unknown User").to_string();
    
//     let reciever = move || {
//         params.with(|p| {
//             p.as_ref()
//             .map(|p| p.id.clone())
//                 .unwrap_or_else(|_| "No ID found".to_string())
//         })
//     };

//     // UI targets for video rendering
//     let local_video_ref = NodeRef::<html::Video>::new();
//     let remote_video_ref = NodeRef::<html::Video>::new();

//     let local_audio_ref = NodeRef::<html::Audio>::new();
//     let remote_audio_ref = NodeRef::<html::Audio>::new();

//     let peer_conn_storage = StoredValue::<Option<RtcPeerConnection>>::new(None);
//     let local_stream_storage = StoredValue::<Option<MediaStream>>::new(None);

//     let (my_id, set_my_id) = signal(user_uuid_str);
//     let (target_peer_id, set_target_peer_id) = signal(reciever);
//     let (active_peer_name, set_active_peer_name) = signal(my_name);
    
//     // Call status tracker
//     let (call_status, set_call_status) = signal("Idle".to_string());

//     // Main WebRTC loop initialized on interaction
//     let start_call = move |_| {
//         set_call_status.set("Connecting...".to_string());

//         wasm_bindgen_futures::spawn_local(async move {
//             let window = web_sys::window().expect("No global window found");
//             let navigator = window.navigator();
//             let media_devices = navigator.media_devices().expect("No media devices API found");

//             // 1. Request Video/Audio permissions
//             let mut constraints = web_sys::MediaStreamConstraints::new();
//             constraints.set_video(&JsValue::from_bool(false));
//             constraints.set_audio(&JsValue::from_bool(true));

//             let stream_promise = media_devices.get_user_media_with_constraints(&constraints)
//                 .expect("Failed calling getUserMedia");
            
//             let local_stream_js = wasm_bindgen_futures::JsFuture::from(stream_promise).await
//                 .expect("User denied camera/microphone access");
            
//             let local_stream: MediaStream = local_stream_js.unchecked_into();

//             if let Some(video_el) = local_video_ref.get() {
//                 let video_el: web_sys::HtmlVideoElement = video_el; 
//                 video_el.set_src_object(Some(&local_stream));
//             }

//             if let Some(audio_el) = local_audio_ref.get() {
//                 let audio_el: web_sys::HtmlAudioElement = audio_el; 
//                 audio_el.set_src_object(Some(&local_stream));
//             }

//             // 3. Configure public STUN server configuration
//             let mut config = RtcConfiguration::new();
//             let ice_servers = js_sys::Array::new();
//             let server_entry = js_sys::Object::new();
//             js_sys::Reflect::set(&server_entry, &"urls".into(), &"stun:://google.com".into()).unwrap();
//             ice_servers.push(&server_entry);
//             config.set_ice_servers(&ice_servers);

//             // 4. Build the WebRTC interface peer
//             let peer_connection = RtcPeerConnection::new_with_configuration(&config)
//                 .expect("Failed creating RtcPeerConnection");

//             local_stream_storage.set_value(Some(local_stream.clone()));

//             // Inside start_call after creating peer_connection:
//             peer_conn_storage.set_value(Some(peer_connection.clone()));

//             // 5. Pipe tracks from our camera straight into the P2P connection pipe
//             for track in local_stream.get_tracks().iter() {
//                 let media_track = track.unchecked_into::<web_sys::MediaStreamTrack>();
//                 peer_connection.add_track(&media_track, &local_stream, &js_sys::Array::new());
//             }

//             // 6. Monitor network ICE Candidates to pass along to signaling server
//             let onicecandidate_cb = Closure::<dyn FnMut(web_sys::RtcPeerConnectionIceEvent)>::new(move |e: web_sys::RtcPeerConnectionIceEvent| {
//                 if let Some(candidate) = e.candidate() {
//                     let candidate_str = candidate.candidate();
//                     // ACTION REQUIRED: Send this string serialized to your signaling socket here
//                     leptos::logging::log!("Send ICE Candidate to peer: {}", candidate_str);
//                     let ice_payload = serde_json::json!({
//                         "event": "ice_candidate",
//                         "target_user_id": target_peer_id.get_untracked(),
//                         "candidate": candidate_str
//                     });
//                     let _ = ws_tx.send_with_str(&ice_payload.to_string());
//                 }
//             });
//             peer_connection.set_onicecandidate(Some(onicecandidate_cb.as_ref().unchecked_ref()));
//             onicecandidate_cb.forget(); // Keep the closure memory alive in WASM landscape

//             // 7. Render remote stream immediately as inbound media tracks surface
//             // let ontrack_cb = Closure::<dyn FnMut(web_sys::RtcTrackEvent)>::new(move |e: web_sys::RtcTrackEvent| {
//             //     if let Some(remote_video_el) = remote_video_ref.get() {
//             //         let remote_video_el: web_sys::HtmlVideoElement = remote_video_el;
                    
//             //         let streams = e.streams();
//             //         if streams.length() > 0 {
//             //             // Extract the raw JsValue item at index 0 safely
//             //             let stream_js_value = streams.get(0);
                        
//             //             // Cast the raw JS object over to the Web API MediaStream structure
//             //             let remote_stream: web_sys::MediaStream = stream_js_value.unchecked_into();
//             //             remote_video_el.set_src_object(Some(&remote_stream));
//             //         }
//             //     }
//             // });
//             let ontrack_cb = Closure::<dyn FnMut(web_sys::RtcTrackEvent)>::new(move |e: web_sys::RtcTrackEvent| {
//                 if let Some(remote_audio_el) = remote_audio_ref.get() {
//                     let remote_audio_el: web_sys::HtmlAudioElement = remote_audio_el;
//                     let streams = e.streams();
//                     if streams.length() > 0 {
//                         let stream_js_value = streams.get(0);
//                         let remote_stream: web_sys::MediaStream = stream_js_value.unchecked_into();
//                         remote_audio_el.set_src_object(Some(&remote_stream));
//                     }
//                 }
//             });
//             peer_connection.set_ontrack(Some(ontrack_cb.as_ref().unchecked_ref()));
//             ontrack_cb.forget();

//             // 8. Create Offer Handshake initialization
//             let offer = wasm_bindgen_futures::JsFuture::from(peer_connection.create_offer()).await
//                 .expect("Failed making local SDP description");
            
//             let sdp_init = web_sys::RtcSessionDescriptionInit::unchecked_from_js(offer);
            
//             let _ = wasm_bindgen_futures::JsFuture::from(peer_connection.set_local_description(&sdp_init)).await;

//             // ACTION REQUIRED: Extract local SDP and push to signaling server to start the call
//             // let sdp_string = sdp_init.sdp();
//             set_call_status.set("Awaiting Remote Answer...".to_string());

//             let sdp_string = sdp_init.sdp();
//             let call_payload = serde_json::json!({
//                 "event": "call_request",
//                 "target_user_id": target_peer_id.get_untracked(),
//                 "sdp_offer": sdp_string
//             });

//             // Assuming you have a WebSocket text transmitter instance in scope:
//             let _ = ws_tx.send_with_str(&call_payload.to_string());
//         });
//     };

//     let cancel_call = move |_| {
//         // 1. Turn off local hardware (stops the browser recording indicator light)
//         if let Some(stream) = local_stream_storage.get_value() {
//             for track in stream.get_tracks().iter() {
//                 let media_track = track.unchecked_into::<web_sys::MediaStreamTrack>();
//                 media_track.stop(); // Hardware off
//             }
//             local_stream_storage.set_value(None);
//         }

//         // 2. Tear down the WebRTC connection pipe
//         if let Some(peer_conn) = peer_conn_storage.get_value() {
//             peer_conn.close(); // Terminate peer-to-peer network connection
//             peer_conn_storage.set_value(None);
//         }

//         // 3. ACTION REQUIRED: Notify the signaling server to stop the recipient's phone from ringing
//         // websocket_tx.send(SignalingMessage::CancelCall { target: target_peer_id.get_untracked() });

//         // 4. Reset component visual view status back to idle
//         set_call_status.set("Idle (Call Cancelled)".to_string());
//     };

//     view! {
//         <div class="p-6 max-w-md mx-auto bg-white rounded-xl shadow-md space-y-4">
//             <div>
//                 <p class="text-xs text-gray-500">"Logged in as:"</p>
//                 <p class="font-bold text-blue-600">{move || active_peer_name.get()}</p>
//             </div>

//             <div class="flex flex-col items-center gap-4 p-6">
//                 <h2 class="text-xl font-bold">"WebRTC Call Client"</h2>

//                 <audio node_ref=local_audio_ref autoplay muted class="hidden"/>
//                 <audio node_ref=remote_audio_ref autoplay class="hidden"/>
                
//                 // <div class="flex gap-4">
//                 //     <div>
//                 //         <p class="text-center font-semibold">"Local Stream (You)"</p>
//                 //         <video node_ref=local_video_ref autoplay playsinline muted class="w-72 h-auto border-2 border-blue-500 rounded bg-black" />
//                 //     </div>
//                 //     <div>
//                 //         <p class="text-center font-semibold">"Remote Stream (Peer)"</p>
//                 //         <video node_ref=remote_video_ref autoplay playsinline class="w-72 h-auto border-2 border-green-500 rounded bg-black" />
//                 //     </div>
//                 // </div>

//                 {move || {
//                     if call_status.get() == "Idle" || call_status.get().contains("Cancelled") {
//                         view! {
//                             <button on:click=start_call class="w-full px-4 py-2 bg-green-600 text-white font-medium rounded hover:bg-green-700 transition">
//                                 "Start Voice Call"
//                             </button>
//                         }.into_any()
//                     } else {
//                         view! {
//                             <button on:click=cancel_call class="w-full px-4 py-2 bg-red-600 text-white font-medium rounded hover:bg-red-700 transition">
//                                 "Cancel / Hang Up"
//                             </button>
//                         }.into_any()
//                     }
//                 }}
           
//                 <button on:click=start_call class="px-4 py-2 bg-blue-600 text-white font-medium rounded hover:bg-blue-700">
//                     "Initialize Call"
//                 </button>
//             </div>

//             <div class="bg-gray-50 p-3 rounded-md text-sm">
//                 <p><strong>"Call Status: "</strong> {move || call_status.get()}</p>
//                 <p><strong>"Connected Peer: "</strong> {move || active_peer_name.get()}</p>
//             </div>
//         </div>
//     }
// }



use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use leptos_router::hooks::use_navigate;
use leptos::serde_json;
use crate::chats::ws_hooks::GlobalWebSocketSender;
use crate::chats::ws_hooks::IncomingSignalStream;
use web_sys::{MediaStream, RtcConfiguration, RtcPeerConnection, RtcSessionDescriptionInit, RtcIceCandidate, RtcIceCandidateInit};
use leptos_router::params::Params;
use leptos_router::hooks::use_params;
use crate::chats::ws_hooks::IncomingCallState;
use crate::chats::ws_hooks::CallState;
use serde::{Deserialize, Serialize};

// 1. Mirror your Backend Enum exactly for frontend JSON Serialization
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "data")]
pub enum SignalingMessage {
    Register { username: String, user_id: String },
    CallRequest { target_user_id: String, sender_name: String, sdp_offer: String },
    CallResponse { target_user_id: String, sdp_answer: String, accepted: bool },
    IceCandidate { target_user_id: String, candidate: String },
}

#[derive(Clone, Copy)]
pub struct ActiveCallSession(pub StoredValue<Option<RtcPeerConnection>>);

#[derive(Params, PartialEq, Clone, Debug)]
struct CallParams { id: String }

#[component]
pub fn WebRtcCall() -> impl IntoView {
    let params = use_params::<CallParams>();

    // Setup identification trackers
    let target_peer_id = Memo::new(move |_| params.with(|p| p.as_ref().map(|p| p.id.clone()).unwrap_or_default()));
    let (call_status, set_call_status) = signal("Idle".to_string());

    // Storage hooks to persist core handles across asynchronous background tasks
    let call_session_context = use_context::<ActiveCallSession>()
    .expect("ActiveCallSession context missing from scope");
    let peer_conn_storage = call_session_context.0;
    let local_stream_storage = StoredValue::<Option<MediaStream>>::new(None);

    let ws_context = use_context::<GlobalWebSocketSender>()
        .expect("GlobalWebSocketSender missing from scope. Is use_websocket_listener running?");
    
    let ws_sender = ws_context.0;

    // Define your WebRTC signaling channel to route packages over this inherited wire
    let ws_outbound = move |msg: SignalingMessage| {
        if let Ok(serialized_json) = serde_json::to_string(&msg) {
            leptos::logging::log!("📤 Forwarding call signal down the existing wire: {}", serialized_json);
            ws_sender.run(serialized_json); // Triggers your pre-existing WsMeta connection channel!
        }
    };
    let incoming_stream = use_context::<IncomingSignalStream>()
        .expect("IncomingSignalStream missing from scope.");
    let incoming_signal = incoming_stream.0; 


    // --- BACKGROUND BACKGROUND LISTENER ---
    // Instantiates a reactive event listener loop to catch incoming server signals
    Effect::new(move |_| {
        // MOCK RECEIVER: Map this hook directly into your frontend incoming WebSocket stream data channel
        let incoming_ws_message: Option<SignalingMessage> = incoming_signal.get();

        if let Some(msg) = incoming_ws_message {
            match msg {
                SignalingMessage::CallResponse { target_user_id, sdp_answer, accepted } => {
                    if accepted {
                        // 1. Update your UI Call Status State immediately
                        set_call_status.set("Connected".to_string());

                        // 2. Retrieve your stored caller peer connection instance
                        if let Some(peer_connection) = peer_conn_storage.get_value() {
                            wasm_bindgen_futures::spawn_local(async move {
                                // 3. Prepare the Remote Description object using the received Answer SDP
                                let mut answer_obj = web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Answer);
                                answer_obj.set_sdp(&sdp_answer);

                                // 4. Finalize the handshake
                                let result = wasm_bindgen_futures::JsFuture::from(
                                    peer_connection.set_remote_description(&answer_obj)
                                ).await;

                                match result {
                                    Ok(_) => leptos::logging::log!("✅ Handshake finalized! Audio path open."),
                                    Err(e) => leptos::logging::log!("❌ Failed to apply remote description: {:?}", e),
                                }
                            });
                        } else {
                            set_call_status.set("Call Rejected by peer".to_string());
                            leptos::logging::log!("⚠️ Received call answer, but no active PeerConnection instance was stored!");
                        }
                    } else {
                        // Handle call rejection state change
                        set_call_status.set("Call Declined".to_string());
                        peer_conn_storage.set_value(None);
                    }
                },
                SignalingMessage::IceCandidate { candidate, .. } => {
                    if let Some(peer_conn) = peer_conn_storage.get_value() {
                        let mut candidate_init = RtcIceCandidateInit::new(&candidate);
                        if let Ok(ice_cand) = RtcIceCandidate::new(&candidate_init) {
                            wasm_bindgen_futures::spawn_local(async move {
                                let _ = wasm_bindgen_futures::JsFuture::from(peer_conn.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&ice_cand))).await;
                            });
                        }
                    }
                },
                _ => {} // Handle incoming CallRequest on a separate call receiver modal setup
            }
        }
    });

    // --- OUTBOUND INITIALIZATION DIAL TRACK LOOP ---
    let start_call = move |_| {
        set_call_status.set("Dialing Peer...".to_string());
        let absolute_target_id = target_peer_id.get_untracked();

        if absolute_target_id.is_empty() || absolute_target_id == "No ID found" {
            leptos::logging::log!("⚠️ Dialing blocked: Target peer parameter ID is empty!");
            set_call_status.set("Error: Invalid Target ID".to_string());
            return;
        }

        let send_payload = ws_outbound;

        wasm_bindgen_futures::spawn_local(async move {
            let window = web_sys::window().expect("No window");
            let media_devices = window.navigator().media_devices().expect("No media hardware access");

            let mut constraints = web_sys::MediaStreamConstraints::new();
            constraints.set_video(&JsValue::from_bool(false));
            constraints.set_audio(&JsValue::from_bool(true));

            let local_stream: MediaStream = wasm_bindgen_futures::JsFuture::from(
                media_devices.get_user_media_with_constraints(&constraints).expect("Media Capture Fail")
            ).await.expect("Hardware Denied").unchecked_into();

            local_stream_storage.set_value(Some(local_stream.clone()));

            let mut ice_server = web_sys::RtcIceServer::new();

            // Define your targets strictly using an array slice structure
            let ice_urls = js_sys::Array::new();
            ice_urls.push(&"stun:google.com".into());

            // Safe, strongly-typed property setter mappings
            ice_server.set_urls(&ice_urls);
            log::info!("these are the ice url {:?}", &ice_urls);

            // 2. Put the server object into your main configurations structure
            let mut config = web_sys::RtcConfiguration::new();
            let ice_servers_list = js_sys::Array::new();
            ice_servers_list.push(&ice_server);

            config.set_ice_servers(&ice_servers_list);

            log::info!("this is the config before making the call {:?}", &config);

            let peer_connection = RtcPeerConnection::new_with_configuration(&config).expect("WebRTC Connection Error");

            let ontrack_cb = Closure::<dyn FnMut(web_sys::RtcTrackEvent)>::new(move |evt: web_sys::RtcTrackEvent| {
            leptos::logging::log!("🎵 Incoming media track received!");
            
            let streams = evt.streams();
            if streams.length() > 0 {
                // Extract the first media stream from the array
                let remote_stream = streams.get(0).unchecked_into::<web_sys::MediaStream>();
                
                // Find your HTML Audio Element in the DOM and play it
                let window = web_sys::window().expect("No window context");
                let document = window.document().expect("No document context");
                
                if let Some(audio_element) = document.get_element_by_id("remote-audio") {
                    let audio: web_sys::HtmlAudioElement = audio_element.unchecked_into();
                    audio.set_src_object(Some(&remote_stream));
                    
                    // Play the incoming stream (handle autoplay restrictions gracefully)
                    let _ = audio.play().expect("Failed to play audio element");
                } else {
                    leptos::logging::log!("⚠️ HTML Audio element with id 'remote-audio' not found!");
                }
            }
        });

        // Bind the callback to the peer connection
        peer_connection.set_ontrack(Some(ontrack_cb.as_ref().unchecked_ref()));

        // Memory safety: Prevent Rust from cleaning up the closure allocation prematurely
        ontrack_cb.forget();
        // --- END OF ONTRACK IMPLEMENTATION ---


            for track in local_stream.get_tracks().iter() {
                let media_track = track.unchecked_into::<web_sys::MediaStreamTrack>();
                peer_connection.add_track(&media_track, &local_stream, &js_sys::Array::new() );
            }

            let candidate_target = absolute_target_id.clone();
            let send_candidate = send_payload;

            let onicecandidate_cb = Closure::<dyn FnMut(web_sys::RtcPeerConnectionIceEvent)>::new(move |e: web_sys::RtcPeerConnectionIceEvent| {
                if let Some(candidate) = e.candidate() {
                    send_candidate(SignalingMessage::IceCandidate { 
                        target_user_id: candidate_target.clone(), 
                        candidate: candidate.candidate() 
                    });
                }
            });
            peer_connection.set_onicecandidate(Some(onicecandidate_cb.as_ref().unchecked_ref()));
            onicecandidate_cb.forget();

            // Fire and serialize the outbound Connection Call Invitation
            let offer = wasm_bindgen_futures::JsFuture::from(peer_connection.create_offer()).await.expect("SDP Build Fail");
            let sdp_init = RtcSessionDescriptionInit::unchecked_from_js(offer);
            let _ = wasm_bindgen_futures::JsFuture::from(peer_connection.set_local_description(&sdp_init)).await;

            let local_desc = peer_connection.local_description()
                .expect("Local description should be populated after calling set_local_description");

            let sdp_string = local_desc.sdp();
            // Submit the finalized SDP Offer over the switchboard wire
            ws_outbound(SignalingMessage::CallRequest {
                target_user_id: absolute_target_id,
                sender_name: "Me".to_string(),
                sdp_offer: sdp_string,
            });

            peer_conn_storage.set_value(Some(peer_connection));
            
            set_call_status.set("Ringing...".to_string());
        });
    };

    let cancel_call = move |_| {
        // 1. Turn off local hardware (stops the browser recording indicator light)
        if let Some(stream) = local_stream_storage.get_value() {
            for track in stream.get_tracks().iter() {
                let media_track = track.unchecked_into::<web_sys::MediaStreamTrack>();
                media_track.stop(); // Hardware off
            }
            local_stream_storage.set_value(None);
        }

        // 2. Tear down the WebRTC connection pipe
        if let Some(peer_conn) = peer_conn_storage.get_value() {
            peer_conn.close(); // Terminate peer-to-peer network connection
            peer_conn_storage.set_value(None);
        }

        // 3. ACTION REQUIRED: Notify the signaling server to stop the recipient's phone from ringing
        // websocket_tx.send(SignalingMessage::CancelCall { target: target_peer_id.get_untracked() });

        // 4. Reset component visual view status back to idle
        set_call_status.set("Idle (Call Cancelled)".to_string());
    };


    view! {
        <div class="p-6 max-w-md mx-auto bg-white rounded-xl shadow-md flex flex-col items-center gap-4">
            <h2 class="text-xl font-bold">"Voice Connection Channel"</h2>
            <p class="text-sm font-medium text-gray-500">"Target Peer ID: " {move || target_peer_id.get()}</p>
            <p class="text-xs bg-blue-50 px-2 py-1 text-blue-700 rounded">"Status: " {move || call_status.get()}</p>
            // <audio id="remote-audio" autoplay=true controls=false></audio>
            
            {move || {
                if call_status.get() == "Idle" || call_status.get().contains("Cancelled") {
                    view! {
                        <button on:click=start_call class="w-full px-4 py-2 bg-green-600 text-white font-medium rounded hover:bg-green-700 transition">
                            "Start Voice Call"
                        </button>
                    }.into_any()
                } else {
                    view! {
                        <button on:click=cancel_call class="w-full px-4 py-2 bg-red-600 text-white font-medium rounded hover:bg-red-700 transition">
                            "Cancel / Hang Up"
                        </button>
                    }.into_any()
                }
            }}
        </div>
    }
}


#[component]
pub fn IncomingCallModal() -> impl IntoView {
    let call_state_context = use_context::<RwSignal<Option<CallState>>>()
        .expect("Missing active_incoming_call context");
        
    let ws_context = use_context::<GlobalWebSocketSender>()
        .expect("GlobalWebSocketSender missing");
    let ws_sender = ws_context.0;

    let call_session_context = use_context::<ActiveCallSession>()
        .expect("ActiveCallSession context missing from scope");
    let peer_conn_storage = call_session_context.0;

    let incoming_stream = use_context::<IncomingSignalStream>()
        .expect("IncomingSignalStream missing from scope.");
    let incoming_signal = incoming_stream.0;

    Effect::new(move |_| {
        if let Some(SignalingMessage::IceCandidate { candidate, .. }) = incoming_signal.get() {
            if let Some(peer_conn) = peer_conn_storage.get_value() {
                let candidate_init = RtcIceCandidateInit::new(&candidate);
                if let Ok(ice_cand) = RtcIceCandidate::new(&candidate_init) {
                    wasm_bindgen_futures::spawn_local(async move {
                        let _ = wasm_bindgen_futures::JsFuture::from(
                            peer_conn.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&ice_cand))
                        ).await;
                    });
                }
            }
        }
    });

    let ws_sender_for_reject = ws_sender.clone();
    let call_state_context_for_reject = call_state_context.clone();

    // Reject Call Routine
    let reject_call = move |_evt: leptos::ev::MouseEvent| {
        if let Some(CallState::Incoming(call)) = call_state_context_for_reject.get_untracked() {
            let reject_msg = SignalingMessage::CallResponse {
                target_user_id: call.target_user_id, // Target the caller profile back
                sdp_answer: String::new(),
                accepted: false,
            };
            if let Ok(json) = serde_json::to_string(&reject_msg) {
                ws_sender_for_reject.run(json);
            }
        }
        call_state_context_for_reject.set(None); // Close modal visual
    };

    let ws_sender_for_accept = ws_sender.clone();
    let call_state_context_for_accept = call_state_context.clone();
    let peer_conn_storage_for_accept = peer_conn_storage.clone();

    // Accept Call Routine
    let accept_call = move |_| {
        if let Some(CallState::Incoming(call)) = call_state_context_for_accept.get_untracked() {
            let ws_sender_clone = ws_sender_for_accept.clone();
            let ws_sender_inner = ws_sender_for_accept.clone();
            let peer_conn_storage_inner = peer_conn_storage_for_accept.clone();
            let call_state_inner = call_state_context_for_accept.clone();
            
            let sdp_offer_str = call.sdp_offer.clone(); 
            let target_user_id = call.target_user_id.clone();
            let sender_name = call.sender_name.clone(); 

            wasm_bindgen_futures::spawn_local(async move {
                let window = web_sys::window().expect("No window context");
                let media_devices = window.navigator().media_devices().expect("No media hardware access");

                // 1. Capture local audio track
                let mut constraints = web_sys::MediaStreamConstraints::new();
                constraints.set_video(&wasm_bindgen::JsValue::from_bool(false));
                constraints.set_audio(&wasm_bindgen::JsValue::from_bool(true));

                let media_js_future = wasm_bindgen_futures::JsFuture::from(
                    media_devices.get_user_media_with_constraints(&constraints).expect("Media API broken")
                ).await;

                let local_stream: web_sys::MediaStream = match media_js_future {
                    Ok(stream_obj) => stream_obj.unchecked_into(),
                    Err(err) => {
                        log::error!("User blocked mic or no hardware detected: {:?}", err);
                        
                        // OPTIONAL: Alert the user or fallback the UI state gracefully
                        call_state_inner.set(None); 
                        return; // Terminate execution loop safely without crashing Wasm engine
                    }
                };
                // 2. Configure ICE Servers (must match the caller's configuration)
                let mut ice_server = web_sys::RtcIceServer::new();
                let ice_urls = js_sys::Array::new();
                ice_urls.push(&"stun:google.com".into());
                ice_server.set_urls(&ice_urls);

                let mut config = web_sys::RtcConfiguration::new();
                let ice_servers_list = js_sys::Array::new();
                ice_servers_list.push(&ice_server);
                config.set_ice_servers(&ice_servers_list);

                // 3. Create Receiver's Peer Connection
                let peer_connection = web_sys::RtcPeerConnection::new_with_configuration(&config)
                    .expect("WebRTC Connection Error");

                // 4. Attach incoming track listener (Plays the caller's voice)
                let ontrack_cb = Closure::<dyn FnMut(web_sys::RtcTrackEvent)>::new(move |evt: web_sys::RtcTrackEvent| {
                    let streams = evt.streams();
                    if streams.length() > 0 {
                        let remote_stream = streams.get(0).unchecked_into::<web_sys::MediaStream>();
                        let win = web_sys::window().expect("No window context");
                        let doc = win.document().expect("No document context");
                        
                        if let Some(audio_element) = doc.get_element_by_id("remote-audio") {
                            let audio: web_sys::HtmlAudioElement = audio_element.unchecked_into();
                            audio.set_src_object(Some(&remote_stream));
                            let _ = audio.play().expect("Failed to play audio element");
                        }
                    }
                });
                peer_connection.set_ontrack(Some(ontrack_cb.as_ref().unchecked_ref()));
                ontrack_cb.forget();

                // 5. Attach Local Tracks to outbound stream
                for track in local_stream.get_tracks().iter() {
                    let media_track = track.unchecked_into::<web_sys::MediaStreamTrack>();
                    peer_connection.add_track(&media_track, &local_stream, &js_sys::Array::new());
                }

                // 6. Hook up ICE Candidate signaling
                let send_candidate_ws = ws_sender_clone.clone();
                let candidate_target = target_user_id.clone();
                let onicecandidate_cb = Closure::<dyn FnMut(web_sys::RtcPeerConnectionIceEvent)>::new(move |e: web_sys::RtcPeerConnectionIceEvent| {
                    if let Some(candidate) = e.candidate() {
                        let ice_msg = SignalingMessage::IceCandidate { 
                            target_user_id: candidate_target.clone(), 
                            candidate: candidate.candidate() 
                        };
                        if let Ok(json) = serde_json::to_string(&ice_msg) {
                            send_candidate_ws.run(json);
                        }
                    }
                });
                peer_connection.set_onicecandidate(Some(onicecandidate_cb.as_ref().unchecked_ref()));
                onicecandidate_cb.forget();

                // 7. Set Remote Offer (Process Caller Description)
                let mut offer_obj = web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Offer);
                offer_obj.set_sdp(&sdp_offer_str);
                let _ = wasm_bindgen_futures::JsFuture::from(peer_connection.set_remote_description(&offer_obj)).await;

                // 8. Create and Set Local Answer
                let answer = wasm_bindgen_futures::JsFuture::from(peer_connection.create_answer()).await.expect("Answer Build Fail");
                let sdp_answer_init = web_sys::RtcSessionDescriptionInit::unchecked_from_js(answer);
                let _ = wasm_bindgen_futures::JsFuture::from(peer_connection.set_local_description(&sdp_answer_init)).await;

                // 9. Send Answer back to Caller
                let local_desc = peer_connection.local_description()
                    .expect("Local description missing after assignment");
                
                let answer_msg = SignalingMessage::CallResponse {
                    target_user_id: target_user_id.clone(),
                    sdp_answer: local_desc.sdp(),
                    accepted: true,
                };

                if let Ok(json) = serde_json::to_string(&answer_msg) {
                    ws_sender_clone.run(json);
                }

                peer_conn_storage_inner.set_value(Some(peer_connection));

                call_state_inner.set(Some(CallState::Connected {
                    target_user_id,
                    sender_name, // Extracted from context at the top
                }));
            });
        }
       
    };

    view! {
        // Show only if there is an active incoming network request
        <audio id="remote-audio" autoplay=true  class="hidden"></audio>

        {move || match call_state_context.get() {  
            Some(CallState::Incoming(call)) => {
                view!{
                    <p>"The call state is incomming"</p>
                    <p class="text-slate-400 text-sm">"From: " {call.sender_name}</p>
                    <div class="flex gap-3 w-full mt-2">
                        <button on:click=reject_call.clone() class="flex-1 py-2.5 bg-red-100 text-red-700 font-semibold rounded-xl hover:bg-red-200 transition-colors">
                            "Decline"
                        </button>
                        <button on:click=accept_call.clone() class="flex-1 py-2.5 bg-green-600 text-white font-semibold rounded-xl hover:bg-green-700 shadow-lg shadow-green-600/20 transition-colors">
                            "Answer"
                        </button>
                    </div>
                }.into_any()
            }
            Some(CallState::Connected{ target_user_id: _, sender_name }) => {
                view!{
                    <p>"The call state is connected  to " {sender_name}</p>
                    <button on:click=reject_call.clone() class="flex-1 py-2.5 bg-red-100 text-red-700 font-semibold rounded-xl hover:bg-red-200 transition-colors">
                        "Decline"
                    </button>
                }.into_any()
            }
            Some(CallState::None) => {view!{}.into_any()}
            None => {
                view!{ <p>"The call state in None"</p>}.into_any() 
            }
        }}
    }
}



// <Show when=move || call_state_context.get().is_some() fallback=|| ()>
//     <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 animate-fade-in">
//         <div class="bg-white p-6 rounded-2xl shadow-2xl max-w-sm w-full mx-4 flex flex-col items-center gap-4 border border-gray-100 text-center">
//             <div class="w-16 h-16 bg-blue-100 rounded-full flex items-center justify-center text-blue-600 text-2xl animate-bounce">
//                 "☎️"
//             </div>
//             <div>
//                 <h3 class="text-lg font-bold text-gray-900">"Incoming Audio Call"</h3>
//                 <p class="text-sm text-gray-500 mt-1">
//                     {move || call_state_context.get().map(|c| c.sender_name).unwrap_or_default()} " is calling you..."
//                 </p>
//             </div>
//             <div class="flex gap-3 w-full mt-2">
//                 <button on:click=reject_call class="flex-1 py-2.5 bg-red-100 text-red-700 font-semibold rounded-xl hover:bg-red-200 transition-colors">
//                     "Decline"
//                 </button>
//                 <button on:click=accept_call class="flex-1 py-2.5 bg-green-600 text-white font-semibold rounded-xl hover:bg-green-700 shadow-lg shadow-green-600/20 transition-colors">
//                     "Answer"
//                 </button>
//             </div>
//         </div>
//     </div>
// </Show>