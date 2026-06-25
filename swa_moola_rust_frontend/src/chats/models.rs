use leptos::prelude::*; 
use serde::{Deserialize, Serialize}; 
use uuid::Uuid; 
use chrono::{DateTime, Utc}; 
use leptos_router::params::Params;
use base64::{Engine as _, engine::general_purpose::STANDARD};

#[derive(Params, PartialEq, Clone, Debug)]
pub struct ChatParams {
    pub id: String, 
}

#[derive(Serialize)]
pub struct ChatPayload { 
    pub conv_id : Uuid, 
}

#[derive(Serialize, Deserialize, Clone, Debug,  PartialEq)]
pub struct Attachment {
    pub file_name: String,
    pub file_size: i32,
    pub file_type: String,
    pub storage_url: String,
    pub file_key: String,
    pub nonce_base: String,
}

#[derive(Serialize, Deserialize, Clone, Debug,  PartialEq)]
pub struct AttachmentMeta {
    pub file_name: String,
    pub file_size: i32,
    pub file_type: String,
    pub storage_url: String,
    pub file_key: String,
    pub nonce_base: String,
}

// #[derive(Debug, Clone,Serialize, Deserialize)] 
// pub struct Message {
//     pub msg_id: Uuid,
//     pub conv_id: Uuid,
//     pub sender_id: Uuid,
//     pub content: Option<String>,
//     pub created_at: DateTime<Utc>,
//     pub attachments: Vec<Attachment>,
// }

#[derive(Debug, Clone,Serialize, Deserialize)] 
pub struct SearchResult {
    pub target_user_id: Uuid,
    pub name: String,
    pub recipient_keys: Vec<UserPublicKeyStrings>
}


pub enum ChatTarget {
    NewChat { recipient_id: Uuid },
    ExistingChat { conv_id: Uuid },
}

#[derive(Debug,Clone , Deserialize, Serialize)] 
pub struct SearchPayload {
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecretInnerPayload {
    pub sender_id: Uuid,
    pub sender_name: String,
    pub timestamp_ms: DateTime<Utc>, 
    pub text_message: String,
    pub attachments: Vec<AttachmentMeta>, 
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutboundMessagePayload {
    pub conv_id: Uuid,
    pub ciphertext: String,       
    pub nonce: String,            
    pub envelopes: Vec<Envelope>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InboundMessagePayload {
    pub msg_id: Uuid,
    pub conv_id: Uuid,
    pub ciphertext: String,
    pub nonce: String,
    pub envelopes: Vec<Envelope>, 
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Envelope {
    pub ephemeral_x25519: String, 
    pub pq_ciphertext: String,
    pub encrypted_master_key: String,    
}

#[derive(Debug,Clone , Deserialize, Serialize)] 
pub struct ConversationPayload {
    pub conv_id: Uuid,
    pub is_group: bool,
    pub created_at: DateTime<Utc>,
    pub name: String,
    pub display_name: Option<String>,
    pub recipient_id: Option<Uuid>, 
    pub recipient_keys: Vec<UserPublicKeys>,
    pub last_msg_id: Option<Uuid>,
    
}

#[derive(Debug,Clone , Deserialize, Serialize)] 
pub struct ConversationPayloadWithStringKeys {
    pub conv_id: Uuid,
    pub is_group: bool,
    pub created_at: DateTime<Utc>,
    pub name: String,
    pub display_name: Option<String>,
    pub recipient_id: Option<Uuid>, 
    pub recipient_keys: Vec<UserPublicKeyStrings>,
    pub last_msg_id: Option<Uuid>,
    
}

#[derive(Debug,Clone , Deserialize, Serialize)] 
pub struct ConversationListPayload {
    pub conv_id: Uuid,
    pub is_group: bool,
    pub created_at: DateTime<Utc>,
    pub name: String,
    pub display_name: Option<String>,
    pub recipient_id: Option<Uuid>, 
    pub last_msg_id: Option<Uuid>,
    
}


#[derive(Debug, Clone,  Deserialize, Serialize)]
pub struct UserPublicKeyStrings {
    pub x25519: String,
    pub mlkem: String,
}

#[derive(Debug, Clone,  Deserialize, Serialize)]
pub struct UserPublicKeys {
    pub x25519: [u8; 32],
    #[serde(with = "serde_bytes")]
    pub mlkem: [u8; 1184],
}

impl UserPublicKeys {
    pub fn new(x_public: Option<&String>, pq_public: Option<&String>) -> Self {
   
        let x25519 = x_public
            .and_then(|s| STANDARD.decode(s).ok())
            .and_then(|bytes| bytes.try_into().ok())
            .unwrap_or([0u8; 32]);

        let mlkem = pq_public
            .and_then(|s| STANDARD.decode(s).ok())
            .and_then(|bytes| bytes.try_into().ok())
            .unwrap_or([0u8; 1184]);

        Self { x25519, mlkem }
    }
}