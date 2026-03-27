-- Admin panel support: user approval, API key policy, and job ownership

ALTER TABLE users ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE users ADD COLUMN IF NOT EXISTS approved_at TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN IF NOT EXISTS approved_by TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS rejected_at TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN IF NOT EXISTS rejected_by TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS rejected_reason TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS api_key_expires_at TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN IF NOT EXISTS api_key_quota BIGINT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS api_key_quota_used BIGINT NOT NULL DEFAULT 0;

ALTER TABLE jobs ADD COLUMN IF NOT EXISTS user_id TEXT;

UPDATE users
SET status = 'active'
WHERE status IS NULL OR trim(status) = '';

UPDATE users
SET api_key_quota_used = 0
WHERE api_key_quota_used IS NULL;

CREATE INDEX IF NOT EXISTS idx_users_status ON users(status);
CREATE INDEX IF NOT EXISTS idx_jobs_user_id ON jobs(user_id);
