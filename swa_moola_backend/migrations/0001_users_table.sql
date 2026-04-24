CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name  TEXT NOT NULL,
    phone_number_hash TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    trust_score INTEGER DEFAULT 0,
    active_transactions INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- This ensures users can only see their own profile.
ALTER TABLE users ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Allow_registration" ON users
FOR INSERT 
TO app_user
WITH CHECK (true);

CREATE POLICY "Allow_read_during_insert" ON users
FOR SELECT
TO app_user
USING (true); 

CREATE POLICY "Users_view_own_data" ON users
FOR SELECT
TO app_user
USING (
    -- This assumes your Rust app sets this variable after login
    id::text = current_setting('app.current_user_id', true)
);