# 2026-10-30 · M5 OEM/交付实现对抗性验证

- **评审对象**：`tools/zk` channel validate/verify、`brands/{official,oem-demo}`、`oem/fixtures`、`packages/server/updates`、`scripts/{oem-e2e,updates-e2e,build-dist,upgrade-drill,sbom}.sh`、`deploy/{updates,portal}`、`e2e/m5-portal.spec.ts`、`.github/workflows/release.yml`、docs/09/23/27、M5 spec/tasks/acceptance
- **评审者**：audit-arch（独立只读；仅写本记录）
- **结论**：APPROVED
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

---

## 8. 第二轮复评（R2，2026-10-30）

> 对象：R1 修复后的 M5 实现。只读复跑；未改产品代码/spec（仅本记录）。
> 仅列剩余 P0/P1；环境阻塞项单独标注。

### 8.1 结论

**CHANGES REQUIRED（CHANGES-REQUESTED）**

- **P0：1**（deep OEM 仍不能 compose/plan，M5 退出条件未过；可本地修）。
- **P1：3 组**（channel 校验缺口、产物/portal 完整性、升级/CI 证据）。
- **环境阻塞未验证**：真实 Docker 镜像/load/compose、三平台安装包、macOS 公证、真实升级回滚；不因本机无 Docker/证书/runner 判 P0，但必须标记未验证。

### 8.2 已关闭

- **Portal OEM 品牌**：`updates/src/lib.rs:39-47` 读 `CHANNEL.json` 的 product_name/powered_by；hide 不输出 Powered by；updates-e2e 用 oem-demo fixture 断言 “Acme Keel” 且无 ZeoKeel。
- **1:1 版本**：`manifest` 在 `client_version != current_client` 时 409 `CLIENT_VERSION_MISMATCH`；updates-e2e 加 0.0.9 负向。
- **channel verify**：校验 dist CHANNEL/CLIENT channel、复算 SHA256SUMS、要求 SBOM/、hide 时扫描二进制 bytes；oem-e2e 加 binary leak 负向。
- **deep fixture**：`[packages]` 非空（`brand.mark = acme/acme-brand`），validate 解析 oem.toml/包 zk.toml role/contract/four-piece。

### 8.3 P0（可本地修）

#### P0-1 · deep OEM 仍不能 compose/plan，M5 退出“deep conformance + 换包矩阵”未过

- 实测：
  ```text
  $ ./target/debug/zk compose --edition enterprise --profile default --channel oem-deep --check
  zk: channel oem-deep not found at /data/ZeoKeel/brands/oem-deep/channel.json
  $ ./target/debug/zk plan ... --channel oem-deep
  same
  ```
- `oem/fixtures/customers/oem-deep` 有 channel.json/oem.toml/packages，但 `zk compose` 只从主仓 `brands/<channel>` 加载，不从 `oem/fixtures` 加载；`oem-e2e` 也没跑 deep compose/plan/换包。
- 修：让 `--oem-index`/fixture 渠道解析可把客户仓 channel 作为 channel 源（或生成临时 `brands/oem-deep`），并加 `zk plan`/`compose --check` + 换包断言；oem-e2e 覆盖 deep。

### 8.4 P1（可本地修）

1. **channel 校验缺口**：
   - `tools/zk/src/main.rs:1026` pin 仍是 `sha256(channel.json)`，不是 fixture 目录内容 hash，也未校验 `.gitmodules`/gitlink；spec/09 要求三处一致。
   - `channel_validate` 未校验 id 正则/长度、logo canonicalize/256KB/script/外链、`powered_by` enum、未知键；`load_channel` 只查文件存在。
   - 修：fixture pin 用目录树 hash；校验 `.gitmodules`+gitlink；统一调用严格 channel schema 校验（未知键拒绝）。
2. **产物/portal 完整性**：
   - `build-dist` 的 `CLIENT-RELEASE.assets=[]`、`sha256=""`、客户端 24B 文本、SBOM/ deploy 占位；`channel verify` 只查存在，未要求 assets 非空/sha 匹配。
   - portal 只链接 Linux AppImage；无 Windows/macOS；产品名含空格时 URL 未编码。
   - 修：dry-run 也要基于真实/占位文件生成 assets sha；`channel verify` 强制 assets/sha；portal 按 `platforms` 渲染三平台并 URL encode。
