ALTER TABLE messages ALTER COLUMN content DROP NOT NULL;

CREATE TABLE message_attachments (
    attachment_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    msg_id UUID NOT NULL REFERENCES messages(msg_id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    file_data BYTEA NOT NULL,       
    file_type TEXT NOT NULL,       
    file_size INT NOT NULL,        
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index the foreign key for fast retrieval when loading chat histories
CREATE INDEX idx_message_attachments_msg_id ON message_attachments(msg_id);