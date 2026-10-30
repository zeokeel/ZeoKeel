BEGIN;
-- Forward-only: refresh tokens bind device_id + channel, and admin sessions get a
-- sliding idle window (docs/04 §3: refresh 绑定 device_id + channel; admin 12h
-- max + 60min idle).
ALTER TABLE refresh_tokens ADD COLUMN IF NOT EXISTS channel text NOT NULL DEFAULT 'official';
ALTER TABLE admin_sessions ADD COLUMN IF NOT EXISTS last_seen_at timestamptz NOT NULL DEFAULT now();
COMMENT ON COLUMN refresh_tokens.channel IS 'Channel the session was created on (docs/04 §3).';
COMMENT ON COLUMN admin_sessions.last_seen_at IS 'Last successful admin request; idle timeout window (docs/04 §3).';
CREATE INDEX IF NOT EXISTS idx_admin_sessions_idle ON admin_sessions(last_seen_at);
COMMIT;
