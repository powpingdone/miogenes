CREATE TABLE IF NOT EXISTS auth_keys (
    id BLOB NOT NULL CHECK (length(id) == 16),
    expiry INTEGER NOT NULL,
    secret BLOB NOT NULL,
    FOREIGN KEY(id) REFERENCES user(id)
) STRICT;
CREATE INDEX IF NOT EXISTS auth_keys_id ON auth_keys (id);