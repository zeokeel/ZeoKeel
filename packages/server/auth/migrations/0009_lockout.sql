BEGIN;
CREATE TABLE IF NOT EXISTS login_attempts (
    scope text NOT NULL,
    subject text NOT NULL,
    failures integer NOT NULL DEFAULT 0,
    locked_until timestamptz,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (scope, subject)
);
COMMIT;
