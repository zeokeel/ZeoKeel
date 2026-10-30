BEGIN;
CREATE TABLE IF NOT EXISTS connectors (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name text NOT NULL UNIQUE,
    transport text NOT NULL CHECK (transport IN ('stdio','http','sse')),
    config jsonb NOT NULL DEFAULT '{}'::jsonb,
    force_update boolean NOT NULL DEFAULT false,
    min_client_version text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    deleted_at timestamptz
);
CREATE TABLE IF NOT EXISTS connector_grants (
    connector_id uuid NOT NULL REFERENCES connectors(id) ON DELETE CASCADE,
    grantee text NOT NULL,
    can_enable boolean NOT NULL DEFAULT true,
    PRIMARY KEY (connector_id, grantee)
);
CREATE TABLE IF NOT EXISTS connector_oauth (
    connector_id uuid PRIMARY KEY REFERENCES connectors(id) ON DELETE CASCADE,
    client_id text NOT NULL,
    scopes jsonb NOT NULL DEFAULT '[]'::jsonb,
    redirect_uri text NOT NULL
);
CREATE TABLE IF NOT EXISTS connector_prefs (
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    connector_id uuid NOT NULL REFERENCES connectors(id) ON DELETE CASCADE,
    enabled boolean NOT NULL DEFAULT false,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, connector_id)
);
COMMIT;
