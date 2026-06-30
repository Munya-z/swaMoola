use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::mpsc::UnboundedSender;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "data")]
pub enum SignalingMessage {
    // Fired when a user connects to declare their identity
    Register { 
        username: String, 
        user_id: String 
    },
    // Initiating a call to a specific destination target
    CallRequest { 
        target_user_id: String, 
        sender_name: String, 
        sdp_offer: String 
    },
    // Accepting an incoming call request
    CallResponse { 
        target_user_id: String, 
        sdp_answer: String, 
        accepted: bool 
    },
    // Passing network route coordinates to a targeted peer
    IceCandidate { 
        target_user_id: String, 
        candidate: String 
    },
}

pub struct Switchboard {
    // Map tracking active connections: Key = User ID, Value = Active WebSocket channel transmitter
    // pub connections: HashMap<String, UnboundedSender<SignalingMessage>>,
    pub connections: std::collections::HashMap<String, tokio::sync::mpsc::UnboundedSender<axum::extract::ws::Message>>,
}

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub switchboard: Arc<RwLock<Switchboard>>,
}

