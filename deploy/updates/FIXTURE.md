# deploy/updates — TEST FIXTURE ONLY (not a deliverable)

This directory is the **committed update-surface fixture** used by tests that need a
1:1 client update manifest without a release environment:

- `scripts/updates-e2e.sh` serves it as `ZEOKEEL_UPDATE_DIR=deploy/updates` (manifest,
  channel/version rejection, artifact download, portal);
- `scripts/upgrade-drill.sh` uses `CLIENT-RELEASE.json` as the local fallback manifest
  when no server URL is supplied.

Everything here is deliberately **not** a release artifact:

- `clients/ZeoKeel_0.1.0_linux_x64.AppImage` is an 18-byte placeholder;
- `CLIENT-RELEASE.json` pins the placeholder's SHA-256 on purpose;
- `CHANNEL.json` is a static fixture (`official`, version-free).

Release tooling must never consume this directory:

- `scripts/build-dist.sh` rejects any `--artifacts`/`--images`/`--sbom`/`--known-limitations`
  input located under `deploy/` (or `oem/`, or any `*/fixtures/` path) as **INVALID (exit 1)**
  and only writes a real `CLIENT-RELEASE.json`/`SHA256SUMS` when real inputs are supplied;
- `.github/workflows/release.yml` assembles its dist from real per-OS installers +
  `docker save` archives + `zk sbom` output + `KNOWN-LIMITATIONS.md`, never from this fixture;
- `scripts/build-dist.sh --self-test` includes the negative case:
  `--artifacts deploy/updates/clients` must exit 1.

The fixture versions (`0.1.0`) are intentionally frozen to keep the update-surface tests
independent from the release version. Do not copy real release artifacts into this directory.
