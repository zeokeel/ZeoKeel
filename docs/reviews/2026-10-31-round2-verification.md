# 2026-10-31 round-2 独立评审记录（config/media/audit-hardening）

- 评审对象：首次评审 base `140a4e6` + round-2 提交 `e8137ef`（config 契约/usage/desktop 接线）、`57d2928`（media 能力）、`8eb2bca`（web agent settings/usage/Ajv；首次评审 SHA）；复评对象为修复提交 `0e03e66`（media/embedding kit）、`c78f3b2`（boot 优先级/hooks/usage）、`7e5507d`（web usage 镜像）、`2cd690d`（deny/security pin + capability host wiring）、`9ed7d40`（Ajv strict），最终复评 SHA **`9ed7d40`**（工作树 clean）。评审期间 HEAD 从 `140a4e6` 推进到 `9ed7d40`（内容与上述提交一致，记录按最终 SHA 固化）。
  受影响路径：`packages/contracts/{agent,config,ipc}`、`apps/desktop/{src,web}`、`packages/ui/{ui-kit,ui-settings,ui-chat,ui-shell}`、`packages/client/{agent-rig,llm-rig,llm-gateway,cap-image,cap-audio,cap-transcription,cap-rerank}`、`shared/contracts/{schema,ts}`、`docs/38`、两份 change spec、`deny.toml`。
- 评审者：auditor-rust（独立只读评审者；未参与 round-2 实现，仅在此前轮次承担 zk-bootstrap/desktop 基础设施修复，与本轮 config/media/Ajv 变更无实现关系）
- 独立性：非实现者；本轮未修改产品代码，只写本记录。
- 关联 spec：
  - `specs/changes/2026-10-31-config-and-media`（契约 agent/usage、桌面配置、设置 UI、Ajv、媒体能力）
  - `specs/changes/2026-10-31-audit-hardening`（上一轮加固；验收证据与剩余 backlog）
- 结论：**CHANGES-REQUESTED**（P0 = 0；P1 = 7；P2 = 12）。latest conclusion 以本行为准。
- 复评日期/结论：2026-10-31，复评通过（详见 §6；最终以文末 latest 结论行为准）。

## 1. 可执行门禁（本机实跑，exit code）

| 命令 | rc | 说明 |
| --- | --- | --- |
| `cargo fmt --all -- --check` | 0 | 提交后工作树内容（round-2 三个提交） |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 | |
| `cargo test --workspace` | 0 | 含新增契约/agent-rig/media/Ajv 单测 |
| `python3 scripts/docs-check.py` / `spec-check.py` | 0 / 0 | |
| `python3 scripts/review-check.py` | **1** | 本记录创建前：两份 spec 指向的 review record 不存在（见 P1-7） |
| `zk validate` / `capability --check` / `db lock --check` / `docs --check` | 0 / 0 / 0 / 0 | |
| `zk compose --edition personal --profile default --channel official --check` | **0**（提交后） | 评审早期工作树曾被先前 e2e 留在 enterprise 基线，`--check` 曾为 1；`8eb2bca` 提交后已恢复 personal 且 `--check` 0 |
| `cargo test --workspace export_schema`（连续两次） | 0 / 0 | 两次 hash 相同（见 P2-8 的 invocation 依赖问题） |
| `bash scripts/schema-check.sh` | 0 | |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | 0 / 0 / 0 / 0 | 全仓 |
| desktop web：`typecheck` / `lint` / `vitest` / `build` | 0 / 0 / 0（12 files, 55 tests）/ 0 | Ajv 边界/设置/usage 测试 |
| `bash scripts/agent-e2e.sh`（`ZEOKEEL_AGENT_GREP='max turns'`） | 0 | 1 passed；仅 max-turns 子集（full e2e 未跑，见 §4） |
| `cargo deny check` | **UNVERIFIED** | 本机未安装 `cargo-deny`（command not found）；CI `security.yml:9-10` 安装后运行 |

## 2. P1（必须修复后才可 APPROVED）

