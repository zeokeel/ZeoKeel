# 2026-10-30 · M5 OEM/交付实现对抗性验证

- **评审对象**：`tools/zk` channel validate/verify、`brands/{official,oem-demo}`、`oem/fixtures`、`packages/server/updates`、`scripts/{oem-e2e,updates-e2e,build-dist,upgrade-drill,sbom}.sh`、`deploy/{updates,portal}`、`e2e/m5-portal.spec.ts`、`.github/workflows/release.yml`、docs/09/23/27、M5 spec/tasks/acceptance
- **评审者**：audit-arch（独立只读；仅写本记录）
- **结论**：CHANGES-REQUESTED
- **关联 spec**：`specs/changes/2026-10-30-oem-delivery`
- 证据日期：2026-10-30；本记录为唯一新增文件，未改产品代码/spec。

## 1. 结论

**CHANGES REQUIRED（CHANGES-REQUESTED）。**

M5 的 channel validate/verify、OEM fixture、updates 端点、dist 脚本和 portal e2e 有 happy path，但实现与 spec 的硬承诺有重大偏差：

- **OEM 门户硬编码 `ZeoKeel`/official**，`powered_by=hide` 会泄漏厂商品牌；
- **1:1 更新没有拒绝版本不匹配**，只把 `forced` 置真；
- **品牌残留门禁只扫文本**，安装包/镜像二进制里的厂商串会漏过；
- **deep OEM 只有 fixture 文件存在性检查**，`[packages]` 为空，未 compose/plan/换包；
- **`channel_verify` 只查三个文件存在**，不校验 SHA/CLIENT-RELEASE/SBOM，`build-dist` 的假产物能通过。

## 2. 证据命令与结果

| 命令 | 结果 |
| --- | --- |
| `bash scripts/oem-e2e.sh` | **PASS**：`channel validate ok: 2 channels`、`verify ok`、compose、brand/residue；但 deep 未 compose，residue 只文本 |
| `bash scripts/updates-e2e.sh` | **PASS**：same-version manifest/channel reject/artifact；无 version mismatch 1:1 测试 |
| `bash scripts/admin-e2e.sh` | **PASS**：10 passed（含 M5 portal，official only） |
| `bash scripts/upgrade-drill.sh --dry-run` | **PASS（dry-run）**：manifest 连接失败仍继续，`docker load` 模拟，`/readyz` 本地 JSON；非真实演练 |
| `bash scripts/build-dist.sh official 0.1.0 /tmp/zk-dist` | **PASS**：生成 dist；但客户端是 18B 文本、`CLIENT-RELEASE.assets=[]`、`sha256=""`、SBOM=README 占位 |
| `cargo test --workspace --offline` | **PASS** |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | **PASS** |
| `zk validate` | **PASS**：70 packages scanned |
| `zk compose --edition enterprise --profile default --channel official --check` | **PASS**：16 files |
| `zk channel validate --index oem/fixtures --all` | **PASS**：2 channels |
| `schema-check` / `zk db lock --check` | **PASS** |

## 3. P0（阻塞）

### P0-1 · OEM 门户硬编码 ZeoKeel/official，`powered_by=hide` 泄漏品牌

- `packages/server/updates/src/lib.rs:39-47` 的 `portal()` 标题/`<h1>`/下载名硬编码 `ZeoKeel 下载`/`ZeoKeel 客户端下载`，只从 `CLIENT-RELEASE.json` 取 channel/version，不读 `CHANNEL.json`/brand。
- `updates-e2e.sh` 也 `grep 'ZeoKeel 客户端下载'`，等于把泄漏固化成验收；admin-e2e 的 `m5-portal.spec.ts` 只跑 official。
- 对 `oem-demo`（`brand.powered_by=hide`）会直接显示厂商名，违反 M5 退出“hide=零厂商串”与 docs/09/27。

