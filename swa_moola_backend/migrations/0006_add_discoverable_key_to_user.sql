ALTER TABLE users ADD COLUMN discoverable_key VARCHAR(12) UNIQUE DEFAULT NULL;

CREATE INDEX idx_users_discoverable_key ON users(discoverable_key);