### P1-1 启动时无条件发布默认 agent 设置，覆盖 `ZEOKEEL_AGENT_*` env（优先级与既有 e2e/开发流冲突）
- 位置：`apps/desktop/src/main.rs:167`（boot 调 `apply_persisted_agent_settings`）→ `apps/desktop/src/ipc.rs:111-133`（无条件 `set_default_agent_settings`，即使 settings 文件不存在、全是默认值）→ `packages/client/agent-rig/src/config.rs:353`（`default_agent_config` 先读已发布设置，env 只在“从未发布”时兜底）。
- 问题：设计 `specs/changes/2026-10-31-config-and-media/design.md:8` 明确优先级 `settings/显式 > env > 内置默认`；实现把“默认 settings”当成显式配置。实测不影响模型 provider（provider 仍有 env 路径），但 `ZEOKEEL_AGENT_MAX_STEPS/_TEMPERATURE/_TOP_P/...`、hooks env 在 desktop 启动后全部被默认值（max_steps=50, hooks.enabled=true）覆盖；`scripts/agent-e2e.sh:95` 的 `ZEOKEEL_AGENT_MAX_STEPS=3` 实际不再生效。max-turns e2e 仍通过只是因为 fake upstream 持续返回工具调用，用 50 步也会在超时内报错，**该 e2e 无法发现此回归**。
- 修法：仅在 settings 文件“存在/被显式保存过”时发布（SettingsStore 增加 existence/has_source，或比较 `AppSettings::default()` 时跳过 `agent` 发布）；hooks sink 同理；新增单测：无 settings 文件 + `ZEOKEEL_AGENT_MAX_STEPS=3` → `effective_config().max_steps == 3`（可用 env-guard 的串行测试）。
- 关联：config-and-media spec §2、design §1；`docs/05` 优先级。

### P1-2 媒体能力在 shipped desktop 中没有任何接线：文档声称“配置 env 后可用”，实际永远 unavailable
- 位置：`packages/client/cap-image/src/lib.rs:1-12`（文档声明 `configure_from_env()` 可用）及 `:178`；同类 `cap-audio:123`、`cap-transcription:133`、`cap-rerank:122`。宿主 `apps/desktop/src/{main,ipc}.rs`、`crates/zk-composed` **无任何调用**。
- 复现：`grep -rn 'configure_from_env' --include='*.rs' packages apps crates` → 仅 cap-* 自身定义、llm-rig 错误文案、测试调用；`grep -rn 'zk_cap_image\|zk_cap_audio\|zk_cap_transcription\|zk_cap_rerank' apps crates` → 0。
- 问题：用户按 `docs/38-agent-runtime.md` §5.3 配置 `ZEOKEEL_IMAGE_*` 等 env 后，composed desktop 不会调用 `configure_from_env`，`available()` 恒 false；测试只直接 `configure(fake_model)`，造成 false-green（成功指标“配置 provider 后能力可用”未达成）。
- 修法：personal 启动路径（`boot_desktop`，`EDITION=="personal"`）对四个 cap 调 `configure_from_env()` 并把结果/原因写入 `meta.get` 的诊断；enterprise 不调用（见 P1-3）。若不接线，则必须把 docs/38、spec §3、acceptance 改为“API 已实现，host 接线下一步”，并从成功指标移除“env 后可用”。
- 关联：config-and-media spec §2/§3、success metric；docs/38 §5.3。

### P1-3 企业版媒体能力没有编译期/运行期闸门，违反 C7（企业 AI 只走网关）
- 位置：`editions/enterprise.toml:3-6` 映射四个 media 角色；`zk.toml:354-373` 的 `media.*` roles `editions = ["personal","enterprise"]`；`packages/client/cap-*/Cargo.toml:8,10` 把 `rig-core` + `zk-llm-rig`（可读 `ZEOKEEL_*_API_KEY`、可填 base_url）编进 enterprise 二进制。`docs/38-agent-runtime.md:69` 声称“企业版保持不可用”，但代码没有 edition 判断，任何宿主/未来 IPC 都能 `configure_from_env/configure_from_config` 直连上游媒体 provider。
- 修法（二选一，推荐前者的编译期方案）：① 将 `cap-{image,audio,transcription,rerank}` 列入 `editions/personal-only.toml`，enterprise `exclude` 同步，`zk.toml` media roles editions 改 `["personal"]`，重跑 compose；② 若必须保留包，则在宿主把 media 配置限制为 `EDITION=="personal"` 并在 capability 内移除 env 自动配置路径 + docs 更新（仍不能阻止 Rust API 调用，弱于①）。
- 关联：C7/ADR-031、docs/38 §5.3、config-and-media spec §3。

