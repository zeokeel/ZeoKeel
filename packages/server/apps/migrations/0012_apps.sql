BEGIN;
ALTER TABLE content_releases ADD COLUMN IF NOT EXISTS wasm bytea;
ALTER TABLE content_releases ADD COLUMN IF NOT EXISTS wasm_sha256 text NOT NULL DEFAULT '';
ALTER TABLE content_releases ADD COLUMN IF NOT EXISTS ui_manifest jsonb NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE content_releases ADD COLUMN IF NOT EXISTS ui_archive bytea;
ALTER TABLE content_releases ADD COLUMN IF NOT EXISTS ui_sha256 text NOT NULL DEFAULT '';
ALTER TABLE content_releases ADD COLUMN IF NOT EXISTS db_schema_version integer NOT NULL DEFAULT 0;
ALTER TABLE content_releases ADD COLUMN IF NOT EXISTS db_migrations jsonb NOT NULL DEFAULT '[]'::jsonb;
CREATE TABLE IF NOT EXISTS app_databases (
    app_id uuid PRIMARY KEY REFERENCES contents(id) ON DELETE CASCADE,
    schema_version integer NOT NULL DEFAULT 0,
    size_bytes bigint NOT NULL DEFAULT 0 CHECK (size_bytes >= 0),
    quota_bytes bigint NOT NULL DEFAULT 104857600 CHECK (quota_bytes > 0),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS app_calls (
    id bigserial PRIMARY KEY,
    app_id uuid NOT NULL REFERENCES contents(id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    request_id text NOT NULL,
    status integer NOT NULL,
    duration_ms bigint NOT NULL,
    fuel bigint NOT NULL,
    at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS app_calls_at ON app_calls(at);
CREATE TABLE IF NOT EXISTS app_runtime_limits (
    app_id uuid PRIMARY KEY REFERENCES contents(id) ON DELETE CASCADE,
    max_concurrency integer NOT NULL DEFAULT 4 CHECK (max_concurrency > 0),
    max_memory_bytes bigint NOT NULL DEFAULT 67108864 CHECK (max_memory_bytes > 0),
    fuel bigint NOT NULL DEFAULT 10000000 CHECK (fuel > 0),
    updated_at timestamptz NOT NULL DEFAULT now()
);
COMMIT;