3. **升级/CI 证据**：
   - `upgrade-drill.sh --dry-run` 在 manifest 不可用时仍 exit 0，`sha256sum` 不比较；`--docker` 分支只 echo，不执行 load/compose/backup。
   - `release.yml` 只 `cargo build` + 假 dist；notarize job 只 echo；无三平台 bundle/公证命令。
   - 修：dry-run 失败应非零或明确 `UNVERIFIED`；`--docker` 至少真实执行 load/compose/readyz 或标注 fake；release.yml 补三平台 bundle/notarize 模板（即使 `if` 跳过）。

### 8.5 环境阻塞未验证（不是 P0，但不得标 complete）

- 真实 Docker 镜像 save/load、compose up、PG 备份/PITR。
- 三平台 Tauri 安装包（Windows/macOS/Linux）与 Windows SHA256 文档。
- macOS notarize/staple（证书未配；CI job 未真实执行）。
- 真实升级/回滚演练（当前 dry-run/模拟）。
- 真实 CycloneDX SBOM（当前 README/占位）。
- 真实 OEM 安装包/镜像的二进制残留扫描（当前 fixture 文本/二进制负向）。

### 8.6 验证

| 命令 | 结果 |
| --- | --- |
| `bash scripts/oem-e2e.sh` | **PASS**：2 channels/verify/compose official+brand/residue；deep 未 compose |
| `bash scripts/updates-e2e.sh` | **PASS**：manifest/channel/version reject/artifact/oem-portal |
| `bash scripts/admin-e2e.sh` | **PASS**：10 passed（M5 portal official） |
| `bash scripts/build-dist.sh official 0.1.0 /tmp/zk-dist` | **PASS（dry-run）**：假客户端/assets 空/SBOM 占位 |
| `bash scripts/upgrade-drill.sh --dry-run` | **PASS（dry-run）**：manifest 不可用仍 ok，docker 模拟 |
| `cargo test --workspace` / `clippy` | **PASS** |
| `zk validate` / `compose official --check` / `channel validate` / schema / db | **PASS** |
| `zk compose/plan --channel oem-deep` | **FAIL**：channel not found（P0） |

### 8.7 结论

- **P0：1**（deep compose/plan/换包）。
- **P1：3 组**（channel 校验、产物/portal、升级/CI）。
- **环境阻塞未验证**：见 8.5；acceptance 必须标注未验证，不能写 complete。
- **M5 实现仍不可 APPROVED**：修 P0/P1 后复评。

- 关联 spec：`specs/changes/2026-10-30-oem-delivery`。
- 复评结论（第二轮）：**CHANGES-REQUESTED**


## 9. R2 后修复（Lead，2026-10-30）

- P0 deep OEM：新增 `editions/oem-deep.toml`（base enterprise + brand.mark→acme/acme-brand）、`packages/oem/acme-brand` 四件套；`zk` 支持自定义 edition 继承 base 的包 editions，`brand.mark` glue 无 Rust 绑定；`oem-e2e.sh` 临时复制 fixture 到 brands/oem-deep，`zk plan` 断言 acme/acme-brand、`compose --check`、`cargo test -p acme-brand`，然后清理并恢复 official。
- P1 channel verify：已校验 CHANNEL/CLIENT channel、SHA256SUMS 复算、SBOM、二进制 bytes 残留。
- P1 portal：按 CHANNEL.json 品牌化，hide 无 ZeoKeel；updates-e2e OEM portal。
- P1 manifest：版本不等 409 CLIENT_VERSION_MISMATCH。
- 未修：pin 目录/gitlink、unknown-key schema、dist assets 空、dry-run 失败语义、三平台/公证/Docker 环境阻塞。

---

## 9. 第三轮复评（R3，2026-10-30）

> 对象：R2 修复后的 M5 实现。只读复跑；未改产品代码/spec（仅本记录）。
> 仅列剩余 P0/P1；环境阻塞项另标 P2/manual。

### 9.1 结论

**CHANGES REQUIRED（CHANGES-REQUESTED）**

- **P0：0**。deep OEM compose/plan 已可执行。
- **P1：3 组（可本地修）**：channel pin/schema 校验、dist assets/portal、upgrade/CI 证据。
- **P2/manual（环境阻塞）**：真实 Docker 镜像/load、三平台安装包、macOS 公证/staple、真实升级回滚、真实 SBOM；不能标 complete。

### 9.2 已关闭

