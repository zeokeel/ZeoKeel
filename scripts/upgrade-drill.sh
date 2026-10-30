#!/usr/bin/env bash
# ZeoKeel M5/M6 upgrade drill.
#
# Modes:
#   --dry-run (default) : local, non-destructive checks only (manifest schema, asset sha256,
#                         backup layout). Docker steps are UNVERIFIED => exit 2.
#   --docker            : REAL backup -> upgrade -> rollback -> restore against a throwaway
#                         compose project (unique project name, no shared state touched).
#                         exit 0 only when every executed step passes; 1 on failure;
#                         2 when Docker / image archives / dist are unavailable (UNVERIFIED).
#   --full              : alias for --docker (kept for scripts/acceptance compatibility).
#
# Evidence: target/evidence/m6/upgrade-drill.json (docs/11 §5).
# Never runs against a production deployment: --docker always uses its own
# COMPOSE_PROJECT_NAME and `docker compose down -v` on exit.
set -uo pipefail
cd "$(dirname "$0")/.."

MODE="dry-run"; DIST=""; CHANNEL="official"; EDITION="enterprise"; OLD_IMAGE=""
ENV_FILE_ARG=""; COMPOSE_FILE="deploy/docker-compose.yml"
BACKUP_ROOT="target/evidence/m6/upgrade-drill-backup"
EVIDENCE="target/evidence/m6/upgrade-drill.json"
URL="${ZEOKEEL_DRILL_URL:-}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) MODE="dry-run"; shift ;;
    --docker) MODE="docker"; shift ;;
    --full) MODE="docker"; shift ;;
    --dist) DIST="${2:?--dist needs a value}"; shift 2 ;;
    --channel) CHANNEL="${2:?--channel needs a value}"; shift 2 ;;
    --edition) EDITION="${2:?--edition needs a value}"; shift 2 ;;
    --old-image) OLD_IMAGE="${2:?--old-image needs a value}"; shift 2 ;;
    --env-file) ENV_FILE_ARG="${2:?--env-file needs a value}"; shift 2 ;;
    --compose) COMPOSE_FILE="${2:?--compose needs a value}"; shift 2 ;;
    --backup-dir) BACKUP_ROOT="${2:?--backup-dir needs a value}"; shift 2 ;;
    --evidence) EVIDENCE="${2:?--evidence needs a value}"; shift 2 ;;
    -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
    *) echo "upgrade-drill: unknown argument: $1" >&2; exit 1 ;;
  esac
done

mkdir -p "$(dirname "$EVIDENCE")" "$BACKUP_ROOT"
TMP="$(mktemp -d)"
STEPS="$TMP/steps.tsv"
: > "$STEPS"
STATUS="unverified"
DETAIL=""
PROJECT="zeokeel-drill-$$"

log() { printf '%s\n' "$*"; }
record() { printf '%s\t%s\t%s\n' "$1" "$2" "${3:-}" >> "$STEPS"; log "  [$2] $1${3:+ — $3}"; }

write_evidence() {
  python3 - "$STEPS" "$EVIDENCE" "$MODE" "$STATUS" "$CHANNEL" "$EDITION" "$DETAIL" <<'PY'
import json
import sys

steps_path, evidence_path, mode, status, channel, edition, detail = sys.argv[1:8]
steps = []
with open(steps_path, encoding="utf-8") as handle:
    for line in handle:
        line = line.rstrip("\n")
        if not line:
            continue
        name, step_status, step_detail = (line.split("\t") + ["", ""])[:3]
        steps.append({"step": name, "status": step_status, "detail": step_detail})
payload = {
    "schema": 1,
    "mode": mode,
    "status": status,
    "channel": channel,
    "edition": edition,
    "detail": detail,
    "steps": steps,
}
with open(evidence_path, "w", encoding="utf-8") as handle:
    handle.write(json.dumps(payload, indent=2) + "\n")
print(f"upgrade-drill: {status} ({mode}) -> {evidence_path}")
PY
}

cleanup() {
  local rc=$?
  if [[ "${MODE:-}" == "docker" ]]; then
    docker compose --project-name "$PROJECT" --env-file "${ENV_FILE:-/dev/null}" -f "$COMPOSE_FILE" down -v --remove-orphans >/dev/null 2>&1 || true
  fi
  rm -rf "$TMP"
  exit "$rc"
}
trap cleanup EXIT

fail() { STATUS="fail"; DETAIL="${1:-}"; record "${2:-failure}" fail "$1"; write_evidence; exit 1; }
unverified_exit() { STATUS="unverified"; DETAIL="${1:-}"; record "overall" unverified "$1"; write_evidence; exit 2; }

