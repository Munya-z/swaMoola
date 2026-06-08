CREATE TABLE conversations (
    conv_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    is_group  BOOLEAN DEfAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    name TEXT DEFAULT 'new chart'
);

CREATE TABLE conversation_participants (
    conv_id UUID REFERENCES conversations(conv_id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (conv_id, user_id)
);

CREATE TABLE messages (
    msg_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conv_id UUID REFERENCES conversations(conv_id) ON DELETE CASCADE, 
    created_at TIMESTAMPTZ DEFAULT NOW()
);

ALTER TABLE messages ADD COLUMN ciphertext TEXT NOT NULL;
ALTER TABLE messages ADD COLUMN nonce TEXT NOT NULL;
ALTER TABLE messages ADD COLUMN s_envelope JSONB NOT NULL;
ALTER TABLE messages ADD COLUMN r_envelope JSONB NOT NULL;


ALTER TABLE conversations ADD COLUMN last_message_id UUID REFERENCES messages(msg_id) ON DELETE SET NULL;

ALTER TABLE conversation_participants ENABLE ROW LEVEL SECURITY;


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


CREATE POLICY "participant_access" ON conversation_participants
FOR SELECT TO app_user
USING (
    user_id::text = current_setting('app.current_user_id', true)
    OR 
    conv_id IN (
        SELECT cp.conv_id 
        FROM conversation_participants cp 
        WHERE cp.user_id::text = current_setting('app.current_user_id', true)
    )
);

CREATE POLICY "message_access" ON messages
FOR SELECT TO app_user
USING (
    conv_id IN (
        SELECT cp.conv_id 
        FROM conversation_participants cp
        WHERE cp.user_id::text = current_setting('app.current_user_id', true)
    )
);

CREATE POLICY "message_insert" ON messages
FOR INSERT TO app_user
WITH CHECK (
    conv_id IN (
        SELECT cp.conv_id 
        FROM conversation_participants cp 
        WHERE cp.user_id::text = current_setting('app.current_user_id', true)
    )
);