BEGIN;
CREATE TABLE IF NOT EXISTS users (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(), username text NOT NULL UNIQUE,
    display_name text NOT NULL DEFAULT '', password_hash text,
    role text NOT NULL CHECK (role IN ('super_admin','auditor','employee')),
    status text NOT NULL DEFAULT 'active' CHECK (status IN ('active','disabled')),
    token_epoch integer NOT NULL DEFAULT 0, created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(), last_login_at timestamptz, deleted_at timestamptz
);
CREATE TABLE IF NOT EXISTS user_devices (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(), user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id text NOT NULL, platform text NOT NULL, channel text NOT NULL DEFAULT 'official',
    first_seen_at timestamptz NOT NULL DEFAULT now(), last_seen_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (user_id, device_id)
);
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(), user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id text NOT NULL, token_hash text NOT NULL UNIQUE, expires_at timestamptz NOT NULL,
    revoked_at timestamptz, replaced_by uuid, created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_refresh_user ON refresh_tokens(user_id, expires_at);
CREATE TABLE IF NOT EXISTS admin_sessions (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(), user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_hash text NOT NULL UNIQUE, csrf_hash text NOT NULL, expires_at timestamptz NOT NULL,
    revoked_at timestamptz, created_at timestamptz NOT NULL DEFAULT now()
);
COMMIT;
