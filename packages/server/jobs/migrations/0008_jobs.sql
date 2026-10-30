BEGIN;
CREATE TABLE IF NOT EXISTS job_runs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(), job text NOT NULL,
    idempotency_key text NOT NULL UNIQUE, status text NOT NULL CHECK (status IN ('running','done','failed')),
    detail jsonb NOT NULL DEFAULT '{}'::jsonb, started_at timestamptz NOT NULL DEFAULT now(), finished_at timestamptz
);
COMMIT;
