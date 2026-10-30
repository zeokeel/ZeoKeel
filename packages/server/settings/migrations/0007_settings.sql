BEGIN;
CREATE TABLE IF NOT EXISTS system_settings (
    key text PRIMARY KEY, value jsonb NOT NULL, hot boolean NOT NULL DEFAULT false,
    updated_at timestamptz NOT NULL DEFAULT now(), updated_by uuid REFERENCES users(id) ON DELETE SET NULL
);
COMMIT;