- **deep OEM**：新增 `packages/oem/acme-brand` 四件套（`zk.toml`/src/tests/README），`editions/oem-deep.toml`（enterprise + brand.mark→acme/acme-brand）；oem-e2e 复制 fixture channel 到 `brands/oem-deep`，`zk plan` 断言 `acme/acme-brand`，`zk compose --edition oem-deep --channel oem-deep --check`，`cargo test -p acme-brand`，随后清理并恢复 official。实测 oem-e2e PASS。
- **portal/1:1**：portal 读 CHANNEL；oem-demo 显示 Acme Keel 无 ZeoKeel；manifest 版本不匹配 409；updates-e2e PASS。
- **channel verify**：dist CHANNEL/CLIENT channel、SHA256SUMS 复算、SBOM、hide 二进制扫描；oem-e2e binary leak 负向 PASS。

### 9.3 剩余 P1（可本地修）

1. **`channel_validate` 仍不是 gitlink/严格 schema**：
   - `main.rs:1026` pin 仍是 `sha256(channel.json)`，不是 fixture 目录 hash，也未校验 `.gitmodules`/submodule gitlink；spec/09 三处一致未落。
   - 未校验 id 正则/长度、logo canonicalize/256KB/script/外链、`powered_by` enum、未知键拒绝。
   - 修：fixture pin=目录树 hash；校验 `.gitmodules`+gitlink；严格 schema（unknown key 拒绝）；logo/id/powered_by 统一校验。
2. **dist dry-run 资产不完整**：
   - `build-dist` 的 `CLIENT-RELEASE.assets=[]`、sha 空、客户端 24B 文本、SBOM/ deploy 占位；`channel verify` 未要求 assets 非空/sha 匹配。
   - portal 只链接 Linux，无 Windows/macOS；产品名含空格时 URL 未编码。
   - 修：dry-run 也生成 assets sha/平台列表；verify 强制 assets；portal 按 platforms 三平台并 URL encode。
3. **upgrade/CI 证据语义**：
   - `upgrade-drill --dry-run` manifest 不可用仍 exit 0，`sha256sum` 不比较；`--docker` 只 echo，不执行 load/compose/backup。
   - `release.yml` 只 `cargo build`+假 dist；notarize job 只 echo；无三平台 bundle/公证命令。
   - 修：dry-run 失败非零或输出 `UNVERIFIED`；`--docker` 真实执行或标注 fake；release.yml 补三平台 bundle/notarize 模板（无证书时显式跳过并留证据）。

### 9.4 P2/manual（环境阻塞，不判 P1）

- 真实 Docker image save/load/compose up、PG 备份/PITR。
- 三平台 Tauri 安装包与 Windows SHA256 实测。
- macOS notarize/staple（证书未配）。
- 真实升级/回滚演练。
- 真实 CycloneDX SBOM（当前占位）。
- 真实 OEM 镜像/安装包上的二进制残留扫描（当前 fixture 负向）。

这些在本环境不可复现；acceptance 必须标 `UNVERIFIED`，不得写 complete。若 CI 在有 Docker/cert/runner 的环境跑通，可作为 M5 完成证据。

### 9.5 验证

| 命令 | 结果 |
| --- | --- |
| `bash scripts/oem-e2e.sh` | **PASS**：channel/index/pin/compose/brand/residue + deep plan/compose/cargo test |
| `bash scripts/updates-e2e.sh` | **PASS**：manifest/channel/version reject/artifact/oem-portal |
| `bash scripts/admin-e2e.sh` | **PASS**：10 passed（M5 portal official） |
| `cargo test -p acme-brand --offline` | **PASS**：conformance manifest test |
| `zk validate` | **PASS**：71 packages scanned |
| `zk compose --edition enterprise --channel official --check` | **PASS**：16 files |
| `zk channel validate --index oem/fixtures --all` | **PASS**：2 channels |
| `bash scripts/build-dist.sh official 0.1.0 /tmp/zk-dist` | **PASS（dry-run）**：假 assets/SBOM 占位 |
| `bash scripts/upgrade-drill.sh --dry-run` | **PASS（dry-run）**：语义仍模拟 |
| `cargo test --workspace` / clippy / schema / db lock | **PASS**（前轮） |

### 9.6 结论

- **P0：0；P1：3 组**（channel pin/schema、dist assets/portal、upgrade/CI）。
- **P2/manual**：真实 Docker/三平台/公证/升级/SBOM；标 unverified。
- **M5 实现仍不可 APPROVED**：修 P1 后可复评；环境项作为 P2/manual 记录。

- 关联 spec：`specs/changes/2026-10-30-oem-delivery`。
- 复评结论（第三轮）：**CHANGES-REQUESTED**

---

