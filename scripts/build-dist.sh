#!/usr/bin/env bash
# ZeoKeel M5/dist tree builder — NEVER fabricates artifacts.
#
# Contract (docs/09 §4/§5, docs/11 §5, docs/37):
#   * The dist tree is assembled only from REAL build outputs supplied by the caller:
#       --artifacts <dir>         real per-platform client installers (AppImage/deb/rpm/dmg/exe/msi)
#       --images <dir>            real `docker save`d image archives (enterprise only; docs/09 §5: 3 images)
#       --sbom <file>             real CycloneDX SBOM (zk sbom / scripts/sbom.sh output)
#       --known-limitations <f>   real KNOWN-LIMITATIONS.md (required for a deliverable dist)
#   * Fixtures/placeholders can never pass: input directories under deploy/ (e.g.
#     deploy/updates/, the 18-byte AppImage + CLIENT-RELEASE.json update-surface fixture),
#     under oem/, or containing a fixtures/ path segment are rejected as INVALID. Stray
#     manifest/checksum files inside --artifacts are rejected as INVALID (not installers).
#   * If any required real input is missing/incomplete the script writes an explicit
#     UNVERIFIED.json and exits 2. It NEVER writes CLIENT-RELEASE.json / SHA256SUMS
#     when the inputs are placeholders, so `zk channel verify` and the release upload
#     cannot mistake a dry-run for a deliverable.
#   * Exit codes: 0 = dist assembled and verified by `zk channel verify`;
#                 1 = invalid input (placeholder/fixture/symlink/unknown channel/illegal edition);
#                 2 = UNVERIFIED (real artifact/image/SBOM/limitations inputs not available).
#   * `--self-test` exercises the negative paths (fixtures => 1, missing inputs => 2) and
#     never builds/verifies real artifacts; CI runs it before assembling a release dist.
#
# Signed CLIENT-RELEASE manifests and notarized installers are produced by the release
# environment (docs/09 §7.4); this script records them as unverified in BUILD-STATUS.json.
set -euo pipefail
cd "$(dirname "$0")/.."

usage() {
  cat <<'EOF'
usage: scripts/build-dist.sh --channel <official|oem-*> --version <x.y.z[-pre]> \
         --edition <personal|enterprise> --artifacts <dir> \
         --sbom <cyclonedx.json> --known-limitations <KNOWN-LIMITATIONS.md> \
         [--images <dir>] [--out <dir>]

  --artifacts           directory containing real client installers (required)
  --images              directory containing real `docker save` archives (required for enterprise)
  --sbom                real CycloneDX SBOM (required)
  --known-limitations   real KNOWN-LIMITATIONS.md (required; docs/37)
  --out                 output directory (default: dist/<version>/<channel>; must not exist)
  --self-test           run negative-path checks only (fixtures => 1, missing inputs => 2)

  Inputs under deploy/, oem/ or any */fixtures/ path are test fixtures and are rejected.
exit: 0 verified, 1 invalid input, 2 UNVERIFIED (missing real inputs)
EOF
}

