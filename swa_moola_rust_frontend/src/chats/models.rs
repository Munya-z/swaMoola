use leptos::prelude::*; 
use serde::{Deserialize, Serialize}; 
use uuid::Uuid; 
use chrono::{DateTime, Utc}; 
use leptos_router::params::Params;


#[derive(Debug, Clone, Serialize, Deserialize)] 
pub struct Chat { 
    pub name: String, 
    pub conv_id: Uuid, 
    pub is_group: bool, 
    pub created_at: DateTime<Utc>, 
    pub display_name : String 
} 

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
    pub attachment_id: uuid::Uuid,
    pub file_name: String,
    pub file_size: i32,
    pub file_type: String,
}


#[derive(Debug, Clone,Serialize, Deserialize)] 
pub struct Message {
    pub msg_id: Uuid,
    pub conv_id: Uuid,
    pub sender_id: Option<Uuid>,
    pub content: Option<String>,
    pub created_at: DateTime<Utc>,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone,Serialize, Deserialize)] 
pub struct SearchResult {
    pub target_user_id: Uuid,
    pub name: String,
}


#[derive(Debug,Clone , Deserialize, Serialize)] 
pub struct MessagePayload {
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub content: String,
}

#[derive(Debug,Clone , Deserialize, Serialize)] 
pub struct ConversationPayload {
    pub conv_id: Uuid,
    pub is_group: bool,
    pub created_at: DateTime<Utc>,
    pub name: String,
    pub display_name: Option<String>,
    pub recipient_id: Option<Uuid>, 
    pub last_msg_content: Option<String>,
    pub last_msg_date: Option<DateTime<Utc>>,
    pub last_msg_sender: Option<Uuid>,
}

#[derive(Debug,Clone , Deserialize, Serialize)] 
pub struct SearchPayload {
    pub key: String,
}