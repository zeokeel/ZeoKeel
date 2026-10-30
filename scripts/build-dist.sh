#!/usr/bin/env bash
# ZeoKeel M5 dist tree builder — NEVER fabricates artifacts.
#
# Contract (docs/09 §4/§5, docs/11 §5):
#   * The dist tree is assembled only from REAL build outputs supplied by the caller:
#       --artifacts <dir>  real per-platform client installers (AppImage/deb/rpm/dmg/exe/msi)
#       --images <dir>     real `docker save`d image archives (enterprise only; docs/09 §5: 3 images)
#       --sbom <file>      real CycloneDX SBOM (scripts/sbom.sh output)
#   * If any required real input is missing/incomplete the script writes an explicit
#     UNVERIFIED.json and exits 2. It NEVER writes CLIENT-RELEASE.json / SHA256SUMS
#     when the inputs are placeholders, so `zk channel verify` and the release upload
#     cannot mistake a dry-run for a deliverable.
#   * Exit codes: 0 = dist assembled and verified by `zk channel verify`;
#                 1 = invalid input (placeholder/symlink/unknown channel/illegal edition);
#                 2 = UNVERIFIED (real artifact/image/SBOM inputs not available).
#
# Signed CLIENT-RELEASE manifests and notarized installers are produced by the release
# environment (docs/09 §7.4); this script records them as unverified in BUILD-STATUS.json.
set -euo pipefail
cd "$(dirname "$0")/.."

usage() {
  cat <<'EOF'
usage: scripts/build-dist.sh --channel <official|oem-*> --version <x.y.z> \
         --edition <personal|enterprise> --artifacts <dir> \
         [--images <dir>] [--sbom <cyclonedx.json>] [--out <dir>]

  --artifacts  directory containing real client installers (required)
  --images     directory containing real `docker save` archives (required for enterprise)
  --sbom       real CycloneDX SBOM (required)
  --out        output directory (default: dist/<version>/<channel>; must not exist)
exit: 0 verified, 1 invalid input, 2 UNVERIFIED (missing real inputs)
EOF
}

CHANNEL=""; VERSION=""; EDITION=""; ARTIFACTS=""; IMAGES=""; SBOM=""; OUT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --channel) CHANNEL="${2:?--channel needs a value}"; shift 2 ;;
    --version) VERSION="${2:?--version needs a value}"; shift 2 ;;
    --edition) EDITION="${2:?--edition needs a value}"; shift 2 ;;
    --artifacts) ARTIFACTS="${2:?--artifacts needs a value}"; shift 2 ;;
    --images) IMAGES="${2:?--images needs a value}"; shift 2 ;;
    --sbom) SBOM="${2:?--sbom needs a value}"; shift 2 ;;
    --out) OUT="${2:?--out needs a value}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "build-dist: unknown argument: $1" >&2; usage >&2; exit 1 ;;
  esac
done

if [[ -z "$CHANNEL" || -z "$VERSION" || -z "$EDITION" ]]; then
  echo "build-dist: --channel/--version/--edition are required" >&2
  usage >&2
  exit 1
fi
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "build-dist: bad version: $VERSION" >&2; exit 1; }
case "$EDITION" in personal|enterprise) ;; *) echo "build-dist: bad edition: $EDITION" >&2; exit 1 ;; esac
[[ -f "brands/$CHANNEL/channel.json" ]] || { echo "build-dist: unknown channel: $CHANNEL (brands/$CHANNEL/channel.json missing)" >&2; exit 1; }
OUT="${OUT:-dist/$VERSION/$CHANNEL}"
[[ -e "$OUT" ]] && { echo "build-dist: refusing to write $OUT (already exists; remove it for a clean build)" >&2; exit 1; }

# Channel/edition legality rules mirror zk compose + docs/09 §1.
if [[ "$EDITION" == "personal" && "$CHANNEL" != "official" ]]; then
  echo "build-dist: personal edition only supports channel official (docs/09 09-Q3)" >&2
  exit 1
fi
if [[ "$EDITION" == "enterprise" && "$CHANNEL" != "official" && "$CHANNEL" != oem-* ]]; then
  echo "build-dist: enterprise channel must be official or oem-<customer>, got $CHANNEL" >&2
  exit 1
fi

# Validate inputs + assemble. The python step is the single "real vs placeholder" judge.
set +e
python3 - "$CHANNEL" "$VERSION" "$EDITION" "$OUT" "$ARTIFACTS" "$IMAGES" "$SBOM" <<'PY'
import hashlib
import json
import pathlib
import shutil
import sys


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_channel(channel: str) -> dict:
    return json.loads((pathlib.Path("brands") / channel / "channel.json").read_text())


TEXT_SUFFIXES = {".md", ".yml", ".yaml", ".json", ".sh", ".example", ".html", ".txt", ".conf", ".toml"}


