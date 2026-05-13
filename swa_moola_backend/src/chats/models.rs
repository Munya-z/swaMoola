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
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)] 
pub struct ConversationParticipant {
    pub conv_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Serialize, sqlx::FromRow)] 
pub struct Message {
    pub msg_id: Uuid,
    pub conv_id: Uuid,
    pub sender_id: Option<Uuid>,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug,Clone , Deserialize)] 
pub struct MessagePayload {
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub content: String,
}

#[derive(Debug,Clone , Deserialize)] 
pub struct AddParticipantPayload {
    pub conv_id: Uuid,
    pub participant_id: Uuid,
}

#[derive(Debug,Clone , Deserialize)]
pub struct CocoPayload{
    pub conv_id: Uuid
}


#[derive(Debug,Clone , Deserialize)]
pub struct GroupPayload{
    pub name: String,
    pub conv_id: Uuid,
    pub other_user_id: Uuid
}

#[derive(Debug,Clone , Deserialize)]
pub struct ConversationNamePayload{
    pub conv_id: Uuid
}