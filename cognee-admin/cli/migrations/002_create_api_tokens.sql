CREATE TABLE IF NOT EXISTS cognee_admin.api_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token_hash  VARCHAR(64) UNIQUE NOT NULL,
    name        VARCHAR(100) NOT NULL,
    role        VARCHAR(20) NOT NULL DEFAULT 'user',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ,
    enabled     BOOLEAN NOT NULL DEFAULT true,
    last_used   TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_api_tokens_hash ON cognee_admin.api_tokens (token_hash);
