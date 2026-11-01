# 2026-11-01-client-settings-ia · 验收与证据

- 验收人：impl-settings（实现者）
- 日期：2026-11-01
- PR/commit：未提交（共享工作树；由 Lead 汇总提交。§1 中两项要求「已提交基线」的门禁因此记为 UNVERIFIED-until-commit）
- 证据根目录：`target/evidence/2026-11-01-client-settings-ia/`

> 状态：`done`（实现与本地证据齐全；`git diff --exit-code` / `pnpm e2e` 的提交后复跑与 `cargo public-api diff` 见 §1 备注）

## 1. 验收命令与结果

| 命令 | 结果 | 证据路径 |
| --- | --- | --- |
| `python3 scripts/spec-check.py` | **exit 0**（0 errors / 0 warnings） | `spec-check.log` |
| `python3 scripts/docs-check.py` | **exit 0**（0 errors / 0 warnings / 41 files） | `docs-check.log` |
| `python3 scripts/review-check.py` | **exit 0** | `review-check.log` |
| `./target/debug/zk validate` | **exit 0**（enterprise=87 / personal=56、roles=90、115 scanned） | `zk-validate.log` |
| `zk compose --edition personal --profile default --channel official --check` | **exit 0**（up to date，17 files） | `compose-personal.log` |
| 企业组合快照 | `cargo test -p zk ui_registry_tests` **exit 0**：enterprise pane 7 个（general/models/agent/tools/search/enterprise/about），无 provider/embedding/media | `zk-pane-snapshots.log`（非基线不支持 `--check`，故用 resolve+registry 快照断言） |
| `zk capability --check` | **exit 0**（无新增 capability） | `capability.log` |
| `zk docs --check` | **exit 0** | `zk-docs.log` |
| `cargo test -p zk -p zk-contract-ipc -p zk-contract-config -p zk-store-settings-json -p zk-desktop` | **exit 0**（含 revision 冲突/递增、meta 回滚/损坏/并发、pane 排序/edition 过滤/空列表） | `cargo-test.log` |
| `cargo test --workspace export_schema` | **exit 0**（新增 `ipc-settings-get-result.json` / `ipc-settings-set-result.json`，`ipc-settings-set.json` 增 `expected_revision`） | `export-schema.log` |
| `git diff --exit-code`（生成物） | **UNVERIFIED-until-commit**：共享工作树按约定不提交；替代证据 = 生成器二次运行字节不变（sha256 前后一致，85 schemas） | `generated-sha-before.txt` / `generated-sha-after.txt` / `export-schema-idempotent-run.log` |
| `bash scripts/schema-check.sh` | **exit 0**（schema-ts up to date；schema-check ok） | `schema-check.log` |
| `pnpm -r typecheck` | **exit 0** | `typecheck.log` |
| `pnpm -r lint` | **exit 0**（0 warnings） | `lint.log` |
| `pnpm -r test:unit` | **exit 0**（desktop-web 121 tests；ui-kit/ui-settings/ui-settings-provider 新增用例全绿） | `test-unit.log` |
| `pnpm -r build` | **exit 0** | `pnpm-build.log` |
| `cargo check --workspace --all-targets`（跨包编译面） | **exit 0** | `cargo-check-workspace.log` |
| `cargo clippy <本次 5 个 crate> --all-targets --all-features -- -D warnings` + `cargo fmt --all -- --check` | **exit 0** | `clippy-fmt.log` |
| `pnpm e2e`（设置/axe 用例） | **等价序列全绿**：mock 阶段 8 passed（20 skipped 属其它 suite 门控）；serve 阶段 14 passed，2 个失败已修（375px 入口、`settings.meta.json` 未列入 e2e 工作区 `.gitignore`）后重跑 5 passed。**`pnpm e2e` 原样复跑需已提交生成基线**（`scripts/restore-baseline.sh` 用 `git checkout --` + `--check`，工作树未提交时必然失败，见 §4） | `e2e-manual.log`、`e2e-retry.log`；截图 `e2e/m1-a11y` 输出到 `target/evidence/release-1.0-rc1/phase-b/b12b14/client-settings*.png` |
| `cargo public-api diff`（契约变更） | **UNVERIFIED（本机未安装）**：新增 DTO/`ConfigError::Conflict` 为 additive；CI/有工具时必须跑（P2-14.3 已补进 `tasks.md`） | `public-api.log`（仅记录命令与"未安装"输出） |
| `bash scripts/i18n-check.sh` | **exit 1（基线即红，非本变更）**：仅 `apps/desktop/web/src/main.tsx:47,49` 两处硬编码中文，属 approval spec task #11（评审记录 §14.5 已记录基线红）；本 spec 未改该文件（`git diff --stat` 为空，见 `i18n-attribution.txt`）；本 spec 新增文案全部走 `i18n.tsx` 双语键 | `i18n-check.log`、`i18n-attribution.txt` |

基线（开工前）：`cargo test -p zk -p zk-contract-ipc -p zk-desktop` **exit 0**；`pnpm -r typecheck` **exit 0**（`baseline-cargo.log` / `baseline-typecheck.log`）。

