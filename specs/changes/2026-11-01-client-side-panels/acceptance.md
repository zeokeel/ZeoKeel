# 2026-11-01-client-side-panels · 验收与证据

> 提交后复跑（Lead，HEAD `324b324`）：`pnpm e2e` **23 passed**、`pnpm e2e:ui` **50 passed**（当时基线）。

> C19 独立评审通过（docs/reviews/2026-11-01-client-phase2-specs.md §15）；C20 `intuition.md` + `intuition-walkthrough.md` 已交付（专家走查；真人用户测试 UNVERIFIED）。
> 实现者：impl-settings（Phase 2b）；证据根 `target/evidence/2026-11-01-client-side-panels/`。

- 验收人：impl-settings（实现者回填）
- 日期：2026-11-01
- PR/commit：未提交（共享工作树；由 Lead 汇总提交）
- 状态：`done`（本地实现与证据齐全；UNVERIFIED 项见 §1 备注）

## 1. 验收命令与结果

| 命令 | 结果 | 证据 |
| --- | --- | --- |
| `cargo test -p zk-contract-session -p zk-contract-ipc` | **exit 0**（Conversation 默认值/trait 默认实现 + Phase 2b payload round-trip） | `logs/cargo-contracts.log` |
| `cargo test -p zk-store-session-sqlite` | **exit 0**（v6 迁移幂等/排序/fork/rename/pin/archive + 既有迁移；首轮曾因 fork 同 `created_at_ms` 排序断言 flaky，已改为集合断言并复跑 11+3 全绿） | `logs/cargo-store.log` |
| `cargo test -p zk-kit-session -p zk-tool-session-query` | **exit 0**（`search` snippet/大小写/limit + 工具完整 content 兼容） | `logs/cargo-kit.log` |
| `cargo test -p zk-desktop` | **exit 0（76 passed）**（files 旧/懒加载、file_read 文本/图片/降级/逃逸、terminal open/send/close/截断、session update/fork/search 隔离） | `logs/cargo-desktop.log` |
| `cargo clippy <changed crates> --all-targets --all-features -- -D warnings`；`cargo fmt --all -- --check` | **exit 0 / 0** | `logs/cargo-clippy.log`、`logs/cargo-fmt.log`、`logs/final-sweep.log` |
| `cargo check --workspace --all-targets` | **exit 0** | `logs/cargo-check.log` |
| `cargo test --workspace export_schema` + `scripts/schema-ts.py` + `schema-check` | **exit 0**（新增 10 个 schema → 107 schemas；二次生成 sha256 不变） | `logs/export-schema.log` |
| `zk validate/compose --check/capability --check/docs --check` | **exit 0 / 0 / 0 / 0** | `logs/zk-gates.log`、`logs/final-sweep.log` |
| `pnpm -r typecheck/lint/test:unit/build` | **exit 0 / 0 / 0 / 0** | `logs/pnpm-typecheck.log`、`logs/pnpm-lint.log`、`logs/pnpm-unit.log`、`logs/pnpm-build.log` |
| `pnpm e2e`（含 m1-serve Phase 2b：树/预览/终端/会话操作/检索/持久化） | **mock 阶段 exit 0；serve 阶段 10/10 passed**（含 3 个新 Phase 2b 用例；按等价序列执行，见备注） | `logs/e2e-main.log`、`logs/gates-run.log` |
| `pnpm e2e:ui` 本 spec 16 个 P0/P1 | **16/16 passed（exit 0）**，16 份证据 + `INS2-VIS-01.png` | `logs/e2e-interaction.log` + `interaction/<ID>.json` |
| `python3 scripts/ui-interaction-gate.py` / `--strict` | **self-test 0 / gate 0 / strict 0**（enforced P0/P1=48；intuition 3 contracts/36 rows） | `logs/interaction-gate.txt` |
| `python3 scripts/spec-check.py` / `docs-check.py` / `review-check.py`；`i18n-check` | **0 / 0 / 0 / 0** | `logs/final-sweep.log` |
| 视觉基线 `INS2-VIS-01` 像素 / `INS2-A11Y-01` aria snapshot | **UNVERIFIED**：需人工确认后 `pnpm e2e:ui:update`（agent 不自动接受）；当前以结构/axe/键盘断言 + 截图证据替代 | `interaction/INS2-VIS-01.png` |
| 真人用户测试（≥5 人 think-aloud） | **UNVERIFIED**（发布前执行；C20） | `intuition-walkthrough.md` §4 |
| `cargo public-api diff` | **UNVERIFIED**（本机未安装；新增均为 additive） | —— |

