# 2026-10-30-oem-delivery · 验收（M5）

- 状态：`in-progress`（spec 阶段）
- 验收人：pending
- 证据目录：`target/evidence/m5/`

## 进展

- task 1：docs/09 schema 的 `ChannelFile`/identity 校验；`zk channel validate --index oem/fixtures --all` 通过。
- task 2：brand/渠道生成物；`compose --channel oem-demo --check` 生成后绿，`brand.ts` 含 `Acme Keel`/`oem-demo`。
- task 3：`oem/fixtures` index/.gitmodules/pin/manifest；pin=sha256(channel.json) 通过。
- task 4：`brands/oem-demo`；`bash scripts/oem-e2e.sh` 通过（含 hide 残留负向）。
- task 5/6/7 未开始。

## 进展（续）

- task 5：`scripts/build-dist.sh official 0.1.0 /tmp/zk-dist` 产出 CLIENT-RELEASE/CHANNEL/VERSION/SHA256SUMS/SBOM/deploy/HANDOVER；`zk channel verify` 通过（dry-run artifacts，无真实安装包/镜像）。
- task 6：`srv.updates` 包 + `/api/updates/manifest`/`client` + `/portal`；`scripts/updates-e2e.sh` 通过；`admin-e2e.sh` 10 passed（含 m5 portal + axe）。
- task 7：`scripts/upgrade-drill.sh --dry-run` 通过；真实 Docker 未验证（无 Docker）。
- task 8 未完成；M5 退出条件中的真实镜像/三平台安装包/公证/deep OEM 未完成。

## 检查

- [ ] `zk channel validate --index oem/fixtures --all` / `zk channel verify --channel official --dist <fixture>`
- [ ] `bash scripts/oem-e2e.sh`
- [ ] `bash scripts/upgrade-drill.sh`
- [ ] M5 门户/更新 e2e
- [ ] 签名/产物 dry-run 证据
- [ ] 独立评审 APPROVED

## 说明

无 Docker/证书/平台 runner 时，产物/签名/升级只算 dry-run；必须在 acceptance 标注“未验证”，不得写 complete。
