BEGIN;
CREATE TABLE IF NOT EXISTS contents (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    kind text NOT NULL CHECK (kind IN ('skill','app')),
    name text NOT NULL,
    owner_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','approved','rejected','delisted')),
    current_release_id uuid,
    visibility text NOT NULL DEFAULT 'all' CHECK (visibility IN ('all','selected')),
    force_update boolean NOT NULL DEFAULT false,
    min_client_version text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    deleted_at timestamptz
);
CREATE UNIQUE INDEX IF NOT EXISTS contents_live_name ON contents(kind, name) WHERE deleted_at IS NULL;
CREATE TABLE IF NOT EXISTS content_releases (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    content_id uuid NOT NULL REFERENCES contents(id) ON DELETE CASCADE,
    version text NOT NULL,
    sha256 text NOT NULL,
    archive_size bigint NOT NULL CHECK (archive_size >= 0),
    archive bytea NOT NULL,
    changelog text NOT NULL DEFAULT '',
    created_by uuid NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    approved_by uuid REFERENCES users(id) ON DELETE SET NULL,
    approved_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (content_id, version)
);
CREATE TABLE IF NOT EXISTS content_grants (
    content_id uuid NOT NULL REFERENCES contents(id) ON DELETE CASCADE,
    grantee text NOT NULL,
    can_install boolean NOT NULL DEFAULT true,
    PRIMARY KEY (content_id, grantee)
);
CREATE TABLE IF NOT EXISTS content_deployments (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    content_id uuid NOT NULL REFERENCES contents(id) ON DELETE CASCADE,
    release_id uuid REFERENCES content_releases(id) ON DELETE SET NULL,
    action text NOT NULL CHECK (action IN ('publish','rollback','delist','delete','force_update')),
    actor_id uuid REFERENCES users(id) ON DELETE SET NULL,
    reason text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS content_installs (
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content_id uuid NOT NULL REFERENCES contents(id) ON DELETE CASCADE,
    release_id uuid NOT NULL REFERENCES content_releases(id) ON DELETE CASCADE,
    tree_sha256 text NOT NULL,
    source text NOT NULL DEFAULT 'server',
    installed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, content_id, release_id)
);
COMMIT;
