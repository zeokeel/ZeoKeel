BEGIN;
CREATE TABLE IF NOT EXISTS schema_migrations (
    id          text PRIMARY KEY,
    checksum    text NOT NULL,
    applied_at  timestamptz NOT NULL DEFAULT now()
);
COMMIT;
