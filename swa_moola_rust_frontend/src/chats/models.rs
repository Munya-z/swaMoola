use leptos::prelude::*; 
use leptos::serde_json; 
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

#[derive(Debug, Clone,Serialize, Deserialize)] 
pub struct Message {
    pub msg_id: Uuid,
    pub conv_id: Uuid,
    pub sender_id: Option<Uuid>,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

