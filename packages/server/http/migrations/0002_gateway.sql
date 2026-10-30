-- M2 P2 gateway/pricing foundation (append-only; never edit 0001).
BEGIN;

ALTER TABLE wallet_holds ADD COLUMN IF NOT EXISTS request_id text;

CREATE TABLE IF NOT EXISTS model_prices (
    model                 text PRIMARY KEY,
    input_per_1k_micros   bigint NOT NULL DEFAULT 0 CHECK (input_per_1k_micros >= 0),
    output_per_1k_micros  bigint NOT NULL DEFAULT 0 CHECK (output_per_1k_micros >= 0),
    cached_per_1k_micros  bigint NOT NULL DEFAULT 0 CHECK (cached_per_1k_micros >= 0),
    peak_multiplier_bps   integer NOT NULL DEFAULT 10000 CHECK (peak_multiplier_bps > 0),
    offpeak_multiplier_bps integer NOT NULL DEFAULT 10000 CHECK (offpeak_multiplier_bps > 0),
    enabled               boolean NOT NULL DEFAULT true,
    updated_at            timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS usage (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id      text NOT NULL UNIQUE,
    user_id         uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    model           text NOT NULL,
    provider        text NOT NULL DEFAULT 'openai-compatible',
    input_tokens    integer NOT NULL DEFAULT 0,
    cached_tokens   integer NOT NULL DEFAULT 0,
    output_tokens   integer NOT NULL DEFAULT 0,
    reasoning_tokens integer NOT NULL DEFAULT 0,
    amount_micros   bigint NOT NULL DEFAULT 0,
    price_version   text NOT NULL DEFAULT '',
    multiplier_bps  integer NOT NULL DEFAULT 10000,
    status          text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','success','aborted','upstream_failed','estimated','voided')),
    source          text NOT NULL DEFAULT 'upstream',
    created_at      timestamptz NOT NULL DEFAULT now(),
    settled_at      timestamptz
);
CREATE INDEX IF NOT EXISTS idx_usage_user_created ON usage(user_id, created_at DESC);

COMMIT;