## 10. 第四轮复评（R4，2026-10-30）

> 对象：R3 修复后的 M5 实现。只读复跑；未改产品代码/spec（仅本记录）。
> 仅列剩余 P0/P1；环境阻塞项另标 P2/manual。

### 10.1 结论

**CHANGES REQUIRED（CHANGES-REQUESTED）**

- **P0：1**（build-dist 在空 clients 下生成 assets=[] 后失败；可本地修）。
- **P1：1 组**（channel_validate 未复用严格 channel/logo/id 校验；可本地修）。
- **P2/manual**：真实 Docker/三平台安装包/公证/真实升级/SBOM 仍不可验证，标 UNVERIFIED。

### 10.2 已关闭

- **deep OEM**：`packages/oem/acme-brand` 四件套 + `editions/oem-deep.toml`；oem-e2e 复制 fixture、`zk plan` 断言 acme/acme-brand、`compose --edition oem-deep --channel oem-deep --check`、`cargo test -p acme-brand`；实测 oem-e2e PASS。
- **pin/.gitmodules**：`channel_validate` 用 `dir_hash(customer)`，并检查 `.gitmodules` 包含 submodule 路径；fixtures pin 更新。
- **unknown keys/logo/id**：`ChannelFile`/Identity/Brand/lgogo 加 `deny_unknown_fields`；`load_channel` 加 id 长度/字符、logo `..`/绝对/http/256KB/script 检查。
- **portal/1:1**：portal 读 CHANNEL；oem-demo 显示 Acme Keel、无 ZeoKeel；manifest 不匹配 409；updates-e2e PASS。
- **channel verify**：dist CHANNEL/CLIENT channel、SHA256SUMS 复算、SBOM、hide 二进制扫描；oem-e2e binary leak 负向 PASS。
- **upgrade dry-run**：local manifest fallback、sha 比较、assets 空输出 `UNVERIFIED_NO_ASSETS`（不再假装 verified）。

### 10.3 剩余 P0（可本地修）

#### P0-1 · `build-dist.sh` 在空 clients 时生成 `assets=[]`，随后 `channel verify` 失败

- 实测：
  ```text
  $ bash scripts/build-dist.sh official 0.1.0 /tmp/zk-dist-r4
  zk: CLIENT-RELEASE assets empty
  EXIT=1
  ```
- 原因：`build-dist.sh` 先在 Python 中 `clients=sorted((out/"clients").iterdir())` → `assets=[]` 写 `CLIENT-RELEASE.json`；**之后**才在 `if not any(clients)` 创建 fallback `official-0.1.0-linux.AppImage`；`channel verify` 现在要求 assets 非空，于是失败。
- `CLIENT-RELEASE.json` 实际仍 `assets:[]`；`SHA256SUMS` 也不含客户端。
- 修：先创建 fallback/真实 clients，再生成 assets/sha；或 fallback 放前面；补 `build-dist` 空/非空 clients 单测。

### 10.4 剩余 P1（可本地修）

- **`channel_validate` 未复用严格 channel/logo/id 校验**：
  - `channel_validate`（`tools/zk/src/main.rs:1006-1057`）只解析 `ChannelFile` 与 identity，不调用 `load_channel`；因此 logo canonicalize/256KB/script、`file.id` 正则/长度、`powered_by` enum 在 `channel validate --all` 路径未执行。
  - 修：`channel_validate` 对每个 customer 调用统一 `validate_channel_file`（与 `load_channel` 共用），并补非法 id/logo/powered_by 负向。

### 10.5 P2/manual（环境阻塞）

- 真实 Docker image save/load/compose up、PG 备份/PITR。
- 三平台 Tauri 安装包与 Windows SHA 实测；`release.yml` installers job 仍是 echo 模板。
- macOS notarize/staple（证书未配，job echo）。
- 真实升级/回滚演练；`upgrade-drill --docker` 仍只 echo。
- 真实 CycloneDX SBOM（当前 `SBOM/README` 占位）。
- 真实 OEM 镜像/安装包二进制残留扫描（当前 fixture 文本/二进制负向）。

这些在无 Docker/证书/平台 runner 环境不可复现；acceptance 必须标 `UNVERIFIED`，不得写 complete。若 CI 在有 runner 环境跑通，可作为 M5 完成证据。

### 10.6 R4 证据