**修法**：portal 从 `CHANNEL.json`/channel brand 读取 `product_name`、`powered_by`、logo、平台列表；`hide` 时禁止出现 ZeoKeel；补 OEM portal e2e/axe/品牌断言；`updates-e2e` 用 oem-demo 负向。

### P0-2 · 1:1 更新未拒绝版本不匹配

- `updates/src/lib.rs:60-68`：`forced = client_version != current_client`，仍然返回 200 清单；ADR-041/docs/04/M5 spec 要求 `client_version` 必须等于 `CLIENT-RELEASE.json`，不匹配应拒绝整份清单（或 426/min_supported）。
- `updates-e2e.sh` 只发 `0.1.0`（与 release 相同），没有 `0.0.9`/`9.9.9` 负向；`updates` 的 assets `sha256=""`，下载也不校验 SHA。

**修法**：版本不等 → 409/426 并返回 `CLIENT_TOO_OLD`/`UPDATE_VERSION_MISMATCH`；补 old/new/below-min 三种 e2e；下载前按 `CLIENT-RELEASE.assets[].sha256` 校验（空 sha 构建红）。

### P0-3 · 品牌残留门禁只扫文本，二进制/安装包漏网；`channel_verify` 未校验产物内容

- `tools/zk/src/main.rs:1054-1071` `channel_verify` 只 `fs::read_to_string` 扫文本，跳过 `LICENSE/NOTICE/CHANNEL.json/SHA256SUMS`；**不扫二进制 strings/镜像层/安装包**，也不校验 `CHANNEL.json` 的 channel、`CLIENT-RELEASE` 的 version/assets、`SHA256SUMS` 复算、SBOM。
- `oem-e2e.sh` 只写 `leak.txt` 测文本；一个 OEM AppImage/镜像含 `ZeoKeel` 仍能过 `channel_verify`。`build-dist` 的假文件也能过。
- 这使 M5 退出“hide 零厂商串”和产物级白标门禁变成 false green。

**修法**：`channel verify` 必须：解包安装包/镜像字符串扫描（含 ELF/Mach-O/PE/AppImage/tar）、按生成指纹 allowlist、校验 `CHANNEL.json` channel、`CLIENT-RELEASE` version/assets/sha、`SHA256SUMS` 复算、SBOM 存在；补二进制泄漏负向/正例。

### P0-4 · deep OEM 未真正 compose/换包，fixture 只查文件存在

- `oem/fixtures/customers/oem-deep/oem.toml` 的 `[packages]` 为空；`channel_validate`（`tools/zk/src/main.rs:1035-1049`）只检查 `packages/*/{zk.toml,README.md,src/lib.rs,tests/}` 是否存在，不解析 `zk.toml` role/contract、不跑 conformance、不 `zk plan`/`compose --check`。
- `oem-e2e.sh` 没有 deep 渠道的 compose/plan/换包矩阵；M5 退出“深度 OEM 代码包通过契约 conformance + 换包矩阵”未验证。

**修法**：`oem.toml [packages]` 写实际 role→package；`channel validate` 解析四件套/契约/role，`zk plan`/`compose --check` 证明换包；补 deep conformance/换包 e2e。

## 4. P1（应修）

