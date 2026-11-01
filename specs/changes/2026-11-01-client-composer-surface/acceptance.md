# 2026-11-01-client-composer-surface · 验收与证据

> 本 spec 已通过 C19 独立评审（复评 APPROVED）：docs/reviews/2026-11-01-client-phase2-specs.md §15。
> C20：本目录 `intuition.md` + `intuition-walkthrough.md` 已交付（专家走查；用户测试 UNVERIFIED）。

- 验收人：impl-settings（实现者回填）
- 日期：2026-11-01
- PR/commit：未提交（共享工作树；由 Lead 汇总提交）
- 证据根：`target/evidence/2026-11-01-client-composer-surface/`（`logs/`、`screenshots/`、`interaction/`）

> 状态：`done`（实现与本地证据齐全；提交后复跑项、像素/ARIA 基线、真人用户测试见 §1 备注）

## 1. 验收命令与结果

| 命令 | 结果 | 证据路径 |
| --- | --- | --- |
| `cargo test -p zk-kit-tools`（`safe_join_under` symlink/../绝对/越界） | **exit 0**（7 个安全单测 + 12 现有） | `logs/rust-gates.log` |
| `cargo test -p zk-contract-ipc -p zk-desktop` | **exit 0**（契约 additivity/round-trip + host 5 个 attachment 单测） | `logs/rust-gates.log` |
| `cargo clippy <本次 crate> --all-targets --all-features -- -D warnings`；`cargo fmt --all -- --check` | **exit 0 / 0** | `logs/rust-gates.log` |
| `cargo check --workspace --all-targets`（跨包编译面） | **exit 0** | `logs/rust-gates.log` |
| `zk compose --edition personal --profile default --channel official --check`（生成 facade） | **exit 0**（`zk-composed` 生成 `pub use zk_kit_tools;` + dep） | `logs/compose-check.log` |
| `cargo test --workspace export_schema` | **exit 0**（新增 `ipc-attachment-save.json` / `ipc-attachment-save-result.json`，97 schemas） | `logs/export-schema.log` |
| `git diff --exit-code`（生成物） | **UNVERIFIED-until-commit**：共享工作树按约定不提交；替代证据 = 生成器二次运行字节不变 | `logs/export-schema.log` |
| `bash scripts/schema-check.sh` | **exit 0** | `logs/schema-check.log` |
| `zk validate` / `zk capability --check` / `zk docs --check` | **exit 0 / 0 / 0**（不新增 capability） | `logs/zk-validate.log`、`logs/capability-check.log`、`logs/zk-docs-check.log` |
| `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build` | **exit 0 / 0 / 0 / 0**（ui-chat 24 tests；desktop-web 130 tests） | `logs/pnpm-gates.log` |
| `bash scripts/i18n-check.sh` | **exit 0** | `logs/i18n-check.log` |
| `pnpm e2e`（m1-serve 含 `/`、`@`、附件、刷新恢复 + 既有 mock/serve/a11y） | **等价序列全绿**：mock 阶段 8 passed；serve 阶段 **19 passed**（m1-serve 7 + m1-a11y 9 + security 3，含本 spec 3 个 composer 用例） | `logs/pnpm-e2e.log`（首次）、`logs/pnpm-e2e-serve.log`（修复 usage 文案冲突后 19/19）、`logs/e2e-mock-final.log`（8/8） |
| `pnpm e2e:ui --grep composer-phase2a`（本 spec 15 个 P0/P1） | **exit 0（15/15 passed）**（完整套件运行中的本 spec 部分亦 15/15） | `logs/e2e-interaction.log` + `interaction/<ID>.json` |
| `python3 scripts/ui-interaction-gate.py` / `--strict` | **exit 0 / 0**（strict：本契约 active 强制；34 个 P0/P1 ID 全绿；intuition 24 rows） | `logs/interaction-gate.txt` |
| `pnpm e2e:ui`（完整交互套件，含基线 surface） | **33 passed / 1 failed**：唯一失败 = 基线 `CMP-VIS-01`（旧 composer 空态像素基线；本 spec 有意改变 composer 视觉，需**人工确认后** `pnpm e2e:ui:update`，agent 不得自动接受）。**像素基线 UNVERIFIED**；基线其它 18 个 P0/P1（含 composer 的行为/键盘/IME/持久化/axe 与 settings/inspector/sessions）全绿。 | `logs/e2e-ui-full.log` |
| `python3 scripts/spec-check.py` / `docs-check.py` / `review-check.py` | **exit 0 / 0 / 0** | `logs/spec-gates.log` |
| `cargo public-api diff`（契约变更） | **UNVERIFIED（本机未安装）**：新增命令/DTO 为 additive | —— |
| 用户测试（≥5 人 think-aloud） | **UNVERIFIED**（发布前执行；C20 `docs/27` §2.8） | `intuition-walkthrough.md` §4 |

