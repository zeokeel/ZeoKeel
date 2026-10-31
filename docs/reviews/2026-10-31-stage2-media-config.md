# Stage 2 · Media Settings（image/audio/transcription/rerank）C19 独立评审

- 评审对象：`HEAD 62b6125`（`feat(config): media settings for image/audio/transcription/rerank`；父 f612c48）。**复评对象：`HEAD d4eab8c`（`docs(38): media settings truth (settings>env, masking, personal-only/C7, error aggregation)`；父 62b6125；详见 §9）**。评审开始时工作树 clean。
- 评审者：**auditor-build**（独立评审；未实现本阶段功能）。
- 关联 spec：`specs/changes/2026-10-31-stage2-media-config`（proposal/spec/design/tasks/acceptance）。
- 日期：2026-10-31
- 结论：**APPROVED**（复评 HEAD **d4eab8c**：P0=0 / P1=0；首评 P1-1 docs/38 已修复，P2 残余保留）。MediaSettings 契约/默认/校验/Debug 脱敏、glue 逐 kind 配置与错误聚合、desktop apply/error/boot、enterprise C7 门禁、masking、UI 个人/企业 gating、schema/TS 与 e2e 回归均通过；docs/38 §5.3/§5.4 与 ADR-067 已同步。真实 provider/Tauri/Docker/hosted runner 仍 UNVERIFIED。

## 1. 评审范围

- 契约：`MediaSettings` 字段/Default、`AppSettings.media` serde default/legacy、base_url 校验、Debug redaction、schema/TS。
- 生成 glue：`configure_media_from_settings` 空 key no-op、逐 kind `MediaConfig`、错误聚合/partial success、enterprise/no-media no-op、generator 依赖。
- desktop：boot/settings.set apply、失败不落盘、agent_config_error、mask/keep-secret、enterprise reject/clear stale。
- UI：media section、字段/校验/i18n、personal-only 可见、enterprise 隐藏、round-trip。
- 安全/文档/测试/e2e：无 key 泄漏、docs/38/13 truth、schema determinism、m1/agent/login/admin 回归。

## 2. 全量门禁复跑（HEAD 62b6125，独立执行）

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` | **0 / 0** |
| `cargo test --workspace` | **0**（714 blocks / **458 passed / 0 failed**） |
| `cargo test --workspace export_schema` + `git diff --exit-code` | **0 / 0**（schema/TS 无漂移） |
| `schema-check` / matrix / docs-check / spec-check / docs-command self+check | **全 0** |
| `zk validate` / `compose personal --check` / `capability` / `docs` / `db lock` / `lint` / `sbom --check` | **全 0** |
| `pnpm -r typecheck/lint/test:unit/build` | **全 0** |
| `review-check` | 写本记录前 **1**（缺 media 记录，预期） |
| e2e m1 / agent / login / admin | **0 / 0 / 0 / 0**；e2e 后 compose `--check` 0，工作树恢复 clean |

证据：`target/evidence/audit/stage2-media/`。

## 3. 契约 / glue / desktop（已确认成立）

- `MediaSettings` 六个字段/Default；`AppSettings.media` `#[serde(default)]`；legacy settings 测试通过。
- `validate` 拒绝非 http(s) 非空 base_url；`Debug` 自定义脱敏 `api_key`（conformance 断言 no key + `[redacted]`）；schema 生成 `media-settings.json`/app settings/TS 无漂移。
- `crates/zk-composed/src/lib.rs:139-170` `configure_media_from_settings`：空 api_key `Ok` no-op；逐 image/audio/transcription/rerank 构造 `MediaConfig::new(kind.default_provider(), base,key,model)`，`configure_from_config` 错误聚合；部分成功保留（spec 允许并测试）。
- `tools/zk/src/main.rs:2028-2070` 生成 personal 实际函数；非 personal 或没有 media 包时生成 `Ok(())` no-op；`compose_cargo_toml` 在 personal + cap-vector/media 时加 `zk-kit-media`。
- `apps/desktop/src/ipc.rs:250-275` `configure_persisted_capabilities` personal 先 embedding 后 media，错误传播；enterprise 清除 stale embedding+media/key；`main.rs:176-179` boot 错误 `record_agent_config_error`。
- `settings.set` `:466-470` 在保存前调用 `apply_media_settings`；`apply_media_settings` `:1225-1239` 校验 + enterprise reject + personal glue error 传播；失败不落盘。
- `masked_settings` `:1140-1146` mask media.api_key；`keep_existing_secrets` `:1251-1254` 仅 personal 保留空 key；UI round-trip 测试发送空 key。
- UI `SettingsDialogProps.showMedia` + `App.tsx` `showMedia={edition==="personal"}`；enterprise 测试不渲染 media/API key；个人测试填写四模型并 round-trip。
- 安全正向：`MediaConfig` Debug 脱敏、`MediaSettings` Debug 脱敏、错误只含 provider/base/prefix；`RuntimeSettings` 无 media；当前日志无 raw key。
- e2e 回归：m1 **0**、agent **0**、login **0**、admin **0**。

