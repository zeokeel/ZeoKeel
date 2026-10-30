#!/usr/bin/env bash
# M5 dist tree builder (dry-run capable): creates CLIENT-RELEASE/CHANNEL/SHA256SUMS/SBOM/VERSION.
set -euo pipefail
cd "$(dirname "$0")/.."
CHANNEL="${1:-official}"; VERSION="${2:-0.1.0}"; OUT="${3:-dist/$VERSION/$CHANNEL}"
mkdir -p "$OUT/clients" "$OUT/images" "$OUT/SBOM" "$OUT/deploy"
python3 - "$CHANNEL" "$VERSION" "$OUT" <<'PY'
import json,sys,pathlib,hashlib
channel,version,out=sys.argv[1],sys.argv[2],pathlib.Path(sys.argv[3])
clients=sorted((out/"clients").iterdir())
assets=[{"name":p.name,"sha256":hashlib.sha256(p.read_bytes()).hexdigest(),"size":p.stat().st_size} for p in clients]
release={"schema":1,"channel_id":channel,"edition":"enterprise","server_version":version,"client_version":version,"min_supported":version,"forced":False,"assets":assets}
(out/"CLIENT-RELEASE.json").write_text(json.dumps(release,indent=2)+"\n")
channel_json=json.loads(pathlib.Path("brands",channel,"channel.json").read_text())
(out/"CHANNEL.json").write_text(json.dumps({"channel":channel,"product_name":channel_json.get("identity",{}).get("product_name",channel),"powered_by":channel_json.get("brand",{}).get("powered_by","show")},indent=2)+"\n")
(out/"VERSION").write_text(version+"\n")
(out/"SBOM"/"README.md").write_text("# SBOM placeholder (dry-run; CI generates CycloneDX)\n")
(out/"CHANNEL.json").write_text((out/"CHANNEL.json").read_text())
(out/"deploy"/"README.md").write_text("# deploy bundle placeholder\n")
if not any((out/"clients").iterdir()):
    (out/"clients"/f"{channel}-{version}-linux.AppImage").write_text("dry-run client artifact\n")
PY
(cd "$OUT" && sha256sum CLIENT-RELEASE.json CHANNEL.json VERSION clients/* > SHA256SUMS)
printf '# Handover (dry-run)\n' > "$OUT/HANDOVER.md"
./target/debug/zk channel verify --channel "$CHANNEL" --dist "$OUT"
echo "build-dist ok: $OUT (dry-run artifacts unless provided)"