# ---------------------------------------------------------------- dry-run checks
log "upgrade-drill: mode=$MODE channel=$CHANNEL edition=$EDITION"

MANIFEST="$TMP/manifest.json"
if [[ -n "$URL" ]] && curl -fsS "$URL/api/updates/manifest" \
      -H "X-ZeoKeel-Client: 0.1.0;${CHANNEL};linux-x64;drill" -o "$MANIFEST"; then
  record manifest pass "downloaded from $URL"
else
  cp deploy/updates/CLIENT-RELEASE.json "$MANIFEST"
  record manifest pass "local fallback deploy/updates/CLIENT-RELEASE.json (server unavailable)"
fi

SCHEMA_RC=0
python3 - "$MANIFEST" "$CHANNEL" "$EDITION" > "$TMP/manifest-check.txt" 2>&1 <<'PY' || SCHEMA_RC=$?
import json
import sys

path, channel, edition = sys.argv[1:4]
manifest = json.load(open(path, encoding="utf-8"))
problems = []
if manifest.get("schema") != 1:
    problems.append("schema != 1")
if manifest.get("channel_id") != channel:
    problems.append(f"channel_id {manifest.get('channel_id')!r} != {channel!r}")
if manifest.get("edition") != edition:
    problems.append(f"edition {manifest.get('edition')!r} != {edition!r}")
assets = manifest.get("assets") or []
if not assets:
    problems.append("assets is empty")
for asset in assets:
    name = asset.get("name")
    sha = asset.get("sha256") or ""
    if not name:
        problems.append("asset without name")
    if len(sha) != 64:
        problems.append(f"asset {name!r}: sha256 must be 64 hex chars")
for problem in problems:
    print(problem)
sys.exit(1 if problems else 0)
PY
if [[ "$SCHEMA_RC" -ne 0 ]]; then
  fail "manifest schema/channel/edition/assets invalid: $(tr '\n' ';' < "$TMP/manifest-check.txt")" "manifest-schema"
fi
record manifest-schema pass "schema/channel/edition/assets valid"

ASSET_RC=0
python3 - "$MANIFEST" "$DIST" > "$TMP/asset-check.txt" 2>&1 <<'PY' || ASSET_RC=$?
import hashlib
import json
import pathlib
import sys

manifest_path, dist = sys.argv[1:3]
manifest = json.load(open(manifest_path, encoding="utf-8"))
roots = [pathlib.Path("deploy/updates/clients")]
if dist:
    roots.insert(0, pathlib.Path(dist) / "clients")
problems = []
checked = 0
for asset in manifest.get("assets") or []:
    name = asset["name"]
    candidates = [root / name for root in roots]
    candidates += [path for path in roots[0].glob(name) if path.is_file()] if roots[0].is_dir() else []
    path = next((candidate for candidate in candidates if candidate.is_file()), None)
    if path is None:
        problems.append(f"asset missing: {name} (looked in {[str(root) for root in roots]})")
        continue
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if digest != asset.get("sha256"):
        problems.append(f"sha256 mismatch for {name}: manifest={asset.get('sha256')} file={digest}")
    checked += 1
print(f"checked {checked} asset(s)")
for problem in problems:
    print(problem)
sys.exit(1 if problems else 0)
PY
if [[ "$ASSET_RC" -ne 0 ]]; then
  fail "asset verification failed: $(tr '\n' ';' < "$TMP/asset-check.txt")" "client-sha256"
fi
record client-sha256 pass "$(tr '\n' ' ' < "$TMP/asset-check.txt")"

if ! command -v docker >/dev/null 2>&1; then
  record backup-docker unverified "docker unavailable on this host"
  record upgrade-rollback-restore unverified "docker unavailable; use --docker on a Docker host"
  unverified_exit "docker unavailable: backup/upgrade/rollback/restore not executed (docs/11 §5 UNVERIFIED)"
fi
if [[ "$MODE" != "docker" ]]; then
  record backup-docker unverified "dry-run does not touch Docker"
  record upgrade-rollback-restore unverified "dry-run does not execute upgrade/rollback/restore"
  unverified_exit "dry-run: only local manifest/asset checks executed; run --docker for the real drill"
fi

# ---------------------------------------------------------------- docker drill
if [[ -z "$DIST" ]]; then
  unverified_exit "--docker requires --dist <dist/<ver>/<channel>> with images/"
fi
if [[ ! -d "$DIST/images" ]]; then
  unverified_exit "--dist has no images/ directory: $DIST/images"
