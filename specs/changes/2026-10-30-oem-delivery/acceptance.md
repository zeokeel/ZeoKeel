# 2026-10-30-oem-delivery · 验收（M5）

- 状态：`in-progress`（spec 阶段）
- 验收人：pending
- 证据目录：`target/evidence/m5/`

## 检查

- [ ] `zk channel validate --index oem/fixtures --all` / `zk channel verify --channel official --dist <fixture>`
- [ ] `bash scripts/oem-e2e.sh`
- [ ] `bash scripts/upgrade-drill.sh`
- [ ] M5 门户/更新 e2e
- [ ] 签名/产物 dry-run 证据
- [ ] 独立评审 APPROVED

## 说明

无 Docker/证书/平台 runner 时，产物/签名/升级只算 dry-run；必须在 acceptance 标注“未验证”，不得写 complete。
