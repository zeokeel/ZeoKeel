# Stage 2 · Embedding/vector Settings 配置 C19 独立评审

- 评审对象：`HEAD dd6920c`（`feat(config): embedding/vector configuration via settings` a992709 + `test(e2e): exact model label after embedding section added` dd6920c）。**复评对象：`HEAD b927efe`（`fix(config): propagate embedding apply errors and gate enterprise C7`；父 dd6920c；详见 §10）**。评审开始时工作树 clean。
- 评审者：**auditor-build**（独立评审；未实现本阶段功能）。
- 关联 spec：`specs/changes/2026-10-31-stage2-embedding-config`（proposal/spec/design/tasks/acceptance）。
- 日期：2026-10-31
- 结论：**APPROVED**（复评 HEAD **b927efe**：P0=0 / P1=0；首评 P1-1/P1-2 已修复，P2 剩余项为低风险加固/测试缺口）。契约/默认值/校验、masked secret、settings>env glue、dir 持久化、enterprise no-op/C7 门禁、UI round-trip、schema determinism 与 e2e 回归均通过。目标真实 provider/Tauri/Docker 仍 UNVERIFIED（见 §8）。

## 1. 评审范围与方法

- 契约/生成 schema/TS：`EmbeddingSettings`、`AppSettings.embedding`、默认/向后兼容、dims/base_url 校验、generated schema。
- 生成 glue：`zk_composed::configure_embedding_from_settings`，settings>env、空 key no-op、dir 持久化、enterprise no-op（C7）。
- desktop：boot/settings.set 应用、错误回传 UI、masked secret 保留、api_key 不进入 RuntimeSettings。
- UI：设置页 embedding section、round-trip、校验、masking、enterprise 可见性。
- 文档/安全/测试：docs/38 truth、无 key 泄漏、schema determinism、单测/e2e regressions。
- 独立复跑门禁与 m1/agent/login/admin e2e；只读审计，唯新增本记录。

## 2. 全量门禁复跑（HEAD dd6920c，独立执行）

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | **EXIT 0** |
| `cargo clippy --workspace --all-targets -- -D warnings` | **EXIT 0** |
| `cargo test --workspace` | **EXIT 0**（420 blocks / **433 passed / 0 failed**） |
| `cargo test --workspace export_schema` + `git diff --exit-code` | **0 / 0**（schema/TS 无漂移） |
| `bash scripts/schema-check.sh` | **0** |
| `python3 scripts/agent-tool-matrix.py --check` | **0** |
| `python3 scripts/docs-check.py` / `spec-check.py` | **0 / 0** |
| `python3 scripts/docs-command-check.py --self-test` / 正式 | **0 / 0** |
| `zk validate` / `compose personal --check` / `capability --check` / `docs --check` / `db lock --check` / `lint --duplicates` / `sbom --check` | **全 0** |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | **0 / 0 / 0 / 0** |
| `python3 scripts/review-check.py` | 写本记录前 **1**（缺本 stage2 记录，预期） |

证据：`target/evidence/audit/stage2/`（fmt-clippy、workspace-tests、other-gates、e2e）。

## 3. embedding 契约 / schema / masking

**已确认成立：**

- `EmbeddingSettings` 字段/Default 完整；`AppSettings.embedding` 有 `#[serde(default)]`，旧 settings 可解析（conformance 测试 + TS `parseSettings` 默认测试均通过）。
- `validate` 拒绝 dims `0/65537` 与非 http(s) `base_url`；UI `settings-validation.ts` 同步拒绝 dims 0/65537/小数和非法 URL。
- `settings.get` / `settings.set` 响应经 `masked_settings`；`embedding.api_key` 回显为空；desktop 单测验证。
- 空 api_key 的 `settings.set` 通过 `keep_existing_secrets` 保留已存 key；UI round-trip 测试发送空 key，符合 write-only 语义。
- `RuntimeSettings` 只有 `agent`/`tools`，没有 embedding；不会把 embedding key/base_url 发布给 agent 侧；`runtime_settings_snapshot_excludes_provider_secrets` 覆盖 provider 无泄漏路径，结构上 embedding 也不在快照。
- schema/TS `export_schema`、`schema-check`、`pnpm` 全 0，生成物无漂移。

