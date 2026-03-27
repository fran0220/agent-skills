-- cognee-admin management schema (lives alongside cognee's public schema)
CREATE SCHEMA IF NOT EXISTS cognee_admin;

CREATE TABLE IF NOT EXISTS cognee_admin.request_logs (
    id              BIGSERIAL PRIMARY KEY,
    timestamp       TIMESTAMPTZ NOT NULL DEFAULT now(),
    method          VARCHAR(10) NOT NULL,
    endpoint        TEXT NOT NULL,
    status_code     INT,
    latency_ms      INT,
    request_body    TEXT,
    response_preview TEXT,
    source          VARCHAR(10) NOT NULL DEFAULT 'cli'
);

CREATE INDEX IF NOT EXISTS idx_request_logs_timestamp
    ON cognee_admin.request_logs (timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_request_logs_endpoint
    ON cognee_admin.request_logs (endpoint);

CREATE TABLE IF NOT EXISTS cognee_admin.health_snapshots (
    id              BIGSERIAL PRIMARY KEY,
    timestamp       TIMESTAMPTZ NOT NULL DEFAULT now(),
    status          VARCHAR(20) NOT NULL,
    components      JSONB NOT NULL,
    uptime          INT
);

CREATE INDEX IF NOT EXISTS idx_health_snapshots_timestamp
    ON cognee_admin.health_snapshots (timestamp DESC);
