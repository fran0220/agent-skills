CREATE TABLE IF NOT EXISTS image_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    model TEXT NOT NULL,
    state JSONB NOT NULL DEFAULT '{}',
    turn_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ DEFAULT now() + INTERVAL '24 hours'
);

CREATE INDEX IF NOT EXISTS idx_image_sessions_user_id ON image_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_image_sessions_expires_at ON image_sessions(expires_at);