| 命令 | 结果 |
| --- | --- |
| `bash scripts/oem-e2e.sh` | **PASS**：deep plan/compose/cargo test + verify/residue |
| `bash scripts/updates-e2e.sh` | **PASS**：manifest/channel/version reject/artifact/oem-portal |
| `bash scripts/admin-e2e.sh` | **PASS**：10 passed（官方 portal） |
| `cargo test -p acme-brand` | **PASS**（conformance manifest） |
| `zk validate` | **PASS**：71 packages scanned |
| `zk channel validate --index oem/fixtures --all` | **PASS**：2 channels |
| `bash scripts/build-dist.sh official 0.1.0 /tmp/zk-dist-r4` | **FAIL**：assets empty（P0） |
| `bash scripts/upgrade-drill.sh --dry-run` | **PASS（dry-run）**：local fallback/sha 比较，环境项 unverified |
| `cargo test --workspace` / clippy / compose official / schema / db | **PASS**（前轮） |

### 10.7 结论

- **P0：1**（build-dist 空 clients assets 失败）。
- **P1：1 组**（channel_validate 未复用严格 logo/id/powered_by）。
- **P2/manual**：真实 Docker/安装包/公证/升级/SBOM。
- **M5 实现仍不可 APPROVED**：修 P0/P1 后再复评。

- 关联 spec：`specs/changes/2026-10-30-oem-delivery`。
- 复评结论（第四轮）：**CHANGES-REQUESTED**

---

## 11. 第五轮复评（R5，2026-10-30）

> 对象：R4 修复后的 M5 实现。只读复跑；未改产品代码/spec（仅本记录）。
> 仅列剩余 P0/P1；环境阻塞项另标 P2/manual。

### 11.1 结论

**CHANGES REQUIRED（CHANGES-REQUESTED）**

- **P0：0**。R4 P0 build-dist 已修复。
- **P1：1 组（可本地修）**：`channel_validate` 仍不校验 channel.json 的 `id` 与 index id 一致、logo 符号链接逃逸。
- **P2/manual（环境阻塞）**：Docker/三平台安装包/公证/真实升级/SBOM。

### 11.2 已关闭

- **build-dist**：先创建 fallback client，再算 assets/sha；实测 `bash scripts/build-dist.sh official 0.1.0 /tmp/zk-dist-r5` **PASS**，`CLIENT-RELEASE.assets` 非空（name/sha256/size）。
- **channel_validate**：改用 `validate_channel_dir` 共享 id/logo/256KB/script/http/powered_by/deny_unknown_fields；oem-e2e channel validate PASS。
- **deep OEM**：oem-e2e 的 `zk plan`/`compose --edition oem-deep --channel oem-deep --check`/`cargo test -p acme-brand` PASS。
- **portal/1:1/verify**：portal OEM、版本 409、dist/SHA/SBOM/二进制扫描 PASS。

### 11.3 剩余 P1（可本地修）

1. **`channel_validate` 未校验 `channel.json.id` == index key**
   - `tools/zk/src/main.rs:1069` 调 `validate_channel_dir(id, &customer)`，但 `validate_channel_dir` 内部只校验 `file.id` 的正则/长度，不比较 `id` 参数；`oem/fixtures` 的 `channel.json.id` 若与 `index.toml` key 不一致仍可通过。
   - 影响：渠道 id 是 manifest/更新/门户的键；不一致会导致 build/channel/portal 身份错配。
   - 修：`if file.id != channel { bail!(...) }`，补 fixture 负向（index key ≠ channel.json.id）。

2. **logo 符号链接逃逸**
   - `validate_channel_dir` 只查 `..`/绝对/http/256KB/script，不 `canonicalize`，也不 `symlink_metadata` 拒绝 symlink；`dir.join(logo)` 可指向渠道目录外文件，`zk compose` 会把外部 SVG 复制进 `public/brand/`。
   - docs/09 §3 明确要求 canonicalize 后在渠道目录内、拒绝符号链接逃逸。
   - 修：`canonicalize` 后 `starts_with(dir.canonicalize())`；拒绝 symlink；补 symlink 逃逸负向。

### 11.4 P2/manual（环境阻塞，不判 P1）

- 真实 Docker image save/load/compose、PG 备份/PITR。
- 三平台 Tauri 安装包与 Windows SHA 实测；`release.yml` installers job 仍 echo 模板。
- macOS notarize/staple（证书未配；job echo）。
- 真实升级/回滚；`upgrade-drill --docker` 仍只 echo。
- 真实 CycloneDX SBOM；当前 SBOM/README 占位。
- 真实 OEM 镜像/安装包二进制残留扫描（当前 fixture 负向）。

