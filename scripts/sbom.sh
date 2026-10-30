#!/usr/bin/env bash
# M5/M6 SBOM + license inventory (CycloneDX-shaped, offline from cargo metadata).
set -euo pipefail
cd "$(dirname "$0")/.."
OUT="${1:-dist/sbom.json}"
python3 - "$OUT" <<'PY'
import json,subprocess,sys,pathlib
meta=json.loads(subprocess.check_output(["cargo","metadata","--no-deps","--format-version","1"],text=True))
components=[{"type":"library","name":p["name"],"version":p["version"],"licenses":[{"license":{"id":p.get("license") or "unknown"}}]} for p in meta["packages"]]
sbom={"bomFormat":"CycloneDX","specVersion":"1.5","version":1,"metadata":{"component":{"type":"application","name":"ZeoKeel"}},"components":components}
path=pathlib.Path(sys.argv[1]); path.parent.mkdir(parents=True,exist_ok=True); path.write_text(json.dumps(sbom,indent=2)+"\n")
print(f"sbom ok: {len(components)} components -> {path}")
PY
