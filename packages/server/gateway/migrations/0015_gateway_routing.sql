BEGIN;
-- Forward-only: DB-driven upstream routing/failover knobs for the AI gateway
-- (docs/07 §4: providers table carries priority/timeout/max_retries; server
-- settings must be admin-configurable rather than hard-coded in env).
ALTER TABLE providers ADD COLUMN IF NOT EXISTS priority integer NOT NULL DEFAULT 100;
ALTER TABLE providers ADD COLUMN IF NOT EXISTS timeout_secs integer NOT NULL DEFAULT 120
    CHECK (timeout_secs BETWEEN 1 AND 600);
ALTER TABLE providers ADD COLUMN IF NOT EXISTS max_retries integer NOT NULL DEFAULT 1
    CHECK (max_retries BETWEEN 0 AND 5);
COMMENT ON COLUMN providers.priority IS 'Lower value is tried first during upstream failover (docs/07 §4).';
COMMENT ON COLUMN providers.timeout_secs IS 'Per-attempt upstream timeout in seconds (docs/07 §4).';
COMMENT ON COLUMN providers.max_retries IS 'Additional same-provider attempts for retryable upstream errors (docs/07 §4).';
COMMIT;