fi
mapfile -t IMAGE_ARCHIVES < <(find "$DIST/images" -maxdepth 1 -type f | sort)
if [[ "${#IMAGE_ARCHIVES[@]}" -lt 1 ]]; then
  unverified_exit "no image archives under $DIST/images"
fi
if [[ ! -f "$COMPOSE_FILE" ]]; then
  fail "compose file missing: $COMPOSE_FILE" "compose-file"
fi

ENV_FILE="$ENV_FILE_ARG"
if [[ -z "$ENV_FILE" ]]; then
  ENV_FILE="$TMP/drill.env"
  python3 - "$ENV_FILE" "$PROJECT" <<'PY'
import secrets
import sys

path, project = sys.argv[1:3]
alphabet = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
def token(length: int) -> str:
    return "".join(secrets.choice(alphabet) for _ in range(length))
def hex32() -> str:
    return secrets.token_hex(32)
lines = [
    "DOMAIN=localhost",
    "TLS_MODE=internal",
    f"ADMIN_PASSWORD={token(24)}",
    f"PG_PASSWORD={hex32()}",
    f"MASTER_KEY={token(43)}=",
    "SERVER_IMAGE=zeokeel/server:drill-pending",
    f"COMPOSE_PROJECT_NAME={project}",
]
open(path, "w", encoding="utf-8").write("\n".join(lines) + "\n")
PY
fi
export COMPOSE_PROJECT_NAME="$PROJECT"
dc() { docker compose --project-name "$PROJECT" --env-file "$ENV_FILE" -f "$COMPOSE_FILE" "$@"; }

if ! dc config >/dev/null 2>"$TMP/compose-config.err"; then
  fail "docker compose config rejected the drill env: $(tr '\n' ' ' < "$TMP/compose-config.err")" "compose-config"
fi
record compose-config pass "docker compose config accepted"

set_server_image() {
  python3 - "$ENV_FILE" "$1" <<'PY'
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
image = sys.argv[2]
lines = []
found = False
for line in path.read_text().splitlines():
    if line.startswith("SERVER_IMAGE="):
        lines.append(f"SERVER_IMAGE={image}")
        found = True
    else:
        lines.append(line)
if not found:
    lines.append(f"SERVER_IMAGE={image}")
path.write_text("\n".join(lines) + "\n")
PY
}

wait_ready() {
  local attempt
  for attempt in $(seq 1 120); do
    if dc exec -T server curl -fsS http://127.0.0.1:8787/readyz >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  return 1
}

record docker-available pass "$(docker --version 2>/dev/null | head -1)"
NEW_IMAGE=""
for archive in "${IMAGE_ARCHIVES[@]}"; do
  LOAD_LOG="$TMP/load-$(basename "$archive").log"
  if ! docker load -i "$archive" > "$LOAD_LOG" 2>&1; then
    fail "docker load failed for $(basename "$archive"): $(tr '\n' ' ' < "$LOAD_LOG")" "docker-load"
  fi
  while IFS= read -r ref; do
    if [[ "$ref" == *"server"* ]]; then NEW_IMAGE="$ref"; fi
  done < <(sed -n 's/^Loaded image: //p' "$LOAD_LOG")
done
if [[ -z "$NEW_IMAGE" ]]; then
  unverified_exit "no server image reference found in --dist/images archives (expected Loaded image: .../server:...)"
fi
record docker-load pass "new server image: $NEW_IMAGE"

OLD_IMAGE_VALUE="$OLD_IMAGE"
if [[ -z "$OLD_IMAGE_VALUE" ]]; then
  RUNNING="$(docker ps -q --filter "label=com.docker.compose.project=$PROJECT" --filter "label=com.docker.compose.service=server" | head -1)"
  if [[ -n "$RUNNING" ]]; then
    OLD_IMAGE_VALUE="$(docker inspect --format '{{.Config.Image}}' "$RUNNING")"
  fi
fi
SAME_IMAGE="false"
if [[ -z "$OLD_IMAGE_VALUE" ]]; then
  OLD_IMAGE_VALUE="$NEW_IMAGE"
  SAME_IMAGE="true"
fi

export ENV_FILE COMPOSE_FILE COMPOSE_PROJECT_NAME BACKUP_DIR
if ! set_server_image "$OLD_IMAGE_VALUE"; then
  fail "failed to set SERVER_IMAGE=$OLD_IMAGE_VALUE in $ENV_FILE" "env-write"
fi