### P1-4 C4/ADR 违规：cap-* 直接依赖实现包 `zk-llm-rig` 且新增 4 个 rig 适配点，无 ADR
- 位置：`packages/client/cap-{image,audio,transcription,rerank}/Cargo.toml:8`（`rig-core`）、`:10`（`zk-llm-rig` path 依赖）；`specs/changes/2026-10-31-config-and-media/design.md:18`（“配置来自 llm-rig 的 media model builders”）。
- 问题：AGENTS C4 要求“包之间只依赖 `zk-contract-*`；跨包禁止依赖具体实现类型”；`zk-llm-rig` 是 `packages/client/*` 实现包（角色 llm.provider.default），不是 `crates/zk-kit-*`。ADR-066（docs/13:606-611）只允许 rig 适配在 `zk-llm-rig`/`zk-agent-rig`/`zk-llm-gateway` 三处；cap-* 各自 `use rig_core::*` 等于新增 4 个 rig 适配点（`docs/38:74` 自己也承认 cap-vector 需要补 ADR，但本轮没有新增/修订 ADR）。
- 修法：把 `MediaConfig`/`MediaKind`/`block_on`/`*_model[_from_config]` 移到 `crates/zk-kit-media`（非包 kit，可合法依赖 rig-core），cap-* 只依赖该 kit + 结果 DTO（或仅 `configure(DynModel<Op>)` 由宿主注入，如 cap-vector 的 `configure` 形态）；同时在 `docs/13` 增加 ADR（修订 ADR-007/066 的适配点清单）或明确媒体能力归 kit。之后 C4/C17 才成立。
- 关联：AGENTS C4/C17；ADR-007/066；docs/38 §5.4。

### P1-5 `docs/38` 状态与已提交实现矛盾（权威文档 false-green）
- 位置：`docs/38-agent-runtime.md:50-51`（“契约字段待 T22 解冻后搬入 `AppSettings.agent`”“`AgentReply.usage`/UI 持久化待 T22 契约”）与 `:74`（“契约层（T22 冻结中，本轮不改 contracts）：`AppSettings.agent` … 仍未接”）。实际 `e8137ef` 已提交这些契约/桌面接线。
- 问题：docs/38 是 agent 运行时唯一权威（AGENTS 当前阶段/ADR-064/066），状态段却把已实现项写成未接，验收和后续 agent 会据此误判；属 C12/证据政策“不得把未验证说成已验证”的反向（把已验证说成未接）。
- 修法：更新 §5.2/§5.4 状态（契约已落地、桌面已接线、usage 已进 AgentReply/UI），删除“T22 冻结中”过时表述；把 P1-2/P1-3/P2 项的剩余状态写清。
- 关联：docs/38、ADR-066；config-and-media acceptance。

### P1-6 `deny.toml` 未入库 + 本地未验证，安全门禁可能 false-green/CI 红
- 位置：`deny.toml`（untracked；`git ls-files deny.toml` 空）；`.github/workflows/security.yml:9-10` `cargo install cargo-deny … --locked` + `cargo deny check`。
- 问题：策略文件不入库则 CI 运行时没有项目策略（或 cargo-deny 用内建默认，行为与作者意图不一致）；本机未安装 cargo-deny，无法验证 `[advisories]` 旧式 `unmaintained = "warn"`、`[bans] deny=epub`、许可证白名单在 CI 安装的**最新** cargo-deny 上可用（workflow 未 pin 版本）。媒体能力新增 rig `audio/image` feature 未引入新 crate（Cargo.lock diff 无新增包），但仍需 deny 证据。
- 修法：`git add deny.toml`；CI pin cargo-deny 版本（或容器）并归档 `cargo deny check` 证据；本地/CI 验证 deny 配置（当前标 UNVERIFIED）。另外 policy 注释“copyleft 不允许”与 `MPL-2.0/OpenSSL` allow 项请写清边界（MPL 为弱 copyleft）。
- 关联：AGENTS §9/§6、security.yml。

### P1-7 C16/C19 流程未闭环：spec tasks/acceptance 未回填、review record 缺失、生成矩阵未提交
- 位置：`specs/changes/2026-10-31-config-and-media/tasks.md:5-12` 全为 `pending`；`acceptance.md:3` 仍 `in-progress`（“完成后回填”）；`python3 scripts/review-check.py` rc1（两份 proposal 指向的 `docs/reviews/2026-10-31-round2-verification.md` 当时不存在）；`docs/generated/AGENT-TOOL-MATRIX.md` + `scripts/agent-tool-matrix.py` 已修改但未提交（矩阵 truth 未随 round-2 入库）。
- 问题：round-2 已提交实现，但 C16 要求的 tasks 状态/acceptance/证据未回填，C19 review gate 红；生成矩阵更新悬空会让 docs/38 “以矩阵为准”的引用再次漂移。
- 修法：按实际门禁结果回填 tasks.md 状态与 acceptance 证据（含 fmt/clippy/test/pnpm/schema/compose/e2e；e2e 若未跑需 UNVERIFIED）；提交矩阵脚本+生成物；本记录 APPROVED 后 review-check 才会转绿。注意 acceptance 中 `cargo test --workspace export_schema && git diff --exit-code` 必须用 **workspace** invocation（见 P2-8）。

