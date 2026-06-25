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

#[derive(Debug, Clone, Serialize, Deserialize)] 
pub struct UserPublicKeys {
    pub x25519: String,
    pub mlkem: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConversationResult {
    pub conv_id: Uuid,
    pub is_group: bool,
    pub created_at: DateTime<Utc>,
    pub name: String,
    pub display_name: Option<String>,
    pub recipient_id: Option<Uuid>, 
    pub recipient_keys: Vec<UserPublicKeys>,
    pub last_message_id : Option<Uuid>,
}


#[derive(Serialize, Deserialize, sqlx::FromRow, Clone, Debug)] 
pub struct Message {
    pub msg_id: Uuid,
    pub conv_id: Uuid,
    pub ciphertext: String,
    pub nonce: String,
    pub envelopes: sqlx::types::Json<Vec<DbEnvelope>>, 
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Clone, Debug)] 
pub struct MessageReturn {
    pub msg_id: Uuid,
    pub conv_id: Uuid,
    pub ciphertext: String,
    pub nonce: String,
    pub envelopes: sqlx::types::Json<Vec<DbEnvelope>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbEnvelope {
    pub ephemeral_x25519: String,
    pub pq_ciphertext: String,
    pub encrypted_master_key: String, // Forces SQLx to map this column field
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedMessagePayload {
    pub ciphertext: String,
    pub nonce: String,
    pub envelopes: Vec<DbEnvelope>,
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
    pub other_user_ids: Vec<Uuid>
}

#[derive(Debug,Clone , Deserialize)]
pub struct ConversationIdPayload{
    pub conv_id: Uuid
}