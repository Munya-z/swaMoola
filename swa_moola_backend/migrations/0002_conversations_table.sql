CREATE TABLE conversations (
    conv_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    is_group  BOOLEAN DEfAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE conversation_participants (
    conv_id UUID REFERENCES conversations(conv_id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (conv_id, user_id)
);

CREATE TABLE messages (
    msg_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conv_id UUID REFERENCES conversations(conv_id) ON DELETE CASCADE, 
    sender_id UUID REFERENCES users(id) ON DELETE SET NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