## 4. glue / desktop 行为

**已确认成立：**

- `crates/zk-composed/src/lib.rs:127-142` 生成的 personal glue 从 `EmbeddingSettings` 构造 `EmbeddingConfig`/`embedding_model_from_config` 并调用 `configure_with_dir`；空/trim 后空 key 直接返回（env fallback 保留）。
- `tools/zk/src/main.rs:2009-2028` 对非 personal 或无 `zk-cap-vector` 的 edition 生成 no-op；个人 + cap-vector 才生成实际配置；符合 C7 enterprise no-op 方向。
- desktop boot 在 env 配置之后调用 `apply_embedding_settings`（settings 覆盖 env）；`settings.set` 在保存前应用；`keep_existing_secrets` 同时保留 provider/embedding key。
- `crates/zk-composed/tests/embedding_settings.rs` 验证 personal 成功配置 `available()` 与 `persistence_dir()`；dir 持久化路径成立。
- e2e 回归：m1 **0**、agent **0**、login **0**、admin **0**；e2e 后 personal compose `--check` **0**，工作树 clean。

**P1-1（阻塞）：配置失败不返回 UI，settings.set 仍成功并持久化**

- `crates/zk-composed/src/lib.rs:127-142` 的 `configure_embedding_from_settings` **返回 `()`**；`embedding_model_from_config` 的 `Unconfigured`/`Config` 与 `configure_with_dir` 的错误只 `eprintln!`，不传播。
- `apps/desktop/src/ipc.rs:1103-1111` `apply_embedding_settings` 只做 `embedding.validate()`（dims/base_url），随后调用上述函数并恒返回 `Ok(())`。
- `apps/desktop/src/ipc.rs:419-426` 因此：只要 dims/base_url 通过，api_key 非空但 model 为空、base_url 不可用、dir 配置失败等 runtime 配置错误都不会让 `settings.set` 失败；错误被吞，settings 仍保存（`apps/desktop/src/ipc.rs:460-465`），UI 显示保存成功。
- 这直接违背 spec §2“settings.set 应用；错误回传 UI”。当前测试只覆盖非法 dims/base_url（`apps/desktop/src/ipc.rs:1555-1561`），未覆盖 model 空/配置失败。
- 修法：将 `configure_embedding_from_settings` 改为 `Result<(), String>`（no-op/enterprise 返回 `Ok`），personal 分支把 `embedding_model_from_config` / `configure_with_dir` 错误 `?` 传播；`apply_embedding_settings` 传播；`settings.set` 在保存前返回错误。补“api_key 有值 + model 空/非法 config 不保存”的 desktop 单测。

## 5. UI / enterprise C7

**P1-2（阻塞）：企业版设置页无门禁地展示 embedding section，并持久化本地 key/base_url**

- `packages/ui/ui-settings/web/src/index.tsx:384-444` 的 embedding fieldset 无条件渲染，`SettingsDialogProps` 无 edition/showEmbedding 开关。
- `apps/desktop/web/src/App.tsx:630-642` 只对 provider/updater/server URL 做 edition 条件，没有把 `state.meta.edition` 传给 embedding section；enterprise 只 exclude `zeokeel/ui-settings-provider`（`editions/enterprise.toml:93`），仍包含 `ui-settings`。
- 因此 enterprise 设置页显示“嵌入/向量索引”及 API Key 输入，保存会把 key/base_url 持久化到本地 settings；但 glue 在 enterprise 是 no-op（`tools/zk/src/main.rs:2009-2010`、`apps/desktop/src/ipc.rs:1108`），不会生效。
- 同时 `apps/desktop/web/src/i18n.tsx:166/474` 的 `settingsDescriptionEnterprise` 明说“客户端不保存上游密钥 / no upstream key is stored”，与可保存 embedding key 矛盾；C7 也要求企业客户端不持有上游 key。
- 修法：给 `SettingsDialogProps` 增加 `showEmbedding`（personal=true, enterprise=false），包住 fieldset；enterprise 后端对 embedding 做拒绝/清除（避免旧设置或直接 IPC 写入）；补 enterprise UI 不渲染、不持久化 key 的测试。