**备注（e2e/基线口径）**
- `pnpm e2e` 原样执行依赖**已提交**生成基线（`scripts/restore-baseline.sh` 用 `git checkout --` + `--check`），共享工作树未提交时会在此步失败；已用 `logs/pnpm-e2e.log` 的等价序列（mock compose → mock 套件 → 恢复 personal 基线 → serve 套件）执行：本 spec 的 3 个 m1-serve composer 用例全绿，既有 mock/serve/a11y 用例全绿。提交后需原样复跑一次。
- `zk docs --check` 的漂移来自**并行会话**已合并的 `zeokeel/ui-settings-mcp` 包（HEAD 有包但 `docs/generated` 未重生成）；由生成器 `zk docs` 写回（C11 生成器所有权），不是本 spec 的源码改动。
- 视觉/ARIA 基线：`CMP2-A11Y-01` 以 axe + 显式 ARIA 断言实现（无 aria snapshot 依赖，已全绿）；`CMP2-VIS-01` 以结构/布局断言 + 截图证据实现，**像素基线 UNVERIFIED**，待人工确认后 `pnpm e2e:ui:update`（`KNOWN-LIMITATIONS.md` 已登记）。
- 交互测试中的文件上传用 `getByTestId("composer-attach-input")`（隐藏 file input 无 role/name，仅作为文档化 test hook；其它定位均为 role/accessible name/键盘序列）。

## 交互验收（ID → 测试 → 证据）

| ID | 测试落点 `e2e/interaction/composer-phase2a.spec.ts` | 证据（均通过） |
| --- | --- | --- |
| CMP2-CMD-01 | `/` 打开/过滤/↑↓/Enter 执行（新建会话）并回输入框、正文保留 | `interaction/CMP2-CMD-01.json` |
| CMP2-CMD-02 | Esc 保留草稿、disabled 只读、`/cancel`、`/retry` 门控 | `interaction/CMP2-CMD-02.json` |
| CMP2-AT-01 | `@` 文件菜单/大小写过滤/kind 标注 | `interaction/CMP2-AT-01.json` |
| CMP2-AT-02 | 引用 chip 原子添加/去重/≤5 上限 | `interaction/CMP2-AT-02.json` |
| CMP2-AT-03 | 文件列表空/加载/错误/离线状态不阻塞 | `interaction/CMP2-AT-03.json` |
| CMP2-ATT-01 | 文件选择上传、chip、`data:` 缩略图、≤5 | `interaction/CMP2-ATT-01.json` |
| CMP2-ATT-02 | 类型/大小/空文件/上传失败/上限错误与重试 | `interaction/CMP2-ATT-02.json` |
| CMP2-ATT-03 | 落 `.zeokeel/attachments/<conv>/` 且刷新恢复 | `interaction/CMP2-ATT-03.json` |
| CMP2-INFO-01 | 模型只读标签 + 点击定位 provider pane | `interaction/CMP2-INFO-01.json` |
| CMP2-INFO-02 | usage 本轮/累计、刷新一致 | `interaction/CMP2-INFO-02.json` |
| CMP2-KEY-01 | Enter/Shift+Enter/Esc 与焦点 | `interaction/CMP2-KEY-01.json` |
| CMP2-IME-01 | composing 不发送/不开菜单、词首触发 | `interaction/CMP2-IME-01.json` |
| CMP2-FILE-01 | 引用块合并/chip 渲染/刷新恢复/不展示原文 | `interaction/CMP2-FILE-01.json` |
| CMP2-A11Y-01 | axe 0 serious + `aria-haspopup/controls/activedescendant` + listbox/option | `interaction/CMP2-A11Y-01.json` |
| CMP2-VIS-01 | composer + `/` 菜单结构与布局截图（像素基线 UNVERIFIED） | `interaction/CMP2-VIS-01.json`、`interaction/CMP2-VIS-01.png` |

## 2. 手工检查