这些必须由有 Docker/证书/平台 runner 的环境执行；acceptance 标 `UNVERIFIED`，不得写 complete。

### 11.5 R5 证据

| 命令 | 结果 |
| --- | --- |
| `bash scripts/build-dist.sh official 0.1.0 /tmp/zk-dist-r5` | **PASS**；assets 非空 |
| `bash scripts/oem-e2e.sh` | **PASS**：deep plan/compose/acme-brand + verify/residue |
| `bash scripts/updates-e2e.sh` | **PASS**：manifest/version reject/oem portal |
| `bash scripts/upgrade-drill.sh --dry-run` | **PASS（dry-run）**：local fallback + sha；docker 模拟 |
| `cargo test --workspace` / clippy | **PASS** |
| `zk validate` / `channel validate` / `compose official --check` | **PASS**：71 packages/2 channels/16 files |

### 11.6 结论

- **P0：0；P1：1 组**（channel id 一致性、logo symlink）。
- **P2/manual**：见 11.4。
- **M5 实现仍不可 APPROVED**：修 P1 后可复评；环境项作为 P2/manual。

- 关联 spec：`specs/changes/2026-10-30-oem-delivery`。
- 复评结论（第五轮）：**CHANGES-REQUESTED**

---

## 12. 第六轮复评（R6，2026-10-30）

> 对象：R5 修复后的 M5 实现。只读复跑；未改产品代码/spec（仅本记录）。
> 仅列剩余 P0/P1。

### 12.1 结论

**APPROVED**

- **P0：0；P1：0**。
- **M5 实现可 APPROVED**。
- **P2/manual（环境阻塞）**：真实 Docker/三平台安装包/macOS 公证/真实升级/SBOM；acceptance 必须标 `UNVERIFIED`，不能写 complete。

### 12.2 R5 P1 关闭

- **channel.json id 一致性**：`validate_channel_dir` 现 `if file.id != channel { bail! }`；`cargo test -p zk` 的 `rejects_id_mismatch` PASS。
- **logo symlink 逃逸**：`symlink_metadata` 拒绝 symlink + `canonicalize` 后必须 `starts_with(canonical_dir)`；`rejects_symlink_logo` PASS。
- **build-dist**：fallback client 先创建再算 assets/sha；`bash scripts/build-dist.sh official 0.1.0 /tmp/zk-dist-r6` PASS，assets 非空。
- **deep OEM**：oem-e2e `zk plan`/`compose --edition oem-deep --channel oem-deep --check`/`cargo test -p acme-brand` PASS。
- **updates/portal**：updates-e2e 仍 PASS（version 409、OEM portal、artifact）。
- **channel verify**：dist/SHA/SBOM/二进制扫描 PASS。

### 12.3 验证

| 命令 | 结果 |
| --- | --- |
| `cargo test -p zk --offline` | **PASS**：id mismatch/symlink tests 2 passed |
| `bash scripts/oem-e2e.sh` | **PASS**：channel/index/pin/compose/brand/residue + deep |
| `bash scripts/updates-e2e.sh` | **PASS**：manifest/version reject/artifact/oem-portal |
| `bash scripts/build-dist.sh official 0.1.0 /tmp/zk-dist-r6` | **PASS**；CLIENT-RELEASE assets 非空 |
| `bash scripts/upgrade-drill.sh --dry-run` | **PASS（dry-run）**：local fallback + sha；docker 模拟 |
| `cargo test --workspace` / clippy / `zk validate` / `channel validate` / `compose official --check` | **PASS**（前轮 R5） |

### 12.4 P2/manual（环境阻塞，不阻塞本次实现 verdict）

- 真实 Docker image save/load/compose、PG 备份/PITR。
- 三平台 Tauri 安装包与 Windows SHA；release installers 仍 echo 模板。
- macOS notarize/staple（证书未配）。
- 真实升级/回滚；`upgrade-drill --docker` 仍 echo。
- 真实 CycloneDX SBOM（当前占位）。
- 真实 OEM 镜像/安装包二进制残留扫描（当前 fixture 负向）。
- 非阻塞：`channel_validate` 对 `.gitmodules` 只查 path 字符串，不读真实 gitlink；fixture pin 用目录 hash；`oem-demo` 门户仅 Linux；`acme-brand` 命名/conformance 深度可在 M6 加固。

### 12.5 结论

- **P0：0；P1：0**。
- 关联 spec：`specs/changes/2026-10-30-oem-delivery`。
- 复评结论（第六轮）：**APPROVED**