def text_files(root: pathlib.Path):
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        if path.suffix in TEXT_SUFFIXES or path.name.startswith(("Dockerfile", "Caddyfile", "LICENSE", "NOTICE")):
            yield path


def sanitize_brand_residue(root: pathlib.Path, product_name: str) -> None:
    # docs/09 §5 产物级白标: delivered text assets must not carry the vendor product
    # string on a `powered_by=hide` channel. Binary artifacts are NOT rewritten here;
    # `zk channel verify` remains the gate for real binaries and their file names.
    if not product_name:
        return
    for path in text_files(root):
        try:
            text = path.read_text()
        except UnicodeDecodeError:
            continue
        if "ZeoKeel" in text:
            path.write_text(text.replace("ZeoKeel", product_name))


channel, version, edition, out_arg, artifacts_arg, images_arg, sbom_arg = sys.argv[1:8]
channel_meta = load_channel(channel)
identity = channel_meta.get("identity", {})
product_name = str(identity.get("product_name") or channel_meta.get("id") or channel).strip()
powered_by = str(channel_meta.get("brand", {}).get("powered_by") or "show")
out = pathlib.Path(out_arg)

unverified: list[str] = []
invalid: list[str] = []


def require_dir(arg: str, label: str) -> pathlib.Path | None:
    if not arg:
        unverified.append(f"--{label} not provided")
        return None
    root = pathlib.Path(arg)
    if not root.is_dir():
        unverified.append(f"--{label} directory missing: {arg}")
        return None
    return root


def real_files(root: pathlib.Path) -> list[pathlib.Path]:
    files = []
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            invalid.append(f"symlink not allowed in build inputs: {path}")
            continue
        if path.is_file():
            files.append(path)
    return files


def plausible_installer(path: pathlib.Path) -> None:
    size = path.stat().st_size
    with path.open("rb") as handle:
        head = handle.read(64)
    if size < 1024 * 1024:
        invalid.append(f"{path.name}: {size} bytes is too small to be a real installer (placeholder?)")
        return
    if head.startswith(b"dry-run") or b"placeholder" in head.lower():
        invalid.append(f"{path.name}: contains a dry-run/placeholder marker")
        return
    suffix = path.suffix.lower()
    if suffix in {".appimage", ".deb", ".rpm"} and not head.startswith(b"\x7fELF"):
        invalid.append(f"{path.name}: {suffix} is not an ELF binary")
    if suffix in {".exe", ".msi"} and not head.startswith(b"MZ"):
        invalid.append(f"{path.name}: {suffix} is not a PE binary")


def plausible_image(path: pathlib.Path) -> None:
    if path.stat().st_size <= 0:
        invalid.append(f"image archive is empty: {path.name}")


def validate_sbom(path: pathlib.Path) -> dict | None:
    try:
        data = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        invalid.append(f"SBOM is not valid JSON: {error}")
        return None
    if data.get("bomFormat") != "CycloneDX":
        invalid.append("SBOM bomFormat must be CycloneDX")
        return None
    if not (data.get("components") or []):
        invalid.append("SBOM has no components (not a real SBOM)")
        return None
    return data


artifacts_root = require_dir(artifacts_arg, "artifacts")
clients: list[pathlib.Path] = []
seen_names: set[str] = set()
if artifacts_root is not None:
    clients = real_files(artifacts_root)
    if not clients:
        unverified.append(f"--artifacts {artifacts_arg} contains no files")
    for client in clients:
        if client.name in seen_names:
            invalid.append(f"duplicate client artifact name: {client.name}")
        seen_names.add(client.name)
        plausible_installer(client)

images_root = require_dir(images_arg, "images") if edition == "enterprise" else None
images: list[pathlib.Path] = []
if edition == "enterprise":
    if images_root is not None:
        images = real_files(images_root)
        if len(images) < 3:
            unverified.append(
                f"--images {images_arg} has {len(images)} archive(s); docs/09 §5 requires 3 (server+postgres+caddy)"
            )
        for image in images:
            plausible_image(image)
elif images_arg:
    invalid.append("--images is only valid for edition=enterprise (docs/09 §6: personal has no images)")

sbom_data = None
if not sbom_arg:
    unverified.append("--sbom not provided")
else:
    sbom_path = pathlib.Path(sbom_arg)
    if not sbom_path.is_file():
        unverified.append(f"--sbom file missing: {sbom_arg}")
    else:
        sbom_data = validate_sbom(sbom_path)

if invalid:
    print("build-dist: INVALID INPUT (not an evidence problem):", file=sys.stderr)
    for reason in invalid:
        print(f"  - {reason}", file=sys.stderr)
    sys.exit(1)