self_test() {
  command -v python3 >/dev/null || { echo "build-dist --self-test: python3 required" >&2; exit 1; }
  local tmp failures=0
  tmp="$(mktemp -d)"
  python3 - "$tmp" <<'PY'
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
fake = root / "fake-artifacts"
fake.mkdir()
# The exact shape of the committed deploy/updates fixture: a tiny "installer".
(fake / "ZeoKeel_0.1.0_linux_x64.AppImage").write_bytes(b"dry-run placeholder\n")
(fake / "CLIENT-RELEASE.json").write_text('{"schema":1,"assets":[]}\n')

real = root / "real-artifacts"
real.mkdir()
elf = bytearray(b"\x7fELF" + b"\x00" * (2 * 1024 * 1024 - 4))
elf[4] = 2  # 64-bit
elf[5] = 1  # little endian
(real / "zeokeel-personal-0.0.0-linux-x64.AppImage").write_bytes(bytes(elf))

(root / "sbom.json").write_text(json.dumps({
    "bomFormat": "CycloneDX",
    "specVersion": "1.5",
    "version": 1,
    "metadata": {"component": {"type": "application", "name": "ZeoKeel"}},
    "components": [{"type": "library", "name": "zk-kit", "version": "0.0.0"}],
}))
(root / "KNOWN-LIMITATIONS.md").write_text(
    "# Known limitations\n\n"
    "- build-dist self-test fixture, not a deliverable; this file only exists to exercise\n"
    "  the real-input checks and is longer than the placeholder threshold on purpose.\n"
)
PY
  expect() {
    local want="$1" rc
    shift
    set +e
    "$@" >"$tmp/last.log" 2>&1
    rc=$?
    set -e
    if [ "$rc" -ne "$want" ]; then
      echo "build-dist self-test: FAILED (expected exit $want, got $rc): $*" >&2
      sed 's/^/    /' "$tmp/last.log" >&2
      failures=$((failures + 1))
    else
      echo "build-dist self-test: ok (exit $want): $*"
    fi
  }
  # 1) 18-byte placeholder AppImage + stray manifest => INVALID (exit 1).
  expect 1 "$0" --channel official --version 0.0.0-self --edition personal \
    --artifacts "$tmp/fake-artifacts" --out "$tmp/out-1"
  # 2) the committed update-surface fixture directory is rejected as a build input (exit 1).
  expect 1 "$0" --channel official --version 0.0.0-self --edition personal \
    --artifacts deploy/updates/clients --out "$tmp/out-2"
  # 3) plausible installer + real SBOM but no real image archives => UNVERIFIED (exit 2).
  expect 2 "$0" --channel official --version 0.0.0-self --edition enterprise \
    --artifacts "$tmp/real-artifacts" --sbom "$tmp/sbom.json" \
    --known-limitations "$tmp/KNOWN-LIMITATIONS.md" --out "$tmp/out-3"
  # 4) plausible installer + real SBOM but no KNOWN-LIMITATIONS => UNVERIFIED (exit 2).
  expect 2 "$0" --channel official --version 0.0.0-self --edition personal \
    --artifacts "$tmp/real-artifacts" --sbom "$tmp/sbom.json" --out "$tmp/out-4"
  rm -rf "$tmp"
  if [ "$failures" -ne 0 ]; then
    echo "build-dist: self-test failed ($failures case(s))" >&2
    exit 1
  fi
  echo "build-dist: self-test ok (placeholder=1, fixture=1, missing images=2, missing limitations=2)"
  exit 0
}

CHANNEL=""; VERSION=""; EDITION=""; ARTIFACTS=""; IMAGES=""; SBOM=""; KNOWN_LIMITATIONS=""; OUT=""; SELFTEST=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --channel) CHANNEL="${2:?--channel needs a value}"; shift 2 ;;
    --version) VERSION="${2:?--version needs a value}"; shift 2 ;;
    --edition) EDITION="${2:?--edition needs a value}"; shift 2 ;;
    --artifacts) ARTIFACTS="${2:?--artifacts needs a value}"; shift 2 ;;
    --images) IMAGES="${2:?--images needs a value}"; shift 2 ;;
    --sbom) SBOM="${2:?--sbom needs a value}"; shift 2 ;;
    --known-limitations) KNOWN_LIMITATIONS="${2:?--known-limitations needs a value}"; shift 2 ;;
    --out) OUT="${2:?--out needs a value}"; shift 2 ;;
    --self-test) SELFTEST=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "build-dist: unknown argument: $1" >&2; usage >&2; exit 1 ;;
  esac
done

if [[ "$SELFTEST" == "1" ]]; then
  self_test
fi

if [[ -z "$CHANNEL" || -z "$VERSION" || -z "$EDITION" ]]; then
  echo "build-dist: --channel/--version/--edition are required" >&2
  usage >&2
  exit 1
fi
# Release versions are semver including pre-release suffixes (the target is a -rc train).
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] \
  || { echo "build-dist: bad version: $VERSION (expected x.y.z or x.y.z-pre)" >&2; exit 1; }
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
python3 - "$CHANNEL" "$VERSION" "$EDITION" "$OUT" "$ARTIFACTS" "$IMAGES" "$SBOM" "$KNOWN_LIMITATIONS" <<'PY'
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