## 4. P1（阻塞 APPROVED）

**P1-1：docs/38 未更新媒体配置，仍把媒体描述为 env-only**

- 本阶段 commit **没有修改 `docs/38-agent-runtime.md`**（`git diff f612c48..HEAD -- docs/38-agent-runtime.md` 为空），而 spec §4/acceptance 明确要求 docs/38。
- §5.3（`docs/38-agent-runtime.md:59-66`）四个媒体表仍只写 `ZEOKEEL_{IMAGE,AUDIO,TRANSCRIPTION,RERANK}_*`，没有 `AppSettings.media`、base_url/api_key、四个 model 字段、settings>env、空 key fallback、key write-only/masking、personal-only/enterprise C7、失败聚合/partial success。
- `docs/38:70` 仍写“personal boot 调用 `configure_capabilities_from_env()`”，`docs/38:77`（§5.4）仍写“media/vector 配置由 personal boot 调用 env glue”，与新的 settings apply 路径不一致。
- 因此文档真源仍是 **false current claim（env-only）**；修法：在 §5.3/§5.4 补 `AppSettings.media` 配置入口、优先级 settings>env>unavailable、空 key no-op、key masking、personal/enterprise 语义、错误聚合/partial success、boot/settings.set 行为。修复后即可复评 APPROVED。

## 5. P2（非阻塞，需记录/加固）

| # | 位置 | 问题 | 建议 |
| --- | --- | --- | --- |
| P2-1 | `docs/13-risks-and-adr.md:618`（ADR-067 后果） | 仍写“真实媒体供应商仍需配置 env”，现在个人版也可经设置页/`AppSettings.media` 配置；ADR 后果过期。 | 补 settings 入口；真实 provider 仍 UNVERIFIED 可保留。 |
| P2-2 | `apps/desktop/src/ipc.rs` media 测试 | 只有 pure helper `apply_media_settings`/edition 测试；**没有真实 AppState `settings.set` media 失败不落盘/agent_config_error 回归**（embedding 有同类真实测试）。代码路径正确（同一 `settings.set` 分支），但测试证据不足。 | 增加 media key+非法 config 的 AppState no-save 测试，以及 boot media 错误 agent_config_error 测试。 |
| P2-3 | `apps/desktop/src/ipc.rs:250-275` enterprise stale clear | 无 enterprise stale embedding+media 清除的显式测试；保存失败路径只 eprintln。 | 补纯 helper/临时 settings 文件测试；保存失败报错/记录。 |
| P2-4 | `crates/zk-composed/src/lib.rs:139-170` | 聚合错误时部分 kind 已配置成功、但 desktop settings 未保存，形成跨 capabilities 的部分状态；spec 允许 media partial success，但 UI 保存失败与运行时部分生效需要文档化。 | 在 docs/38 明确；或失败时回滚已配置 kind。 |
| P2-5 | `apps/desktop/src/ipc.rs:466-470` | media 在 agent/provider 保存前生效；若后续 agent/provider 失败，settings 不保存但 media 已切换。与 embedding 同类顺序风险。 | 全部校验/保存成功后再提交，或失败恢复。 |
| P2-6 | `specs/changes/2026-10-31-stage2-media-config/tasks.md`、`acceptance.md` | tasks 仍 pending、acceptance in-progress；写入本记录后由 Lead 更新 done/APPROVED 并归档。 | 更新 acceptance/tasks。 |

