BEGIN;
CREATE TABLE IF NOT EXISTS wallet_accounts (
    user_id uuid PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    balance_micros bigint NOT NULL DEFAULT 0 CHECK (balance_micros >= 0),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS wallet_ledger (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(), user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    delta_micros bigint NOT NULL, balance_after bigint NOT NULL CHECK (balance_after >= 0),
    reason text NOT NULL, ref text NOT NULL, idempotency_key text UNIQUE, created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_ledger_user ON wallet_ledger(user_id, created_at DESC);
CREATE TABLE IF NOT EXISTS wallet_holds (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(), user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    request_id text, amount_micros bigint NOT NULL CHECK (amount_micros >= 0),
    status text NOT NULL DEFAULT 'held' CHECK (status IN ('held','settled','expired','void')),
    expires_at timestamptz NOT NULL, created_at timestamptz NOT NULL DEFAULT now()
);
ALTER TABLE wallet_holds ADD COLUMN IF NOT EXISTS request_id text;
COMMIT;