if unverified:
    out.mkdir(parents=True, exist_ok=True)
    (out / "UNVERIFIED.json").write_text(json.dumps({
        "schema": 1,
        "status": "unverified",
        "channel": channel,
        "edition": edition,
        "version": version,
        "reasons": unverified,
        "note": "No CLIENT-RELEASE.json/SHA256SUMS were written: placeholders must not look like deliverables (docs/11 §5).",
    }, indent=2) + "\n")
    print("build-dist: UNVERIFIED — refusing to fabricate a dist tree:", file=sys.stderr)
    for reason in unverified:
        print(f"  - {reason}", file=sys.stderr)
    sys.exit(2)

assert artifacts_root is not None and sbom_data is not None
if edition == "enterprise":
    assert images_root is not None

(out / "clients").mkdir(parents=True)
(out / "SBOM").mkdir(parents=True)
(out / "deploy").mkdir(parents=True)
if edition == "enterprise":
    (out / "images").mkdir(parents=True)

for client in clients:
    shutil.copy2(client, out / "clients" / client.name)
for image in images:
    shutil.copy2(image, out / "images" / image.name)

assets = [
    {"name": client.name, "sha256": sha256(client), "size": client.stat().st_size}
    for client in sorted((out / "clients").iterdir(), key=lambda path: path.name)
]

# SBOM: keep the real component inventory; white-label only the metadata component name.
sbom_data.setdefault("metadata", {}).setdefault("component", {})["name"] = product_name
(out / "SBOM" / "cyclonedx.json").write_text(json.dumps(sbom_data, indent=2) + "\n")

# Deploy bundle (docs/09 §5): compose + .env.example + caddy + scripts + docker + portal.
for rel in ["docker-compose.yml", ".env.example", "README.md"]:
    src = pathlib.Path("deploy") / rel
    if src.is_file():
        shutil.copy2(src, out / "deploy" / rel)
for rel in ["caddy", "scripts", "docker", "portal"]:
    src = pathlib.Path("deploy") / rel
    if src.is_dir():
        shutil.copytree(
            src,
            out / "deploy" / rel,
            ignore=shutil.ignore_patterns("certs", "*.pem", "*.key", "node_modules"),
        )
sanitize_brand_residue(out / "deploy", product_name)

(out / "CHANNEL.json").write_text(json.dumps({
    "channel": channel,
    "edition": edition,
    "product_name": product_name,
    "powered_by": powered_by,
}, indent=2) + "\n")

release = {
    "schema": 1,
    "channel_id": channel,
    "edition": edition,
    "server_version": version,
    "client_version": version,
    "min_supported": version,
    "forced": False,
    "assets": assets,
}
(out / "CLIENT-RELEASE.json").write_text(json.dumps(release, indent=2) + "\n")
(out / "VERSION").write_text(version + "\n")

status = {
    "schema": 1,
    "status": "verified-local",
    "channel": channel,
    "edition": edition,
    "version": version,
    "sha256": {
        "artifacts": [sha256(client) for client in clients],
        "images": [sha256(image) for image in images],
        "sbom": sha256(pathlib.Path(sbom_arg)),
    },
    "unverified": [
        "CLIENT-RELEASE manifest signature (release signing is a release-environment step, docs/09 §7.4)",
        "macOS notarize/staple (secrets required, docs/09 §7.4)",
        "Windows authenticity (unsigned by decision, ADR-033)",
        "three-image tar contents (archives are passed through unchanged)",
    ],
    "note": "Assembled from real inputs; `zk channel verify` is executed by build-dist.sh after this file is written.",
}
(out / "BUILD-STATUS.json").write_text(json.dumps(status, indent=2) + "\n")

# SHA256SUMS covers every delivered file (docs/09 §4).
sums = []
for path in sorted(out.rglob("*")):
    if not path.is_file() or path.name == "SHA256SUMS":
        continue
    sums.append(f"{sha256(path)}  {path.relative_to(out).as_posix()}")
(out / "SHA256SUMS").write_text("\n".join(sums) + "\n")
print(f"build-dist: assembled {out} ({len(assets)} client asset(s), {len(images)} image archive(s))")
PY
BUILD_RC=$?
set -e

case "$BUILD_RC" in
  0) ;;
  2) echo "build-dist: exit 2 (UNVERIFIED). Evidence: $OUT/UNVERIFIED.json" >&2; exit 2 ;;
  *) echo "build-dist: exit 1 (invalid input). No deliverable dist tree was produced." >&2; exit 1 ;;
esac

# Real verification gate: CHANNEL/CLIENT-RELEASE/SHA256SUMS/SBOM/assets/brand residue.
ZK_BIN="${ZK_BIN:-target/debug/zk}"
if [[ ! -x "$ZK_BIN" ]]; then
  cargo build -q -p zk
  ZK_BIN="target/debug/zk"
fi
"$ZK_BIN" channel verify --channel "$CHANNEL" --dist "$OUT"
echo "build-dist: ok $OUT (edition=$EDITION channel=$CHANNEL version=$VERSION)"
echo "build-dist: release signing/notarize/dist-content rules remain UNVERIFIED — see $OUT/BUILD-STATUS.json"