## 2. 手工检查（对应 tasks/spec R1–R7）

- [x] R1 懒加载树：根项目录优先/名称排序；展开只取直属子项；隐藏 `.git`/`.zeokeel`；符号链接不跟随/不展示；行内 loading/error+重试；刷新保持已展开路径；旧 `workspace.files` 无参行为不变。
- [x] R2 预览：文本 ≤256 KiB 截断标记；png/jpeg/webp/gif ≤8 MiB；PDF/Office/HTML/二进制/非 UTF-8/超限降级（文件名/大小/支持类型）；路径逃逸/`..`/绝对/符号链接/缺失拒绝且只读。
- [x] R3 终端：open/send/list/close；输出 stdout/stderr 追加；运行态/清空/关闭；输出 ≤64 KiB 截断标记；无工具行/审批；平台无 shell 错误可读。
- [x] R4 布局：会话级 tab/宽度/折叠 localStorage（`zk.inspector.v1.<conversationId>`）；默认 files/288/展开；宽度 240–720 且 ≤视口 50%；分隔条 aria + 键盘；窄窗 <768 自动收起、变宽恢复；脏数据回默认。
- [x] R5 会话操作：`⋯` rename（非空 ≤200，空/超长拒绝）/fork（同 workspace、复制消息、选中副本、不继承 pin/archive）/copy id（成功提示）/pin（置顶排序）/archive（默认列表隐藏、可筛选恢复）；C18 workspace 不变、跨 workspace 拒绝；失败可读、列表不变、流式中 archive/fork 禁用。
- [x] R6 检索：内容模式 `session.search`（当前 workspace、防抖 ≥250ms、取消旧请求、snippet、点击打开、归档标记、空/超长/无命中状态）；工具 `session_search` 仍返回完整 content。
- [x] R7 六态/i18n/axe/键盘：相关区域六态占位/重试；文案 zh/en；axe 0 serious；键盘 tab 切换/树展开/菜单/终端输入/搜索可达；无新增 slot/包/运行期加载。

## 3. 交互 ID 覆盖

| ID | 测试（`e2e/interaction/side-panels.spec.ts`） | 证据 |
| --- | --- | --- |
| INS2-TREE-01 | 懒加载展开/隐藏 `.git`/`.zeokeel`/折叠 | `INS2-TREE-01.json` |
| INS2-TREE-02 | 目录错误隔离 + 重试 + 刷新保持展开 | `INS2-TREE-02.json` |
| INS2-PREVIEW-01 | 文本/截断/图片预览 + 返回树 | `INS2-PREVIEW-01.json` |
| INS2-PREVIEW-02 | PDF 降级说明 | `INS2-PREVIEW-02.json` |
| INS2-PREVIEW-03 | 逃逸/绝对/缺失只读拒绝 | `INS2-PREVIEW-03.json` |
| INS2-TERMINAL-01 | open→send→output→clear→close | `INS2-TERMINAL-01.json` |
| INS2-TERMINAL-02 | 超 64 KiB 输出截断标记 | `INS2-TERMINAL-02.json` |
| INS2-PERSIST-01 | tab/宽度/折叠按会话恢复 | `INS2-PERSIST-01.json` |
| INS2-PERSIST-02 | 键盘调宽 + 边界 + aria | `INS2-PERSIST-02.json` |
| INS2-SES2-OPS-01 | rename/fork/copy id/pin/archive 一致性 | `INS2-SES2-OPS-01.json` |
| INS2-SES2-OPS-02 | archive 恢复 + 跨 workspace 拒绝 + 失败不变 | `INS2-SES2-OPS-02.json` |
| INS2-SEARCH-01 | 内容搜索命中并打开会话 | `INS2-SEARCH-01.json` |
| INS2-SEARCH-02 | 空/无命中/超长/归档状态 | `INS2-SEARCH-02.json` |
| INS2-ERR-01 | 错误态 + 重试，侧栏仍可用 | `INS2-ERR-01.json` |
| INS2-A11Y-01 | axe 0 serious + 键盘遍历 | `INS2-A11Y-01.json` |
| INS2-VIS-01 | files 默认布局截图（像素基线 UNVERIFIED） | `INS2-VIS-01.json` + `.png` |

