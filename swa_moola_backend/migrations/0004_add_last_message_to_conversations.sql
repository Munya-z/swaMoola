ALTER TABLE conversations 
ADD COLUMN last_message_id UUID REFERENCES messages(msg_id) ON DELETE SET NULL