# 2026-11-01-client-side-panels · 验收与证据

> 本 spec 已通过 C19 独立评审（复评 APPROVED）：docs/reviews/2026-11-01-client-phase2-specs.md §15。P2-R1…R4 为批准前编辑性修复，见评审记录。

- 验收人：待独立评审 / 实现者回填
- 日期：2026-11-01（预填；实现后回填实际日期）
- PR/commit：待填
- 证据根：`target/evidence/2026-11-01-client-side-panels/`（日志 `logs/`、截图 `screenshots/`、`axe.json`、`golden/`）

> 状态：`draft`（未开工）。实现完成后按任务表逐项填证据路径；本文件不预写「已通过」。
> 范围提示：本 spec 只含右栏（文件树/预览/终端/状态持久化）+ 会话操作 + 检索入口；能力面板（jobs/subagent/goal/todo/schedule/plan/present）
> 拆到子 spec `2026-11-01-client-activity-panels`（proposal §8），不计入本 acceptance。

## 1. 验收命令与结果

| 命令 | 结果 | 证据路径 |
| --- | --- | --- |
| `cargo test -p zk-contract-session -p zk-contract-ipc` | 待跑 | `.../logs/contract-tests.txt` |
| `cargo test -p zk-store-session-sqlite`（v6 迁移/排序/fork） | 待跑 | `.../logs/store-tests.txt` |
| `cargo test -p zk-kit-session -p zk-tool-session-query`（检索单一实现） | 待跑 | `.../logs/search-tests.txt` |
| `cargo test -p zk-desktop`（files 扩展/file_read/terminal/session.*/安全边界） | 待跑 | `.../logs/desktop-tests.txt` |
| `cargo test --workspace export_schema && git diff --exit-code` | 待跑 | `.../logs/export-schema.txt` |
| `bash scripts/schema-check.sh` | 待跑 | `.../logs/schema-check.txt` |
| `zk compose --edition personal --profile default --channel official --check` | 待跑 | `.../logs/compose-check.txt` |
| `zk capability --check`（无新增条目） | 待跑 | `.../logs/capability-check.txt` |
| `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build` | 待跑 | `.../logs/pnpm-gates.txt` |
| `bash scripts/i18n-check.sh` | 待跑 | `.../logs/i18n-check.txt` |
| `pnpm e2e`（m1-serve 追加：树/预览/终端/会话操作/检索/持久化/窄窗） | 待跑 | `.../logs/pnpm-e2e.txt`、`.../screenshots/`、`.../axe.json` |
| `cargo check --workspace --all-targets` | 待跑 | `.../logs/workspace-check.txt` |
| `python3 scripts/spec-check.py && python3 scripts/docs-check.py` | 待跑 | `.../logs/spec-gates.txt` |
| **待工具落地**：`pnpm e2e:ui --grep "inspector|sessions"`（本 spec `interaction.md` 改 `active` 后；依赖 `ui-interaction-verification`，当前 UNVERIFIED） | 待跑/UNVERIFIED | `.../interaction/<ID>.json` |
| **待工具落地**：`python3 scripts/ui-interaction-gate.py`（同上；当前脚本不存在） | 待跑/UNVERIFIED | `.../logs/interaction-gate.txt` |

## 交互验收（ID → 测试 → 证据）

> 契约见本目录 `interaction.md`（`planned`；实现开始时改 `active`）。P0/P1 必须有 `@interaction:<ID>` 测试与证据
> `target/evidence/2026-11-01-client-side-panels/interaction/<ID>.json`；口径见 `docs/11` §8.6、`docs/19` §11。

