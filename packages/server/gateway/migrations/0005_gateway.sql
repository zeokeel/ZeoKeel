BEGIN;
CREATE TABLE IF NOT EXISTS providers (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(), name text NOT NULL UNIQUE,
    kind text NOT NULL DEFAULT 'openai_responses', base_url text NOT NULL, api_key_enc text NOT NULL,
    enabled boolean NOT NULL DEFAULT true, created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS models (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(), provider_id uuid NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    model text NOT NULL, display_name text NOT NULL DEFAULT '', capabilities jsonb NOT NULL DEFAULT '{}'::jsonb,
    enabled boolean NOT NULL DEFAULT true, UNIQUE (provider_id, model)
);
CREATE TABLE IF NOT EXISTS usage_pending (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(), user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    request_id text NOT NULL UNIQUE, model text NOT NULL, estimated_micros bigint NOT NULL DEFAULT 0,
    status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','settled','void')),
    created_at timestamptz NOT NULL DEFAULT now(), settled_at timestamptz
);
CREATE TABLE IF NOT EXISTS usage (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(), request_id text NOT NULL UNIQUE,
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE, model text NOT NULL,
    provider text NOT NULL DEFAULT 'openai-compatible', input_tokens integer NOT NULL DEFAULT 0,
    cached_tokens integer NOT NULL DEFAULT 0, output_tokens integer NOT NULL DEFAULT 0,
    reasoning_tokens integer NOT NULL DEFAULT 0, amount_micros bigint NOT NULL DEFAULT 0,
    price_version text NOT NULL DEFAULT '', multiplier_bps integer NOT NULL DEFAULT 10000,
    status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','success','aborted','upstream_failed','estimated','voided')),
    source text NOT NULL DEFAULT 'upstream', created_at timestamptz NOT NULL DEFAULT now(), settled_at timestamptz
);
CREATE INDEX IF NOT EXISTS idx_usage_user_created ON usage(user_id, created_at DESC);
COMMIT;
