BEGIN;
CREATE TABLE IF NOT EXISTS app_runtime_logs (
    id bigserial PRIMARY KEY,
    app_id uuid NOT NULL REFERENCES contents(id) ON DELETE CASCADE,
    level text NOT NULL DEFAULT 'info',
    message text NOT NULL,
    at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS app_runtime_logs_at ON app_runtime_logs(at);
COMMIT;