## 3. P2（建议修复/澄清，不阻塞本轮但需记录）

1. `AgentUsage` 与 spec 字段不一致：spec `spec.md:12` 列 `input_tokens, output_tokens, total_tokens, cached_tokens, reasoning_tokens` 为必有（cost 可选），实现全部为 `Option<u64>`（`packages/contracts/agent/src/lib.rs:27-39`）。更诚实的实现可保留，但需回写 spec/schema/UI 类型（当前 UI 已按 nullable 处理）。
2. usage 未持久化：design §2 说“reducer 保存到 assistant message”，实际 `apps/desktop/src/ipc.rs:942-1000` 只把 `reply.text` 写入 `Message.content`，`packages/contracts/session/src/lib.rs:52-64` 的 Message 无 usage 字段；重启后 usage 丢失。要么加持久化字段（`#[serde(default)]`），要么 design 明确“仅当前会话内存展示”。
3. usage 只取最后一次 rig 响应：`agent-rig/src/lib.rs:493-546` 仅从 `FinalResponse`/`PromptResponse` 的 usage 取数，多步工具循环的中间调用 usage 未累计（与设计 §2 一致但语义上是“最后一跳”）；请 docs 明确或聚合。
4. `MediaConfig` `#[derive(Debug, ...)]` 含 `api_key`（`packages/client/llm-rig/src/lib.rs:335` 附近），一旦 host 打印配置即泄 key；建议自定义 redacted Debug / 去掉 Debug。
5. hooks 日志安全：`agent-rig/src/hooks.rs:159` 用默认 `OpenOptions`（0644/umask）写 JSONL，事件含 `ToolStarted{args}`（原始工具参数，可能含命令/密钥）；`apps/desktop/src/ipc.rs:98-106` 的 `DesktopHookSink` 把每个 hook（含 args）`eprintln!` 到 stderr。建议 0600 + 默认不打印 ToolStarted args（或 env 开关）。
6. Ajv 严格性：`apps/desktop/web/src/validate.ts:19` `new Ajv({ allErrors: true, strict: false, logger: false })`，无 `ajv-formats`；design §4 明确选了 strict:false，但未知 keyword/`format` 被静默忽略 → C10 边界可能 false-green（例如 `format:"uint64"` 不生效；integer/minimum 仍生效）。建议 strict 模式白名单未知格式 + 增加“schema 使用了未注册 keyword 会失败”的回归测试。
7. media cap 包间重复实现：四个 cap 的 model slot/available/unavailable/configure/unconfigure/env 解析结构近似（C17），修 P1-4 时一并抽到 kit，避免四份漂移。
8. schema 生成 invocation 敏感（false-green 风险）：`cargo test -p zk-contract-config -p zk-contract-agent export_schema` 与 `cargo test --workspace export_schema` 生成内容**顺序不同**（`serde_json` 的 `preserve_order` 由 workspace 其它 crate 的 feature 统一启用；包级运行走 BTreeMap 排序）。实测同一文件 hash 由 `a1038…` 变为 `881d…`（`app-settings.json`/`agent-settings.json` 仅 key 顺序差异）。CI/验收必须用 workspace invocation；建议在 workspace 显式启用/关闭 `serde_json/preserve_order` 或让 exporter 规范化排序，避免开发者跑单包测试污染生成物。
9. `#[allow(clippy::arithmetic_side_effects)]` 在产品代码 `agent-rig/src/lib.rs:576`（USD→micros）；`docs/18` §4/C9 只明确允许测试模块 allow；建议改整型安全换算（如 `usd.to_bits`/字符串定点）或登记例外。
10. docs 漂移：`docs/38:44` 写 “`pdf` 需要未缓存的 `lopdf 0.44`”，实际 workspace `Cargo.toml:18` 为 `=0.42.0`；`specs/.../config-and-media/design.md:18` 写“rig-core 启用 features：audio,image,pdf,epub”，实际根 `Cargo.toml` 只启用 `audio,image`（pdf/epub 故意不启用）。需改 design/docs 对齐。
11. `cap-*` 包缺 `@capability` 标注（`grep '@capability' packages/client/cap-{image,audio,rerank,transcription}/src/lib.rs` 为空；capability index 未登记这些能力），C17/`zk capability` 的登记要求未满足（部分为既有占位问题，本轮实现后更应登记）。
12. `AppSettings`/`AgentSettings` schema 无 `additionalProperties:false`（schemars 默认），Ajv 边界接受未知字段；`app-settings.json` 里 `agent`/`server_url` 非 required（serde default，向后兼容正确），但若要 C10“严格边界”建议对入站 settings 增加 unknown-field 拒绝（需 contract 变更/评审）。
13. 测试性注意：cap 媒体测试用 `#[allow(clippy::panic)]`（test-only，`cap-image/tests/media.rs:1`），docs/18 只列 unwrap/expect 允许；建议保持但登记原因（低）。
14. `AgentSettings` 进程全局 registry 放在契约 crate（`packages/contracts/agent/src/lib.rs` `AGENT_SETTINGS`/`AGENT_HOOK_SINK`），契约层承载可变运行时状态，C4/架构上建议随 P1-4 移到 kit/host；至少加 ADR 说明。

