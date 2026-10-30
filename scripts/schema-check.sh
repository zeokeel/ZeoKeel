#!/usr/bin/env bash
# T22/C10: ensure generated JSON Schema and TS types are in sync.
set -euo pipefail
python3 scripts/schema-ts.py --check
for schema in shared/contracts/schema/*.json; do
  [[ -s "$schema" ]] || { echo "empty schema: $schema"; exit 1; }
done
echo "schema-check ok"