def fixture_reason(arg: str) -> str | None:
    """Reject test fixtures (deploy/updates, oem/fixtures, */fixtures/) as build inputs."""
    try:
        rel = pathlib.Path(arg).resolve().relative_to(pathlib.Path.cwd().resolve())
    except ValueError:
        return None
    parts = rel.parts
    if not parts:
        return f"{rel}: the repository root is not a build-output directory"
    if parts[0] == "deploy":
        return f"{rel}: deploy/ holds deployment config and the deploy/updates test fixture, not real build output"
    if parts[0] == "oem":
        return f"{rel}: oem/ holds OEM fixtures, not real build output"
    if "fixtures" in parts:
        return f"{rel}: */fixtures/ paths are test fixtures, not real build output"
    return None


channel, version, edition, out_arg, artifacts_arg, images_arg, sbom_arg, limitations_arg = sys.argv[1:9]
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
    reason = fixture_reason(arg)
    if reason is not None:
        invalid.append(reason)
        return None
    root = pathlib.Path(arg)
    if not root.is_dir():
        unverified.append(f"--{label} directory missing: {arg}")
        return None
    return root


INSTALLER_SUFFIXES = {".appimage", ".deb", ".rpm", ".dmg", ".exe", ".msi"}


def real_files(root: pathlib.Path, label: str, installers_only: bool) -> list[pathlib.Path]:
    files = []
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            invalid.append(f"symlink not allowed in build inputs: {path}")
            continue
        if path.is_file():
            if installers_only and path.suffix.lower() not in INSTALLER_SUFFIXES:
                invalid.append(
                    f"--{label} contains {path.name}: not a client installer "
                    f"(allowed: {' '.join(sorted(INSTALLER_SUFFIXES))}); manifests/checksums are not deliverables"
                )
                continue
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


out_reason = fixture_reason(out_arg)
if out_reason is not None:
    invalid.append(out_reason)

artifacts_root = require_dir(artifacts_arg, "artifacts")
clients: list[pathlib.Path] = []
seen_names: set[str] = set()
if artifacts_root is not None:
    clients = real_files(artifacts_root, "artifacts", installers_only=True)
    if not clients:
        unverified.append(f"--artifacts {artifacts_arg} contains no real client installers")
    for client in clients:
        if client.name in seen_names:
            invalid.append(f"duplicate client artifact name: {client.name}")
        seen_names.add(client.name)
        plausible_installer(client)

images_root = require_dir(images_arg, "images") if edition == "enterprise" else None
images: list[pathlib.Path] = []
if edition == "enterprise":
    if images_root is not None:
        images = real_files(images_root, "images", installers_only=False)
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
    if fixture_reason(sbom_arg) is not None:
        invalid.append(fixture_reason(sbom_arg) or "fixture path")
    elif not sbom_path.is_file():
        unverified.append(f"--sbom file missing: {sbom_arg}")
    else:
        sbom_data = validate_sbom(sbom_path)

limitations_path = None
if not limitations_arg:
    unverified.append(
        "--known-limitations not provided (KNOWN-LIMITATIONS.md is required in the dist tree, docs/37)"
    )
else:
    reason = fixture_reason(limitations_arg)
    if reason is not None:
        invalid.append(reason)
    else:
        limitations_path = pathlib.Path(limitations_arg)
        if not limitations_path.is_file():
            unverified.append(f"--known-limitations file missing: {limitations_arg}")
            limitations_path = None
        elif limitations_path.stat().st_size < 100:
            invalid.append("KNOWN-LIMITATIONS is too small to be a real limitations document")
            limitations_path = None

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

assert artifacts_root is not None and sbom_data is not None and limitations_path is not None
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

# KNOWN-LIMITATIONS.md is part of the delivery (docs/37). On a powered_by=hide channel the
# delivered text must not carry the vendor product string, so white-label it like the rest.
limitations = limitations_path.read_text()
if powered_by == "hide":
    limitations = limitations.replace("ZeoKeel", product_name)
(out / "KNOWN-LIMITATIONS.md").write_text(limitations if limitations.endswith("\n") else limitations + "\n")

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
        "known_limitations": sha256(limitations_path),
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
print(f"build-dist: assembled {out} ({len(assets)} client asset(s), {len(images)} image archive(s), KNOWN-LIMITATIONS included)")
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