| ID | 计划测试落点（`e2e/interaction/`） | 证据路径（待实现） |
| --- | --- | --- |
| INS2-TREE-01 | `inspector-files.spec.ts`：懒加载展开/排序/隐藏项 | `target/evidence/2026-11-01-client-side-panels/interaction/INS2-TREE-01.json` |
| INS2-TREE-02 | `inspector-files.spec.ts`：行内错误重试、刷新保持展开 | `.../INS2-TREE-02.json` |
| INS2-PREVIEW-01 | `inspector-files.spec.ts`：文本截断/图片预览 | `.../INS2-PREVIEW-01.json` |
| INS2-PREVIEW-02 | `inspector-files.spec.ts`：PDF/二进制/超限降级 | `.../INS2-PREVIEW-02.json` |
| INS2-PREVIEW-03 | `inspector-files.spec.ts`：路径逃逸拒绝、不存在错误 | `.../INS2-PREVIEW-03.json` |
| INS2-TERMINAL-01 | `inspector-terminal.spec.ts`：open/send/output/clear/close | `.../INS2-TERMINAL-01.json` |
| INS2-TERMINAL-02 | `inspector-terminal.spec.ts`：运行态/超时/截断/平台不可用 | `.../INS2-TERMINAL-02.json` |
| INS2-PERSIST-01 | `inspector-layout.spec.ts`：tab/宽度/折叠按会话恢复 | `.../INS2-PERSIST-01.json` |
| INS2-PERSIST-02 | `inspector-layout.spec.ts`：键盘调宽/上下限/窄窗收起 | `.../INS2-PERSIST-02.json` |
| INS2-SES2-OPS-01 | `sessions-ops.spec.ts`：rename/fork/copy id/pin/archive | `.../INS2-SES2-OPS-01.json` |
| INS2-SES2-OPS-02 | `sessions-ops.spec.ts`：C18 同 workspace、归档筛选、失败不变 | `.../INS2-SES2-OPS-02.json` |
| INS2-SEARCH-01 | `sessions-search.spec.ts`：内容模式/防抖/取消/打开会话 | `.../INS2-SEARCH-01.json` |
| INS2-SEARCH-02 | `sessions-search.spec.ts`：空/超长/无命中/错误/archived 标记 | `.../INS2-SEARCH-02.json` |
| INS2-ERR-01 | `inspector-a11y.spec.ts`：六态不白屏、状态不丢 | `.../INS2-ERR-01.json` |
| INS2-A11Y-01 | `inspector-a11y.spec.ts`：axe 0 serious + 键盘可达 + i18n | `.../INS2-A11Y-01.json` |
| INS2-VIS-01 | `inspector-a11y.spec.ts`：右栏默认态视觉基线（人工确认） | `.../INS2-VIS-01.json` |

## 1.6 直觉验收（C20）

- 文件：`intuition.md`（planned，12 条 INT，P0 全覆盖）+ `intuition-walkthrough.md`（3 persona × 3 task + 摩擦表，无 OPEN P0）。
- P0 摩擦：预览路径越界、archive 语义/跨 workspace fork、内容搜索跨 workspace 泄露——均已在 design/tasks 落最小修法并关闭。
- 3×3 persona × top task 矩阵已满格（N/A 格有理由+替代验证）；P0 无 OPEN。
- 用户测试：**UNVERIFIED**（未执行；发布前 ≥5 人 think-aloud，证据 `target/evidence/2026-11-01-client-side-panels/usability/`）；**未完成前 acceptance/发布文案不得声称「符合直觉/直观」**。
- 评审：待 fresh-context 评审员按走查复核（P0 摩擦未关闭 → 不通过）。

## 2. 手工检查

- [ ] 文件树：展开/折叠/懒加载（每目录一次请求）、刷新保持展开、空/错误/行内重试、`.git`/`.zeokeel` 隐藏
- [ ] 预览：文本（截断提示）/图片（data URL）/PDF·二进制·超限降级；路径逃逸/父目录与文件 symlink 拒绝；不存在/权限错误提示
- [ ] 文件树不展示 symlink；`workspace.files` 旧调用行为不变（P2-5）
- [ ] 终端：新建/切换/关闭、命令执行/输出追加/清空、运行态/超时/截断、平台不可用提示；命令不经审批链
- [ ] 右栏状态：Tab/宽度（拖拽+键盘）/折叠按会话持久化；刷新/切会话恢复；窄窗自动收起、变宽恢复；脏 localStorage 回默认
- [ ] 会话操作：rename（空/超长拒绝）、fork（同 workspace 复制消息、选中新会话）、copy id（剪贴板+提示）、pin 置顶、archive 隐藏/筛选/恢复
- [ ] 检索：标题模式不变；内容模式防抖/取消旧请求/空/错误/无命中；命中点击打开会话；archived 标记；query 空/超长边界；snippet±60 且 `session_search` 工具仍返回完整 content（P2-6）
- [ ] 检索隐私：只返回当前 workspace；不写日志/不上报；与打开会话同权限（P2-7）
- [ ] C18：所有会话操作保持 workspace_id；跨 workspace fork 不可达；检索只查当前 workspace
- [ ] 六态/键盘/axe：空/加载/错误/离线/流式中/已取消不白屏；axe 0 serious/critical；i18n 无硬编码
- [ ] 兼容：旧 `workspace.files` 调用不变；v5 库打开/读写正常；回滚后归档/置顶数据不影响旧 UI

## 3. 文档与决策

- [ ] `docs/26`：schema v6、pinned/archived 语义、fork/archive/检索的 C18 规则；§2 硬规则 11/13 口径更新
- [ ] `docs/16`：右栏 tab order（files 10/changes 20/terminal 30，activity 40 预留）、无新增 slot
- [ ] `docs/19` §2：右栏/会话操作/检索状态口径
- [ ] `docs/28` §4：Inspector/会话操作实现状态
- [ ] `KNOWN-LIMITATIONS.md`：终端无审批/审计、同步阻塞、PDF 降级、无多 pane/float、检索无索引、fork 复制成本
- [ ] `research/client-dsh-parity-2026-11-01.md`：缺口状态回写（右栏/会话操作/检索已实现子集）
- [ ] proposal §7 D1–D6 拍板记录已回填；无新增 ADR/D；子 spec `client-activity-panels` 已在 tasks §6 登记

