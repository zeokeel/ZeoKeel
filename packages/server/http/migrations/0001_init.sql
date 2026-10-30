-- ZeoKeel M2 P1 initial schema (PostgreSQL 16).
-- Migrations are immutable; changes go in new numbered files, never edit history.
BEGIN;

CREATE TABLE IF NOT EXISTS schema_migrations (
    id          text PRIMARY KEY,
    checksum    text NOT NULL,
    applied_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS users (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    username        text NOT NULL UNIQUE,
    display_name    text NOT NULL DEFAULT '',
    password_hash   text,
    role            text NOT NULL CHECK (role IN ('super_admin','auditor','employee')),
    status          text NOT NULL DEFAULT 'active' CHECK (status IN ('active','disabled')),
    token_epoch     integer NOT NULL DEFAULT 0,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    last_login_at   timestamptz,
    deleted_at      timestamptz
);

CREATE TABLE IF NOT EXISTS user_devices (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id   text NOT NULL,
    platform    text NOT NULL,
    channel     text NOT NULL DEFAULT 'official',
    first_seen_at timestamptz NOT NULL DEFAULT now(),
    last_seen_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (user_id, device_id)
);

CREATE TABLE IF NOT EXISTS refresh_tokens (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id   text NOT NULL,
    token_hash  text NOT NULL UNIQUE,
    expires_at  timestamptz NOT NULL,
    revoked_at  timestamptz,
    replaced_by uuid,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_refresh_user ON refresh_tokens(user_id, expires_at);

CREATE TABLE IF NOT EXISTS admin_sessions (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_hash  text NOT NULL UNIQUE,
    csrf_hash     text NOT NULL,
    expires_at    timestamptz NOT NULL,
    revoked_at    timestamptz,
    created_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS wallet_accounts (
    user_id        uuid PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    balance_micros bigint NOT NULL DEFAULT 0 CHECK (balance_micros >= 0),
    updated_at     timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS wallet_ledger (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    delta_micros  bigint NOT NULL,
    balance_after bigint NOT NULL CHECK (balance_after >= 0),
    reason        text NOT NULL,
    ref           text NOT NULL,
    idempotency_key text UNIQUE,
    created_at    timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_ledger_user ON wallet_ledger(user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS wallet_holds (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount_micros bigint NOT NULL CHECK (amount_micros >= 0),
    status        text NOT NULL DEFAULT 'held' CHECK (status IN ('held','settled','expired','void')),
    expires_at    timestamptz NOT NULL,
    created_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS usage_pending (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    request_id    text NOT NULL UNIQUE,
    model         text NOT NULL,
    estimated_micros bigint NOT NULL DEFAULT 0,
    status        text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','settled','void')),
    created_at    timestamptz NOT NULL DEFAULT now(),
    settled_at    timestamptz
);

CREATE TABLE IF NOT EXISTS price_versions (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name          text NOT NULL,
    effective_at  timestamptz NOT NULL,
    created_at    timestamptz NOT NULL DEFAULT now(),
    UNIQUE (name, effective_at)
);

CREATE TABLE IF NOT EXISTS price_periods (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    version_id  uuid NOT NULL REFERENCES price_versions(id) ON DELETE CASCADE,
    model       text NOT NULL,
    peak_micros_per_1k bigint NOT NULL CHECK (peak_micros_per_1k >= 0),
    offpeak_micros_per_1k bigint NOT NULL CHECK (offpeak_micros_per_1k >= 0),
    start_minute smallint NOT NULL CHECK (start_minute BETWEEN 0 AND 1439),
    end_minute   smallint NOT NULL CHECK (end_minute BETWEEN 0 AND 1440)
);

CREATE TABLE IF NOT EXISTS providers (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name        text NOT NULL UNIQUE,
    kind        text NOT NULL DEFAULT 'openai_responses',
    base_url    text NOT NULL,
    api_key_enc text NOT NULL,
    enabled     boolean NOT NULL DEFAULT true,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS models (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_id uuid NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    model       text NOT NULL,
    display_name text NOT NULL DEFAULT '',
    capabilities jsonb NOT NULL DEFAULT '{}'::jsonb,
    enabled     boolean NOT NULL DEFAULT true,
    UNIQUE (provider_id, model)
);

CREATE TABLE IF NOT EXISTS gateway_usage (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    request_id      text NOT NULL UNIQUE,
    model           text NOT NULL,
    prompt_tokens   integer NOT NULL DEFAULT 0,
    completion_tokens integer NOT NULL DEFAULT 0,
    cost_micros     bigint NOT NULL DEFAULT 0,
    estimated       boolean NOT NULL DEFAULT false,
    created_at      timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_usage_user ON gateway_usage(user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS audit_log (
    id          bigserial PRIMARY KEY,
    actor_id    uuid REFERENCES users(id) ON DELETE SET NULL,
    action      text NOT NULL,
    target      text NOT NULL DEFAULT '',
    detail      jsonb NOT NULL DEFAULT '{}'::jsonb,
    request_id  text NOT NULL DEFAULT '',
    ip          inet,
    device      text NOT NULL DEFAULT '',
    prev_hash   text NOT NULL,
    hash        text NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS system_settings (
    key         text PRIMARY KEY,
    value       jsonb NOT NULL,
    hot         boolean NOT NULL DEFAULT false,
    updated_at  timestamptz NOT NULL DEFAULT now(),
    updated_by  uuid REFERENCES users(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS job_runs (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    job         text NOT NULL,
    idempotency_key text NOT NULL UNIQUE,
    status      text NOT NULL CHECK (status IN ('running','done','failed')),
    detail      jsonb NOT NULL DEFAULT '{}'::jsonb,
    started_at  timestamptz NOT NULL DEFAULT now(),
    finished_at timestamptz
);

COMMIT;
