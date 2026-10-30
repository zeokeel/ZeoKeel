BEGIN;
CREATE TABLE IF NOT EXISTS audit_log (
    id bigserial PRIMARY KEY, actor_id uuid REFERENCES users(id) ON DELETE SET NULL,
    action text NOT NULL, target text NOT NULL DEFAULT '', detail jsonb NOT NULL DEFAULT '{}'::jsonb,
    request_id text NOT NULL DEFAULT '', ip inet, device text NOT NULL DEFAULT '',
    prev_hash text NOT NULL, hash text NOT NULL, created_at timestamptz NOT NULL DEFAULT now()
);
COMMIT;
