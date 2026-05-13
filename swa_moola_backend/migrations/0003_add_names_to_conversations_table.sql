ALTER TABLE conversations ADD COLUMN name TEXT DEFAULT 'new chart';

ALTER TABLE conversation_participants ENABLE ROW LEVEL SECURITY;

-- Users can only see participants in conversations they belong to
CREATE POLICY "participant_access" ON conversation_participants
FOR SELECT TO app_user
USING (
    conv_id IN (
        SELECT conv_id FROM conversation_participants 
        WHERE user_id::text = current_setting('app.current_user_id', true)
    )
);

ALTER TABLE conversations ENABLE ROW LEVEL SECURITY;

CREATE POLICY "conversation_access" ON conversations
FOR SELECT TO app_user
USING (
    conv_id IN (
        SELECT conv_id FROM conversation_participants 
        WHERE user_id::text = current_setting('app.current_user_id', true)
    )
);

ALTER TABLE messages ENABLE ROW LEVEL SECURITY;

CREATE POLICY "message_access" ON messages
FOR SELECT TO app_user
USING (
    conv_id IN (
        SELECT conv_id FROM conversation_participants 
        WHERE user_id::text = current_setting('app.current_user_id', true)
    )
);

-- Optional: Only allow users to insert messages into conversations they belong to
CREATE POLICY "message_insert" ON messages
FOR INSERT TO app_user
WITH CHECK (
    conv_id IN (
        SELECT conv_id FROM conversation_participants 
        WHERE user_id::text = current_setting('app.current_user_id', true)
    )
);