## 6. P2（非阻塞，需记录/加固）

| # | 位置 | 问题 | 建议 |
| --- | --- | --- | --- |
| P2-1 | `packages/contracts/config/src/lib.rs:364` | `EmbeddingSettings` `#[derive(Debug)]` 直接 Debug 出 `api_key`；当前无日志路径打印它，但公共类型 Debug 是未来泄漏面（kit 的 `EmbeddingConfig` 已做 redaction）。 | 自定义 `Debug` 脱敏 `api_key`（或移除 Debug），加 `format!("{:?}")` 不含 key 的测试。 |
| P2-2 | `docs/38-agent-runtime.md:54` | 只把设置页路径塞进现有行；未明确 spec 要求的优先级 **settings > env > unavailable**、key write-only/masking、personal-only/enterprise no-op；仍保留“personal boot 经 configure_capabilities_from_env 自动配置”的旧口径，未说明 boot 后 settings 覆盖。 | 更新 §5.2 Vector 行：settings 覆盖 env；空 key 才回退 env；key 只写；enterprise no-op。 |
| P2-3 | `shared/contracts/schema/embedding-settings.json:25` | `dims` 生成 schema 只有 `minimum:0`，而 contract/UI 都是 1..=65536；Ajv 边界会接受 0，后端再拒。 | 给 `EmbeddingSettings.dims` 加 schemars range（或自定义 schema），重生成并测 0/65537 均被 schema 拒。 |
| P2-4 | `apps/desktop/src/ipc.rs:256-262` | boot 时 embedding 配置失败仅 `eprintln!`，未通过 `state.record_agent_config_error`/返回值让 UI 获知（settings.set 路径的同类错误还受 P1-1 影响）。 | boot 错误记录到 agentConfigError 或返回 boot error；与 settings.set 统一。 |
| P2-5 | `crates/zk-composed/tests/embedding_settings.rs`；`apps/desktop/src/ipc.rs` 测试；`packages/ui/ui-settings/web` 测试 | 缺测试：settings > env 优先级、enterprise no-op（生成 glue/desktop）、config 失败不保存、enterprise UI 隐藏、Debug 不泄漏 key。 | 补上述回归；至少覆盖 P1-1 与 P1-2 的修复。 |
| P2-6 | `apps/desktop/src/ipc.rs:424-430` | embedding 在 agent/provider 应用前生效；若后续 agent/provider/保存失败，settings 未保存但 in-memory vector 已切换，形成部分状态。既有 agent 路径有类似顺序；embedding 新增了外部模型切换。 | 在全部校验/保存成功后再提交 embedding，或失败时恢复上一模型/状态。 |

## 7. 安全 / 无泄漏验证

- `settings.get`/`settings.set` 均 mask `embedding.api_key`；desktop 单测覆盖（`apps/desktop/src/ipc.rs:1538-1551`）。
- `RuntimeSettings` 无 embedding 字段（`packages/contracts/config/src/lib.rs:503-516`），key/base_url 不进入 agent runtime settings。
- `configure_embedding_from_settings` 错误日志使用 `EmbeddingError`/`VectorError`，不含 key；`EmbeddingConfig` Debug 已 redaction；未发现当前日志打印 raw key。
- **P2-1** 的 `EmbeddingSettings` 公共 Debug 是剩余泄漏面；**P1-2** 的 enterprise UI 会持久化本不该持有的 upstream key。
- UI 输入 `type=password`、`autoComplete=off`；`settings.get` 不回显。

## 8. UNVERIFIED / 非目标

- 真实 embedding provider 调用（本机仅构造 rig model，无网络；spec 已标 UNVERIFIED）。
- media image/audio/transcription/rerank 配置（spec 明确下一阶段，不在本阶段）。
- Tauri/Windows 真机、Docker/多平台安装包。
- GitHub hosted runner 的 CI 执行（本阶段 e2e 仅本机复跑）。

## 9. 结论与复评建议