## 6. 安全 / 无泄漏验证

- `MediaSettings` 自定义 Debug 脱敏（conformance 测试）；`MediaConfig` 已有 Debug redaction；media glue 错误聚合使用 `MediaError` 文本（provider/prefix/base URL），未发现 raw key。
- `settings.get`/`settings.set` 均 mask `media.api_key`；desktop helper 测试覆盖；空 key 保留仅 personal。
- `RuntimeSettings` 只有 agent/tools，不发布 media key/base；agent 侧不可见。
- UI `type=password`、`autoComplete=off`；enterprise 不渲染 media section。
- 剩余: P2-1 docs stale、P2-4 部分生效；无当前泄漏证据。

## 7. UNVERIFIED / 非目标

- 真实 image/audio/transcription/rerank provider 调用（本机只构造 rig model，无网络；spec 已标 UNVERIFIED）。
- Tauri/Windows 真机、Docker/多平台安装包、GitHub hosted runner。
- 真实企业部署的 C7 运行验证（本机 personal baseline；enterprise 代码/测试路径已检查）。

## 8. 结论与复评建议

- **首评：P0=0；P1=1；P2=6**。
- **CHANGES-REQUESTED（首评）**：docs/38 media 配置真源（env-only）需修复；docs/13 ADR-067 同步。
- **复评（d4eab8c）：P1=0；APPROVED**。P1-1 已修复；P2-1 docs/13 已修；P2-2..6 低风险测试/顺序/状态项保留（详见 §9）。
- 本记录是唯一新增文件；未修改产品或生成物。`review-check` 终评后应 0。

## 9. 复评（HEAD d4eab8c）

- 复评对象：`HEAD d4eab8c`（`docs(38): media settings truth (settings>env, masking, personal-only/C7, error aggregation)`；父 62b6125）。此 commit 仅 docs/38、docs/13、stage2 media spec 文件；无产品/生成物/迁移变化。工作树 clean（唯一未跟踪文件 = 本记录）。
- 复评结论：**APPROVED（P0=0 / P1=0；P2 残余保留）**。

### 9.1 P1 复核

- **P1-1 已修复**：`docs/38-agent-runtime.md:63-66` 四个媒体 row 现在同时记录 env 与 `AppSettings.media`（设置页媒体能力，共享 base_url/api_key + 各 model；settings>env；api_key 只写/masking；enterprise 隐藏/拒绝）；`:70` 宿主接线明确 personal 先 env、后有保存设置时 `configure_embedding_from_settings`/`configure_media_from_settings` 覆盖（Result；失败阻断 settings.set 不落盘、boot agent_config_error；媒体错误聚合/部分成功；enterprise UI 隐藏、settings.set 拒绝、boot 清理 stale）；`:77` §5.4 同步。
- **docs/13 ADR-067** `:618` 后果从 env-only 更新为 settings+env，真实 provider 仍 UNVERIFIED。
- docs-check **0**；docs/spec review 结构检查 0。

### 9.2 复评门禁

- 本 commit 无代码/生成物；首评 HEAD 62b6125 的全量门禁保持有效：fmt/clippy 0、workspace 714 blocks/**458 passed/0 failed**、export_schema+diff 0、schema/matrix/docs/spec 0、zk 全 0、pnpm typecheck/lint/test:unit/build 0、e2e m1/agent/login/admin 0。
- 复评 docs-check **0**；docs-command self+check **0**；review-check 终评后 **0**。

### 9.3 P2 残余（不阻塞）

- P2-2：media 缺真实 AppState settings.set no-save / boot agent_config_error 专项回归（embedding 有；代码路径同一）。
- P2-3：enterprise stale embedding+media 清除无显式测试，保存失败仅 eprintln。
- P2-4：媒体错误聚合时部分 kind 已配置而 settings 未保存（跨 capabilities 部分状态；spec 已声明 media partial success，建议文档/回滚）。
- P2-5：media 在 agent/provider 保存前生效，后续失败有顺序风险。
- P2-6：tasks/acceptance 仍 pending/in-progress，由 Lead 在本记录后更新 done/APPROVED。

- **终评结论：APPROVED（HEAD d4eab8c；P0=0 / P1=0；P2 残余保留）**。
