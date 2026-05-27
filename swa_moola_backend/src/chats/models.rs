use uuid::Uuid;
use sqlx::FromRow;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub name: String,
    pub conv_id: Uuid,
    pub is_group: bool,
    pub created_at: DateTime<Utc>,
    pub display_name: Option<String>,
    pub last_message_id : Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConversationParticipant {
    pub conv_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConversationResult {
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

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AttachmentMetadata {
    pub attachment_id: uuid::Uuid,
    pub file_name: String,
    pub file_size: i32,
    pub file_type: String,
}


#[derive(Serialize, Deserialize, sqlx::FromRow, Clone, Debug)] 
pub struct Message {
    pub msg_id: Uuid,
    pub conv_id: Uuid,
    pub sender_id: Option<Uuid>,
    pub content: Option<String>,
    pub created_at: DateTime<Utc>,
    pub attachments: sqlx::types::Json<Vec<AttachmentMetadata>>, 
}

#[derive(Debug,Clone , Deserialize)] 
pub struct MessagePayload {
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub content: Option<String>,
}

#[derive(Debug,Clone , Deserialize)] 
pub struct AddParticipantPayload {
    pub conv_id: Uuid,
    pub participant_id: Uuid,
}

#[derive(Debug,Clone , Deserialize)]
pub struct GroupPayload{
    pub name: String,
    pub conv_id: Uuid,
    pub other_user_id: Uuid
}

#[derive(Debug,Clone , Deserialize)]
pub struct ConversationIdPayload{
    pub conv_id: Uuid
}