# 1. backup (real: pg_dump + /data tar, master.key excluded)
if ! dc up -d postgres server >/dev/null 2>"$TMP/up-old.err"; then
  fail "compose up (old image) failed: $(tr '\n' ' ' < "$TMP/up-old.err")" "up-old"
fi
if ! wait_ready; then
  fail "server did not become ready (old image $OLD_IMAGE_VALUE)" "readyz-old"
fi
if ! BACKUP_DIR="$BACKUP_ROOT" bash deploy/scripts/backup.sh > "$TMP/backup.log" 2>&1; then
  fail "backup failed: $(tr '\n' ' ' < "$TMP/backup.log")" "backup"
fi
BACKUP_DIR_PATH="$(ls -1d "$BACKUP_ROOT"/*/ 2>/dev/null | tail -1)"
BACKUP_DIR_PATH="${BACKUP_DIR_PATH%/}"
if [[ -z "$BACKUP_DIR_PATH" ]] || [[ ! -s "$BACKUP_DIR_PATH/zeokeel.dump" ]]; then
  fail "backup produced no zeokeel.dump under $BACKUP_ROOT" "backup"
fi
record backup pass "PG dump + /data archive: $BACKUP_DIR_PATH"

# 2. upgrade (real: new image, restart, readyz)
if ! set_server_image "$NEW_IMAGE"; then
  fail "failed to set SERVER_IMAGE=$NEW_IMAGE" "env-write"
fi
if ! dc up -d server >/dev/null 2>"$TMP/up-new.err"; then
  fail "upgrade compose up (new image) failed: $(tr '\n' ' ' < "$TMP/up-new.err")" "upgrade"
fi
if ! wait_ready; then
  fail "server did not become ready after upgrade ($NEW_IMAGE)" "readyz-new"
fi
record upgrade pass "new image started: $NEW_IMAGE"

# 3. rollback (real: previous image ref, restart, readyz)
if ! set_server_image "$OLD_IMAGE_VALUE"; then
  fail "failed to set SERVER_IMAGE=$OLD_IMAGE_VALUE" "env-write"
fi
if ! dc up -d server >/dev/null 2>"$TMP/up-rollback.err"; then
  fail "rollback compose up failed: $(tr '\n' ' ' < "$TMP/up-rollback.err")" "rollback"
fi
if ! wait_ready; then
  fail "server did not become ready after rollback" "readyz-rollback"
fi
if [[ "$SAME_IMAGE" == "true" ]]; then
  record rollback pass "previous ref $OLD_IMAGE_VALUE (same image: image rollback not exercised)"
else
  record rollback pass "rolled back to $OLD_IMAGE_VALUE"
fi

# 4. restore (real: stop server, pg_restore + /data, start, readyz, login smoke)
if ! dc stop server >/dev/null 2>&1; then
  fail "could not stop server for restore" "restore-stop"
fi
if ! bash deploy/scripts/restore.sh "$BACKUP_DIR_PATH" > "$TMP/restore.log" 2>&1; then
  fail "restore failed: $(tr '\n' ' ' < "$TMP/restore.log")" "restore"
fi
if ! dc up -d server >/dev/null 2>"$TMP/up-restore.err"; then
  fail "compose up after restore failed: $(tr '\n' ' ' < "$TMP/up-restore.err")" "restore-up"
fi
if ! wait_ready; then
  fail "server did not become ready after restore" "readyz-restore"
fi
ADMIN_PASSWORD_VALUE="$(sed -n 's/^ADMIN_PASSWORD=//p' "$ENV_FILE" | head -1)"
LOGIN_CODE="$(dc exec -T server curl -sS -o /dev/null -w '%{http_code}' \
  -X POST http://127.0.0.1:8787/api/client/v1/auth/login \
  -H 'content-type: application/json' \
  -d "{\"username\":\"admin\",\"password\":\"$ADMIN_PASSWORD_VALUE\",\"device_id\":\"upgrade-drill\"}" 2>/dev/null)"
if [[ "$LOGIN_CODE" != "200" ]]; then
  fail "post-restore admin login returned HTTP $LOGIN_CODE" "restore-login"
fi
record restore pass "PG + /data restored, readyz ok, admin login 200"

if [[ "$SAME_IMAGE" == "true" ]]; then
  record data-migration unverified "same image: no schema migration was exercised (needs N-1 image via --old-image)"
fi
record caddy-tls unverified "drill runs postgres+server only; Caddy TLS e2e is not exercised"

STATUS="pass"
DETAIL="backup/upgrade/rollback/restore executed against throwaway project $PROJECT"
write_evidence
log "upgrade-drill: PASS (throwaway project $PROJECT)"
exit 0