## 4. 已核对通过 / 无问题项

- 契约向后兼容：`AppSettings.agent` 与 `AgentReply.usage` 均有 `#[serde(default)]`/`Option`；旧 settings JSON（无 `agent`）与旧 reply（无 `usage`）可解析；`app-settings.json` 的 required 未把新字段列为必填；`schema-check.sh` rc0。
- `Agent::configure_agent` 默认空实现不破坏既有实现；agent-loop/llm-mock 已补 `usage: None`。
- desktop Ajv 边界实际接入：`transport/http.ts`、`transport/tauri.ts` 命令响应、`controller.ts` agent 事件、`parseSettings` 均调用 `validate.ts`；负向测试覆盖缺字段/错类型/未知事件/旧 settings 缺 agent 默认。
- 设置 UI 字段与契约对齐（max_steps/completion/memory/hooks/usage），`pnpm -r typecheck/lint/test:unit/build` 全绿；desktop web vitest 55 passed。
- 媒体 cap 单元测试用 rig `Local<Op>`+fake transport 覆盖“配置后真实调用/编码探测/未配置”两条路径，无网络；未发现 panic/unsafe/裸 `as`。
- 本轮未发现 P0（无立即数据/资金/认证破坏；C7 企业媒体为潜在越界，已列 P1-3）。

## 5. UNVERIFIED（环境阻塞，不得写成已验证）

- `cargo deny check`：本机无 cargo-deny；`deny.toml` 未入库。CI 安装最新版的行为未验证。
- 真实媒体 provider（OpenAI/Voyage）：仅 fake/local 模型；真实上游与计费未验证。
- 企业网关 media 端点：明确非目标；enterprise 媒体可用性按 P1-3 评估。
- full e2e（agent 全 9 条、m1/admin/login）未在 round-2 提交后重跑；仅跑 agent-e2e 的 max-turns 子集（1 passed）。acceptance 若声明全绿需补证据。
- 系统密钥库、真实上游 Responses、100 并发压测：沿用 audit-hardening §2 UNVERIFIED。

## 6. 复评记录：round-2 P1 fixes（2026-10-31）

- 复评对象：HEAD `9ed7d40`（`0e03e66` kit 重构、`c78f3b2` config 修复、`7e5507d` web usage 镜像、`2cd690d` deny/security pin+host wiring、`9ed7d40` Ajv strict）；工作树 clean，`zk compose --check` 0。
- 复评门禁（本机实跑，exit code）：`cargo fmt` 0 · `cargo clippy --workspace --all-targets --all-features -D warnings` 0 · `cargo test --workspace` 0 · `cargo test --workspace export_schema` 0（随后 `git status` clean）· `scripts/schema-check.sh` 0 · `docs-check` 0 · `spec-check` 0 · `zk capability --check` 0（45 条）· `zk db lock --check` 0 · `zk docs --check` 0 · `zk compose personal --check` 0 · `pnpm -r typecheck/lint/test:unit/build` 0/0/0/0 · desktop web `typecheck`/`vitest` 0/0。`cargo deny check` 仍 **UNVERIFIED**（本机未安装 cargo-deny；CI 已 pin 版本，证据待归档）。
- 并发说明：复评期间另一 agent 的 compose/e2e 曾短暂把 `apps/admin/web/src/generated/registry.ts` 写成 enterprise 内容，导致一次 `pnpm -r lint` 在 `apps/admin/web/src/app.tsx:10` 误报；工作树稳定后重跑为 0，最终 `git status` clean、`compose --check` 0，非 HEAD 缺陷。