## 2. 手工检查（以自动化证据覆盖）

- [x] 设置六态/键盘/a11y/错误提示（分页后 Tab 顺序、Esc、焦点回位）：`e2e-manual.log` 设置 dialog axe/Esc/焦点回位 + pane 遍历 axe；`pnpm -r test:unit` 的 dialog 用例（空 pane 空态、保存中态、冲突态）。
- [x] 保存冲突：宿主 `settings.set` 过期 `expected_revision` 拒绝不写盘（`cargo-test.log` desktop `settings_set_revision_fence_rejects_stale_writes`）；前端保留草稿 + 冲突文案 + 「重新加载」（`test-unit.log` settings dialog conflict 用例、app 集成 conflict 用例）。
- [x] 窄窗 375 / 900 / 1280：375 由 `e2e-retry.log` 断言无横向滚动 + axe；900/1280 由既有 m1-a11y/serve 用例（1280/820）与 CSS 断点覆盖。
- [x] personal/enterprise 分区差异：`zk-pane-snapshots.log` personal=9 / enterprise=7；edition 过滤、空列表、OEM 继承单测。
- [x] 企业 `server_url` 渠道锁定只读、改地址需重登（C14/ADR-042/044）：EnterprisePane 单测断言锁定只读 + 提示 + 登出/网关模型入口；改地址重登逻辑沿用既有 m1-login/设置保存路径（App `requiresRelogin`，本 spec 未改语义）。**真实企业 serve e2e 未在本环境跑**（需企业组合 + PG）；记为局部覆盖。
- [x] 双文件保存故障注入：settings 成功 + meta 失败 → 回滚；回滚失败 → 删 meta + revision=0；meta 损坏/hash 不匹配 → revision=0；并发写串行递增（`cargo-test.log` store 单测 8 条）。
- [x] 空 pane 列表空态、pane 渲染异常局部降级（`SlotErrorBoundary` 保留由 app 包裹，dialog 用例空态断言）。

## 3. 文档与决策

- [x] 任务 0：`docs/16`（多贡献 slot 排序/空列表/`editions` 过滤/内建 order 表/pane props/根 edition 规则）、`docs/24`（manifest `editions` 字段 + `§3.1` registry 生成规则）、`docs/19`（`§3.4` 设置分页/单一保存/revision 冲突）已落定。
- [x] 任务 12 回写：`docs/16`/`docs/28`（Commands/Settings + DSH 事实校正）/`docs/19` §3.4 状态。
- [x] ADR/D：不新增 ADR（T6：`docs/16`/`docs/24` 既有机制扩展）。
- [x] `docs/README.md` 主题登记：不适用。
- [ ] 变更 spec 归档到 `specs/archive/`：由 Lead 在合并后执行（本实现未移动 spec 目录）。
- [x] P2-14.3：`tasks.md` 已补 `cargo public-api diff` 一行（任务 13 + 验收前检查）。

## 4. 回滚

- 回滚步骤：回退 commit；删除 `settings.meta.json` 即回退 revision=0；`settings.json` 未改；`SettingsPanes` 生成物由 `zk compose` 重生成。
- 数据影响：无数据库迁移。
- 已验证回滚：store 单测覆盖「meta 缺失/hash 不匹配 → revision=0 重建」；`settings.json` schema 不变（`ipc-settings-set.json` 仅加 `expected_revision`，`#[serde(default)]`，旧客户端兼容）。

## 5. 剩余风险 / 后续

| 项 | 影响 | 后续 spec |
| --- | --- | --- |
| 设置内搜索/快捷键跳转 | 低 | 后续增强（本 spec 非目标） |
| 逐 pane 保存/局部合并 | 低 | 暂不做（已定单一保存） |
| 真实企业组合 e2e（C14 全链路） | 中 | 需企业 serve + PG 的本机环境；本 spec 用单测/既有企业 e2e 覆盖 |
| `cargo public-api diff` / `pnpm e2e` 原样复跑 | 低 | 需提交后/CI 环境；已给出等价证据 |
| i18n 门禁基线红（`main.tsx`） | 低 | approval spec task #11 修复 |

## 6. 结论

- [x] `done`：实现与本地证据齐全（`pnpm e2e` 原样复跑与生成物 `git diff --exit-code` 依赖提交，已给出等价证据）
- [ ] `abandoned`

## 7. 复用证据（C17）

- 搜索记录：`rg -n "settings.provider|slot_binding|revision" tools/zk packages/ui packages/contracts packages/client/store-settings-json`
- 复用清单：`ui-kit.useDialogFocus` 等价焦点逻辑（dialog 现有 restoreFocus 保留）、`ui-kit.SlotErrorBoundary`（app 包裹）、`ui-kit` 新增共享字段原语（避免 ui-settings 与 ui-settings-provider 各写一份）、`ui-settings-provider`（改挂 `ui.settings.pane`，未复制字段实现）、`store-settings-json`（原子写扩展为双文件事务）。
- 新增能力登记：无（`ui.settings.pane` 是 slot 机制，不是 `@capability`；`zk capability --check` 保持绿）。