## 4. 回滚

- 回滚步骤：回退 commit；SQLite v6 仅加列，旧二进制忽略新列可读；无需数据回滚
- 数据影响：pin/archive 字段被旧代码忽略；fork 生成的新会话保留（无破坏）；终端输出不持久化
- 已验证回滚：待实现后填（v5 库 + 旧列表查询可读）

## 5. 剩余风险 / 后续

| 项 | 影响 | 后续 |
| --- | --- | --- |
| 终端无审批/审计、同步 IPC（≤10s） | 中 | KNOWN 记录；Phase 3 评估流式/审计 |
| PDF/Office/HTML 不内嵌预览（CSP/平台） | 中 | D6-B 另立 spec（安全评估 + CSP） |
| 右栏单栏、无多 pane/float/fullscreen/拖拽排序 | 低 | Phase 3（xterm/多 pane 另立 spec） |
| 检索为子串遍历、无索引/高亮/滚动定位 | 中 | Phase 3 索引/高亮 spec |
| fork 复制全部消息（大会话成本） | 低 | 后续可加消息数上限/异步 |
| 能力面板未在本 spec 交付 | 中 | 子 spec `2026-11-01-client-activity-panels` |
| 预览图片经 base64（内存/带宽） | 低 | 后续可换自定义协议/流式 |

## 5.5 评审条目关闭（2026-11-01 Phase 2 评审 §7）

| 条目 | 处理 | 验证 |
| --- | --- | --- |
| P1-5 交互门禁不可执行 | tasks/acceptance 标「依赖 `ui-interaction-verification` 落地（当前 UNVERIFIED）」；不声称已绿 | 实现前确认脚本/`e2e:ui` 存在 |
| P2-4 共享文件 owner | tasks §5 增 `e2e/m1-serve.spec.ts`（composer 先、本 spec 后）与 docs owner | tasks §5 |
| P2-5 symlink 选择 | design/spec 统一「不展示 symlink + 预览拒绝」 | host/UI 单测 + e2e |
| P2-6 检索 snippet 兼容 | kit 返回 snippet，`session_search` 工具仍返回完整 content；tasks #5 加一致性断言 | kit/tool 单测 |
| P2-7 检索隐私 | spec R6/design 明确同权限、不写日志/不上报、不跨 workspace；snippet 显示 | 代码评审 + 负向检查 |
| P2-11 @generated 模板 | 改用生成器源 `tools/zk/src/main.rs` 生成 `zk-composed` facade；不手改模板/产物 | `zk compose --check` |
| P2-15 docs/11 基线 | 由 `ui-interaction-verification` 落地（当前 docs/11 已列 activity planned 但门禁脚本未落地）；本 spec 不显式改 docs/11 | 复评核对 |

## 6. 结论

- [ ] `done`：证据齐全，可合并（且复评通过）
- [ ] `abandoned`：原因：待填

## 7. 复用证据（C17）

- 搜索记录：`rg -n "workspace.files|inspector|session_search|list_conversations|persistent|terminal" packages apps crates docs`；
  `./target/debug/zk find kit.session` / `kit.tools`；capability 索引 `contract.session`（`shared/contracts/capability-index.json:63`）、
  `contract.ipc`（`:43`）、`kit.session`（`:123`）、`kit.tools`（`:131`）
- 复用清单：`WorkspaceFilesResult`/`FileEntry`、`zk-kit-tools::persistent`、`SessionStore`/SQLite store、`session_search` 逻辑（提升 `zk_kit_session::search`）、
  `ui-sessions` 菜单/Modal/焦点、`App.tsx` 右栏状态与 ⌘B、`SlotErrorBoundary`/六态模式
- 新增包内实现（不登记 INDEX）：`workspace.file_read`/`terminal`/`session.*` IPC 与 host helper；`ui-inspector/panels.tsx`；
  `zk_kit_session::search`（`kit.session` 内扩展，不新增 capability id）
- 无第二份实现：终端复用现有持久会话；检索工具与 IPC 共用 kit 函数；文件列表扩展既有 `workspace.files`（不建文件服务）

## 8. 已记录拆分（proposal §8 汇总）

- 本 spec：右栏（文件树/预览/终端/状态持久化）+ 会话操作（rename/fork/copy id/pin/archive）+ 检索入口，28 文件。
- 子 spec：`specs/changes/2026-11-01-client-activity-panels/`（五件套 draft 已写，与本 spec 同批评审：jobs/subagent/agent-team/goal/todo/schedule/plan 展示/present +
  右栏 activity tab order 40 + chat turnTail），其 proposal §7 D1–D4 待拍板；本 acceptance 不包含其功能。
