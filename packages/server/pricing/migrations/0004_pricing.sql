BEGIN;
CREATE TABLE IF NOT EXISTS model_prices (
    model text PRIMARY KEY, input_per_1k_micros bigint NOT NULL DEFAULT 0 CHECK (input_per_1k_micros >= 0),
    output_per_1k_micros bigint NOT NULL DEFAULT 0 CHECK (output_per_1k_micros >= 0),
    cached_per_1k_micros bigint NOT NULL DEFAULT 0 CHECK (cached_per_1k_micros >= 0),
    peak_multiplier_bps integer NOT NULL DEFAULT 10000 CHECK (peak_multiplier_bps > 0),
    offpeak_multiplier_bps integer NOT NULL DEFAULT 10000 CHECK (offpeak_multiplier_bps > 0),
    enabled boolean NOT NULL DEFAULT true, updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS price_versions (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(), name text NOT NULL, effective_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(), UNIQUE (name, effective_at)
);
CREATE TABLE IF NOT EXISTS price_periods (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(), version_id uuid NOT NULL REFERENCES price_versions(id) ON DELETE CASCADE,
    model text NOT NULL, peak_micros_per_1k bigint NOT NULL CHECK (peak_micros_per_1k >= 0),
    offpeak_micros_per_1k bigint NOT NULL CHECK (offpeak_micros_per_1k >= 0),
    start_minute smallint NOT NULL CHECK (start_minute BETWEEN 0 AND 1439),
    end_minute smallint NOT NULL CHECK (end_minute BETWEEN 0 AND 1440)
);
COMMIT;