- [x] `/`：打开/过滤/↑↓/Enter/Esc/disabled/执行后清空触发文本；⌘K 与 `/` 命令集一致（`buildCommandCatalog` 单测 + e2e）。
- [x] `@`：过滤/加 chip/去重/上限/空态/错误态；引用块发送后 chip 渲染、刷新后恢复（e2e）。
- [x] 附件：拖放由 `onDrop` 接 `dataTransfer.files`、文件选择按钮；类型/大小/数量上限；失败保留草稿与重试；缩略图为 **FileReader `data:` URL**（e2e 断言 `data:image/png`）；文件落 `.zeokeel/attachments/<conv>/` 且父目录 symlink 逃逸被拒（kit + host 单测）。
- [x] 模型标签：personal `provider.model`；未配置显示「未配置模型」；点击打开设置并定位 `provider` pane（e2e；企业版为 `enterprise` pane，同一 action 参数）。
- [x] 用量：流式/完成后更新、刷新一致、无 usage 显示「用量未报告」、不显示伪造百分比（单测 + e2e）。
- [x] 键盘/IME：Enter/Shift+Enter/Esc/↑↓；composing 期间不触发；焦点回输入框（e2e 合成 composition 事件）。
- [x] 六态：空/加载/错误/离线/流式中/已取消不白屏；离线沿用既有「发送禁用 + 横幅 + 输入禁用」基线（`app-states` 单测）。
- [x] axe 0 serious/critical；无硬编码中英文（i18n gate 0）。
- [x] 兼容：无引用消息行为不变（composer.test）；引用块为纯文本，旧客户端按文本显示；删除附件文件后 chip 仍显示路径（列表渲染不依赖文件存在，不白屏）。

## 3. 文档与决策

- [x] `docs/19` §2.1：composer 输入面（`/`、`@`、附件、模型、usage、引用块、键盘/IME/六态）与上限口径。
- [x] `docs/26` §5：`workspace.attachment_save` 命令 + `attachment/*` 错误码登记（SSOT）。
- [x] `docs/28` §4：composer Phase 2a 现状 + 不注册 `ui.composer` slot。
- [x] `KNOWN-LIMITATIONS.md`：附件 ≤1 MiB/数量/位置/不直传多模态/无 GC/非内联 chip/serve body limit/视觉基线 UNVERIFIED。
- [x] proposal §7 D1–D6 拍板记录（用户 2026-11-01）；无新增 ADR/D；无新增 capability；`docs/README.md` 不适用。
- [x] C20：`intuition.md`（12 INT）+ `intuition-walkthrough.md`（3 persona × 3 task + 摩擦表；无 P0 摩擦）。

## 4. 回滚

- 回滚步骤：回退 commit；可选删除 `<workspace>/.zeokeel/attachments/`（孤儿文件不影响正确性）。
- 数据影响：无数据库迁移；`chat.send`/`AppSettings`/会话 schema 不变；引用块是普通文本，回滚后按文本显示（可读）。
- 已验证回滚：新增字段/命令均为 additive，旧客户端不调用新命令；删除附件文件不白屏（§2）。

## 5. 剩余风险 / 后续

| 项 | 影响 | 后续 |
| --- | --- | --- |
| 附件上限 1 MiB（受 serve `/ipc` 2 MiB JSON body 限制） | 中 | 后续 spec 提升 body limit + 更大附件（design §2.2） |
| chip 显示在输入框上方、非内联原子 token | 低 | Phase 3 评估 Lexical/结构化 composer |
| 图片不直传多模态（靠 `read_image`） | 中 | provider/llm 契约支持后另立 spec |
| 附件无 GC/去重 | 低 | Phase 3 会话清理/附件管理 |
| 视觉/ARIA snapshot 基线未接入（UNVERIFIED） | 低 | reviewer 确认后 `pnpm e2e:ui:update` |
| 真人用户测试未执行（UNVERIFIED） | 中 | 发布前 ≥5 人 think-aloud（`intuition-walkthrough.md` §4） |
| `@` 候选受 `workspace.files` 有界列表（depth≤2/≤300）限制 | 中 | side-panels/懒加载树扩展 |

## 6. 结论

- [x] `done`：实现与本地证据齐全（提交后复跑/像素基线/真人测试见 §1 备注与 §5）
- [ ] `abandoned`：原因：待填

## 7. 复用证据（C17）

- 搜索记录：`rg -n "AssistantComposer|composerPlaceholder|commands =|workspace.files|parseFileEntries|AgentUsage|usage" packages apps docs`；
  `./target/debug/zk find kit.tools` → `crates/zk-kit-tools/src/lib.rs`。
- 能力索引复用：`contract.ipc`（`shared/contracts/capability-index.json`）、`kit.tools`；**不新增 capability/INDEX 条目**。
- 复用清单：`commands`/`runCommand`（`App.tsx` 提升为唯一 catalog）、`workspace.files`（IPC + `parseFileEntries`）、
  `AgentUsage`/`usage` 事件、`UiActions.send/cancel`、`settings.get/set` revision（模型标签跳设置）、`ui-commands` 的 listbox 交互模式。
- 新增包内实现（不登记 INDEX）：`workspace.attachment_save` IPC + host 校验、`zk_kit_tools::safe_join_under`（`kit.tools` 内扩展）、
  `composerRefs.ts`（引用块/触发唯一实现）。
