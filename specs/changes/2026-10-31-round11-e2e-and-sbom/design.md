# 2026-10-31-round11-e2e-and-sbom · 设计

- 状态：`approved`

## 1. schedule e2e
独立 playwright config webServer entry or shell script launches serve; spec uses env ZEOKEEL_SCHEDULE_URL. Reuse serve-m1 fake upstream python; schedules.json format from automation.rs.
## 2. async turn
agent-rig test uses existing fake model builder; tool call sequence via queues; assert kit registry/list.
## 3. sbom
`zk sbom`: `Command::new("cargo").args(["metadata","--format-version","1","--no-deps"])`? need all workspace deps licenses; `--no-deps` gives workspace packages only; full metadata offline may be large but okay. Generate minimal components from `Cargo.lock` (name+version) for all crates; license from cargo metadata package.license map. pnpm: parse package.json via serde_json. Deterministic serial number = UUID v5? avoid dependency; use fixed "urn:uuid:00000000-0000-0000-0000-000000000000" or sha256 of content. Use stable SHA-256 hex as serialNumber `urn:uuid:<uuid-like>`? CycloneDX serialNumber must be urn:uuid; can derive 32 hex from sha256 and format.
## 4. docs check
Python regex; command set; backlog keywords window.
