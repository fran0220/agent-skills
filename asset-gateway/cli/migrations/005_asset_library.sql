-- Asset Library — searchable catalog of reusable assets
-- Supports full-text search on name + description + tags

CREATE TABLE IF NOT EXISTS asset_library (
    id TEXT PRIMARY KEY NOT NULL,
    asset_type TEXT NOT NULL,           -- audio, music, image, video, model3d, sprite, etc.
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    tags TEXT[] NOT NULL DEFAULT '{}',
    file_path TEXT NOT NULL,            -- relative to uploads/ dir
    file_url TEXT NOT NULL,             -- full public URL
    file_size BIGINT NOT NULL DEFAULT 0,
    duration_seconds DOUBLE PRECISION,  -- for audio/music/video
    source TEXT NOT NULL DEFAULT 'manual', -- manual, generated, imported
    source_job_id TEXT,                 -- references jobs(id) if source=generated
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Full-text search index on name + description + tags
ALTER TABLE asset_library ADD COLUMN IF NOT EXISTS search_vector tsvector
    GENERATED ALWAYS AS (
        setweight(to_tsvector('english', coalesce(name, '')), 'A') ||
        setweight(to_tsvector('english', coalesce(description, '')), 'B') ||
        setweight(to_tsvector('english', coalesce(array_to_string(tags, ' '), '')), 'A')
    ) STORED;

CREATE INDEX IF NOT EXISTS idx_asset_library_search ON asset_library USING GIN (search_vector);
CREATE INDEX IF NOT EXISTS idx_asset_library_asset_type ON asset_library(asset_type);
CREATE INDEX IF NOT EXISTS idx_asset_library_source ON asset_library(source);
CREATE INDEX IF NOT EXISTS idx_asset_library_created_at ON asset_library(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_asset_library_tags ON asset_library USING GIN (tags);