### 6.1 P1 关闭状态（全部关闭）

| P1 | 修复证据（HEAD 9ed7d40） | 状态 |
| --- | --- | --- |
| P1-1 env 优先级 | `apps/desktop/src/ipc.rs:144-149`（仅 settings 文件存在且 agent≠默认才发布）、`:173-182`；回归测试 `ipc.rs:1115-1130`；fresh install 保留 `ZEOKEEL_AGENT_*` fallback（c78f3b2） | 关闭 |
| P1-2 媒体宿主接线 | `crates/zk-composed/src/lib.rs:106` 生成 `configure_capabilities_from_env()`；generator 仅 personal 生成实际调用（`tools/zk/src/main.rs` compose）；`apps/desktop/src/main.rs:166-170` 仅 personal 调用；enterprise no-op（2cd690d） | 关闭 |
| P1-3 企业媒体 C7 | ADR-067（`docs/13-risks-and-adr.md:613`）+ 构建期 enterprise no-op 闸门；`docs/38:69` 同步 | 关闭（构建期方案，ADR 已记录） |
| P1-4 C4/ADR | 新增 `crates/zk-kit-media`（`@capability kit.media`，capability-index 45 条）；cap-* Cargo 仅 `thiserror`+kit，无 `rig-core`/`zk-llm-rig` import；ADR-067 扩展适配点 | 关闭 |
| P1-5 docs/38 真源 | `docs/38:50-51`（契约已落地）、`:73-77`（§5.4 已实现/剩余 gap）更新；docs-check 0 | 关闭 |
| P1-6 deny 策略 | `deny.toml` 已入库（2cd690d）；`security.yml` pin cargo-deny 0.20.2 / audit 0.22.2 / machete 0.9.2 / geiger 0.14.0 | 关闭（本地 UNVERIFIED，CI 证据待归档） |
| P1-7 C16/C19 | tasks.md 任务 1–7 `done`；acceptance 已回填自动验证证据；矩阵脚本+生成物已入库；本记录复审后 review-check 转绿 | 关闭（task 8 全量 e2e 由 Lead 在归档前完成，见 §6.2） |

### 6.2 复评后仍留 P2（不阻塞本轮 APPROVED，记录备查）

1. `specs/changes/2026-10-31-config-and-media/design.md:18` 仍写 rig features `audio,image,pdf,epub`，实际只启用 `audio,image`（docs/38 §5.1 已正确）——建议改 design。
2. `packages/client/cap-vector/src/lib.rs:263` 仍有产品代码 `#[allow(clippy::arithmetic_side_effects)]`（agent-rig 的 USD 换算已改为定点无 allow；cap-vector 建议同处理或登记例外）。
3. 契约 crate 仍持有进程级 `AGENT_SETTINGS`/`AGENT_HOOK_SINK` 状态（`packages/contracts/agent/src/lib.rs:259-267`）；C4 边缘，建议后续迁 kit/host 并加 ADR。
4. schema exporter 的 key 顺序仍依赖 invocation（workspace 与包级 `cargo test -p … export_schema` 因 serde_json feature 统一而顺序不同）；验收/CI 必须用 `cargo test --workspace export_schema`（本轮已用 workspace 且第二次运行零 diff）。
5. usage 未持久化到 session `Message`（重启丢失），docs/38 §5.4 已标 gap；如需展示历史需后续持久化字段。
6. enterprise 闭包仍编译 media caps/kit（ADR-067 接受“no-op 不接线”）；若未来企业版本需更硬边界，再走 personal-only 排除。
7. `apps/desktop/src/ipc.rs:126-139` 的 `settings_file_path()` 与 `store-settings-json` 的默认路径约定重复，需保持同步（建议 store 提供 path API）。
8. `cargo deny check` 本机 UNVERIFIED；acceptance 中 e2e/task 8 与真实媒体 provider 仍按 UNVERIFIED 记录，不得写成已验证。

- 复评结论：**APPROVED**（2026-10-31；P0 = 0、剩余 P1 = 0；最终评审对象 HEAD `9ed7d40`；上述 P2 不阻塞本轮关闭）。
