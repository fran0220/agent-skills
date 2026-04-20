-- Asset Library — searchable catalog of reusable assets
-- Uses trigger-based tsvector for full-text search (PostgreSQL requires IMMUTABLE for generated columns)

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
    search_vector tsvector,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Trigger function to maintain search_vector
CREATE OR REPLACE FUNCTION asset_library_search_update() RETURNS trigger AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('english', coalesce(NEW.name, '')), 'A') ||
        setweight(to_tsvector('english', coalesce(NEW.description, '')), 'B') ||
        setweight(to_tsvector('english', coalesce(array_to_string(NEW.tags, ' '), '')), 'A');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_asset_library_search
    BEFORE INSERT OR UPDATE ON asset_library
    FOR EACH ROW EXECUTE FUNCTION asset_library_search_update();

CREATE INDEX IF NOT EXISTS idx_asset_library_search ON asset_library USING GIN (search_vector);
CREATE INDEX IF NOT EXISTS idx_asset_library_asset_type ON asset_library(asset_type);
CREATE INDEX IF NOT EXISTS idx_asset_library_source ON asset_library(source);
CREATE INDEX IF NOT EXISTS idx_asset_library_created_at ON asset_library(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_asset_library_tags ON asset_library USING GIN (tags);