- **首评：P0=0；P1=2；P2=6**。
- **CHANGES-REQUESTED（首评）**：P1-1 配置失败传播到 settings.set UI 且不持久化失败配置；P1-2 enterprise 隐藏/拒绝 embedding 配置，避免持有上游 key。
- **复评（b927efe）：P1=0；APPROVED**。P1-1/P1-2 已修复；P2-1/2/3/4 已修，P2-5/6 剩低风险测试/顺序加固（详见 §10）。
- 本记录是唯一新增文件；未修改产品或生成物。`review-check` 写本记录前 1（缺 stage2 记录），本 APPROVED 后应 0。

## 10. 复评（HEAD b927efe）

- 复评对象：`HEAD b927efe`（`fix(config): propagate embedding apply errors and gate enterprise C7`；父 dd6920c）。工作树 clean（唯一未跟踪文件 = 本记录）。
- 复评结论：**APPROVED（P0=0 / P1=0；P2 剩余低风险项保留）**。

### 10.1 P1 复核

- **P1-1 已修复**：`crates/zk-composed/src/lib.rs:127-142` `configure_embedding_from_settings` 现在返回 `Result<(), String>`，把 `embedding_model_from_config` 与 `configure_with_dir` 错误 `?` 传播；`apps/desktop/src/ipc.rs:1166-1184` `apply_embedding_settings` 将 glue 错误映射为 `embedding settings: ...` 返回；`settings.set` 在保存前 `return CommandResponse::err(error)`（`apps/desktop/src/ipc.rs:419-426`），不再保存失败配置。新增真实 `AppState` 回归测试 `settings_set_rejects_failed_embedding_without_persisting`（`apps/desktop/src/ipc.rs`），验证 key+空 model 被拒且 store 保持旧值；glue 测试 `embedding_configuration_failures_are_propagated` 覆盖缺 model/bad scheme。
- **P1-2 已修复**：enterprise UI 通过 `showEmbedding={ui.meta.edition === "personal"}`（`apps/desktop/web/src/App.tsx:638`）隐藏 embedding fieldset；`packages/ui/ui-settings/web/src/index.tsx:40-90` 增加 `showEmbedding` 默认 true 并条件渲染（`:384,446`）；enterprise 测试验证不渲染 embedding/API key（`apps/desktop/web/tests/app-settings.test.tsx`）。后端 `embedding_edition_rejection`（`apps/desktop/src/ipc.rs:1150-1161`）拒绝 enterprise 任何本地 embedding 尝试；boot `configure_persisted_embedding` 清除 stale embedding（`apps/desktop/src/ipc.rs:250-271`，main `:176`）。C7 语义成立。

### 10.2 复评门禁（b927efe，独立执行）

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` / clippy workspace | **0 / 0** |
| `cargo test --workspace` | **0**（712 blocks / **455 passed / 0 failed**） |
| `cargo test --workspace export_schema` + `git diff --exit-code` | **0 / 0**（schema 已含 dims 1..65536） |
| `schema-check` / matrix / docs-check / spec-check / docs-command self+check | **全 0** |
| `pnpm -r typecheck/lint/test:unit/build` | **全 0**（150 tests） |
| `review-check` | 写本记录前 **1**（缺本记录）；APPROVED 后应 0 |
| e2e m1 / agent / login / admin | **0 / 0 / 0 / 0**；e2e 后 compose personal --check 0，工作树 clean |

### 10.3 P2 状态与剩余

- **已修**：P2-1 Debug 脱敏（`packages/contracts/config/src/lib.rs:379-395`，conformance 断言 no key + `[redacted]`）；P2-2 docs/38 补 priority/masking/personal-only/enterprise/failure；P2-3 schema dims `minimum 1/maximum 65536`（`embedding-settings.json` + schema test）；P2-4 boot 错误 `state.record_agent_config_error`（`apps/desktop/src/main.rs:176-179`）。
- **剩余低风险 P2**：①`settings.set` 在 agent/provider 保存前已切换 embedding，若后续失败仍可能部分状态（顺序加固）；②缺 settings>env 优先级、enterprise stale 清除、enterprise glue no-op 的显式回归测试（P2-5 部分）；③enterprise 清除 stale 的保存失败仅日志（极端路径）。这些不阻塞 APPROVED，记录为 residual。
- **终评结论：APPROVED（HEAD b927efe；P0=0 / P1=0；P2 residual 保留）**。
