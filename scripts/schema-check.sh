#!/usr/bin/env bash
# T22/C10: ensure generated JSON Schema + TS types/SCHEMAS are in sync and usable.
set -euo pipefail
python3 scripts/schema-ts.py --check
for schema in shared/contracts/schema/*.json; do
  [[ -s "$schema" ]] || { echo "empty schema: $schema"; exit 1; }
done
# Every schema file must be embedded + addressable through the SCHEMAS map
# (runtime Ajv validation, C10/T22) and the TS file must type-check standalone.
grep -q 'export const SCHEMAS' shared/contracts/ts/contracts.ts || {
  echo "schema-check: contracts.ts is missing the SCHEMAS map"
  exit 1
}
for schema in shared/contracts/schema/*.json; do
  key="$(basename "$schema" .json | tr '-' '_')"
  grep -q "^  ${key}: ${key}Schema,$" shared/contracts/ts/contracts.ts || {
    echo "schema-check: $schema missing from SCHEMAS map"
    exit 1
  }
done
echo "schema-check ok"