1. **`channel_validate` pin 不是 gitlink**：`main.rs:1026` 用 `sha256(channel.json)` 当 fixture pin；没有读 `.gitmodules`/submodule gitlink，真实仓无法证明三处一致。
2. **`channel_validate` 缺 logo/id/powered_by 校验**：没有 id 正则/长度；logo 路径安全/256KB/script/外链；`powered_by` enum；未知键拒绝；这些只在 `load_channel` 的部分字段里，未覆盖全渠道。
3. **`channel_verify`/`build-dist` 假产物**：`CLIENT-RELEASE.assets=[]`、sha256 空、SBOM/ deploy 占位；`build-dist` 生成 18B 文本 AppImage；不能证明三平台安装包/SBOM。
4. **`updates` 清单/下载缺 SHA**：`assets[].sha256=""`；下载无校验；portal 链接仅 Linux。
5. **portal 只 official/Linux**：无 OEM 品牌、无 Windows/macOS、无 platforms 收窄；`deploy/portal/index.html` 与 server `/portal` 两份且都硬编码 ZeoKeel。
6. **`upgrade-drill --dry-run` 在 manifest 不可用时仍 exit 0 并说 ok**；`--docker` 分支只 echo，不执行 docker load/compose/backup；不能作为升级演练证据。
7. **CI release.yml 不产出三平台安装包**：只 `cargo build` + `build-dist` 假产物；notarize job 只有 echo；无 Windows/macOS/Linux 真实 bundle、无签名/公证命令。
8. **docs/23 仍写 `brand.name`**（docs/09 用 `identity.product_name`）；SSOT 冲突；docs/23 应随 M5 回写。
9. **`updates-e2e` 无 version 1:1 负向、无 OEM portal、无 SHA 校验；`oem-e2e` 无 deep。
10. **`oem/fixtures` pin 与 `.gitmodules` 未交叉校验**；fixture 模拟范围需明确。

## 5. P2（建议）

- `deploy/portal/index.html` 与 server `/portal` 双实现；统一或删除静态页。
- `build-dist` 客户端命名 `official-0.1.0-linux.AppImage` 与 docs/09 命名不一致。
- `SBOM` 用 `README` 占位；CI 只生成 `dist/sbom.json`，未按渠道进 dist。
- `release.yml` 的 notarize job 无实际 `notarytool`/staple；macOS 证书后启用时需真实命令。
- `upgrade-drill` 的 `sha256sum` 只生成未比较；应对比 `CLIENT-RELEASE`/`SHA256SUMS`。
- `channel verify` 文本扫描跳过 `CHANNEL.json` 整文件，allowlist 应逐字段。
- `oem-demo` fixture `powered_by=hide` 的 `CHANNEL.json`/portal 未测。
- `channel validate` 的 `manifest` 缺失时静默通过（只 if is_file）；应要求必填。

## 6. M5 退出条件核对（dry-run 边界）

| docs/12 M5 退出 | 当前 |
| --- | --- |
| demo 客户全流程 → 镜像 + 三平台安装包 | **UNVERIFIED**：无真实镜像/三平台包；build-dist 假产物 |
| powered_by 品牌残留 hide/show | **PARTIAL/FAIL**：文本扫描有；二进制/安装包/portal 未覆盖 |
| deep OEM conformance + 换包矩阵 | **FAIL/UNVERIFIED**：`[packages]` 空，无 compose/plan |
| macOS 公证 CI 就绪 + Windows SHA | **UNVERIFIED**：release.yml 只有 echo/SHA 文档；无 notarytool/staple |
| 门户下载 + 企业版 1:1 + 个人版 GitHub | **PARTIAL/FAIL**：portal official 硬编码；1:1 未拒绝；个人版 GitHub 未实现 |
| 服务端升级演练 | **UNVERIFIED**：dry-run 在 manifest 不可用时仍 ok；--docker 假路径 |

**明确未验证（无 Docker/证书/runner）**：真实 docker save/load/compose、三平台 Tauri bundle、macOS 公证/staple、Windows 安装包 SHA、真实升级回滚、真实 OEM 镜像/安装包、SBOM CycloneDX 产物。dry-run 不能标记完成。

## 7. 结论

- **P0：4**：portal 品牌泄漏、1:1 未拒绝、残留/产物 false gate、deep OEM 未 compose。
- **P1：10**：pin/logo/id、假产物/SHA/SBOM、portal 平台、upgrade/CI、docs/23、e2e 缺口。
- **P2**：见上。
- **M5 实现不能 APPROVED**：修 P0/P1，补真实/明确标注 dry-run 证据后再复评。

- 关联 spec：`specs/changes/2026-10-30-oem-delivery`。
- 评审结论：**CHANGES-REQUESTED**