## 4. 文档与决策

- [x] `docs/26`：schema v6（pinned/archived）+ §2 规则 11 + §5.1 新增错误码（workspace/path-escape、file/*、terminal/*）。
- [x] `docs/16`：右栏 tab order（files 10/changes 20/terminal 30/activity 40 预留），不新增 slot/settings pane。
- [x] `docs/19` §2.0：右栏与会话操作状态口径；`docs/28` §4 Inspector 现状。
- [x] `KNOWN-LIMITATIONS.md`：终端无审批/审计、无 PTY、预览上限、懒加载上限、检索口径、像素/aria 基线 UNVERIFIED。
- [x] 决策只在 proposal §7（D1–D6 用户 2026-11-01 拍板），不新增 docs/15 D 编号；无新增 capability/slot/包。

## 5. 回滚

- 回退 commit；SQLite v6 仅加列，旧二进制忽略新列可读；引用/归档/置顶数据回滚后不影响旧 UI（字段忽略）。终端不持久化，无需清理。

## 6. 剩余风险 / 后续

| 项 | 影响 | 后续 |
| --- | --- | --- |
| 终端无 PTY/交互式程序、无审批 | 中 | 后续 spec 评估 PTY/审计 |
| 预览不支持 PDF/Office 内嵌 | 低 | 保持诚实降级（CSP 不变） |
| 文件树无监听/自动刷新 | 低 | 手动刷新 + 后续 watch |
| 检索无索引/高亮 | 低 | 大数据量再评估 |
| 像素/aria 基线待人工确认；用户测试未执行 | 中 | reviewer + ≥5 人用户测试 |
| `@`/检索仅当前 workspace（设计） | — | 保持 C18 |

## 7. 结论

- [x] `done`：本地实现与证据齐全（UNVERIFIED 项见 §1）
- [ ] `abandoned`：——

## 8. 复用证据（C17）

- 复用：`safe_join_under`/新增 `safe_resolve_under`（kit 单一实现，host 经 `zk_composed::zk_kit_tools`）、`zk-kit-tools::persistent`（终端复用持久 bash/pwsh）、`zk_kit_session::search`（IPC 与工具单一实现）、`SessionStore` 契约 DTO、`workspace.files`/`workspace.changes`（扩展而非复制）、ui-kit/`ui-sessions`/`ui-inspector` 现有原语。
- 新增包内私有实现：`composerRefs` 之外的懒加载/预览/终端视图状态（ui-inspector 内）与 `side-panels.spec.ts`；无新建 `utils/helpers`、无第二套文件/终端/检索实现。

## 9. 备注（e2e 口径）

- `pnpm e2e` 原样执行依赖已提交生成基线（`scripts/restore-baseline.sh` 用 `git checkout --` + `--check`），共享工作树未提交时该步不可用；已用等价序列（mock compose → mock 套件 8/8 → 恢复 personal 基线 → m1-serve 10/10）执行，证据见 `logs/e2e-main.log`、`logs/gates-run.log`。提交后请原样复跑一次。
- 生成物幂等：`cargo test --workspace export_schema` + `scripts/schema-ts.py` 二次运行后 `shared/contracts/{schema,ts}` sha256 不变；`compose --check` 与 `cargo fmt --check` 同时绿（generator facade 的 `pub use` 顺序按 rustfmt 排序输出）。
