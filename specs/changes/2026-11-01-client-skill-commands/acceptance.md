# 2026-11-01-client-skill-commands · 验收与证据

- 验收人：impl-skill；日期：2026-11-01；PR/commit：未提交（共享工作树；起点 `39f3405`）
- 状态：`done`（本地实现与证据齐全；UNVERIFIED 见 §1）
- 证据根：`target/evidence/2026-11-01-client-skill-commands/`

## 0. 预算例外

> Lead 2026-11-01 追认：规划 10 文件；实际约 30 路径（含生成物/测试/文档/spec 记录）。仍为单目标、
> 无第二套扫描/菜单/存储实现，不拆分（同时修复了 MCP 合入后遗留的 `tools/zk` 个人版 pane 快照测试）。

## 1. 门禁

| 命令 | 结果 | 证据 |
| --- | --- | --- |
| `cargo test --workspace` | **exit 0**（443 个 test result 全 ok） | `logs/cargo-tests.log` |
| `cargo fmt --all -- --check` | **exit 0** | `logs/cargo-fmt.log` |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | **exit 0** | `logs/cargo-clippy.log` |
| `cargo check --workspace --all-targets` | **exit 0** | `logs/cargo-check.log` |
| `cargo test --workspace export_schema` 幂等 | **exit 0，STATUS/DIFF 均未变化** | `logs/export-schema.log` |
| `bash scripts/schema-check.sh` | **exit 0**（137 schemas） | `logs/schema-check.log` |
| `zk validate` / `compose --check` / `capability --check` / `docs --check` | **0/0/0/0** | `logs/zk-gates.log` |
| `pnpm -r typecheck/lint/test:unit/build` | **exit 0**（最新改动后重跑） | `logs/pnpm-gates.log` |
| `pnpm e2e` | **UNVERIFIED**：mock 8/8 绿；serve 23/24，唯一失败为既有 `/compact` P1（见 §1.1，非本 spec） | `logs/e2e-main.log`、`logs/e2e-serve-panic-rootcause.txt` |
| `pnpm e2e:ui`（全量） | **74/74 passed，exit 0** | `logs/e2e-ui-full.log` |
| `pnpm e2e:ui --grep command-skill` | **3/3 passed + 3 份证据** | `logs/e2e-interaction.log`、`interaction/` |
| `python3 scripts/ui-interaction-gate.py --self-test && --strict` | **self-test 12 checks OK；strict exit 0** | `logs/interaction-gate.txt` |
| `python3 scripts/spec-check.py` / `docs-check.py` / `review-check.py` | **0/0（2 warnings 为其它 in-progress spec）/ok** | `logs/gates-summary.txt` |
| `bash scripts/i18n-check.sh` | **ok** | `logs/gates-summary.txt` |

### 1.1 `pnpm e2e` 结果与预处理（诚实口径）

两条独立原因导致原样 `pnpm e2e` 不能全绿，均**与本 spec 的 skill 改动无关**：

1. `scripts/restore-baseline.sh` 用 `git checkout --` 恢复生成基线，会在**未提交**的生成物
   （本 spec 的 `crates/zk-composed/src/lib.rs` facade）上失败。按 plan-mode 先例执行等价序列并记录：
   mock compose → `ZEOKEEL_E2E_SKIP_SERVE=1 playwright test --grep-invert "serve carrier"` →
   **compose personal（代替 git checkout）** → `playwright test e2e/m1-serve*.spec.ts e2e/mcp-config.spec.ts`。
   mock 段 **8 passed / 21 skipped / exit 0**（见 `logs/e2e-main.log`）。
2. serve 段 **23 passed / 1 failed**：失败项是**既有** `/compact` 分隔 e2e（`m1-serve.spec.ts:278`），
   根因已定位（非本 spec）：`/compact` 在 serve 异步 handler 中直接调用 `Agent::complete_raw` →
   agent-rig 内部新建 tokio runtime `block_on` → panic「Cannot start a runtime from within a runtime」，
   请求永不返回；只有先前设置测试把 provider 持久化为非 mock 时触发（单测式隔离运行时走 mock 抽取式摘要，
   故此前记为「flaky」，见 `specs/changes/2026-11-01-client-compact-command/acceptance.md` 97f6583）。
   证据 `logs/e2e-serve-panic-rootcause.txt`。该失败不涉及 `skill.*`；skill 自身的交互证据由
   `pnpm e2e:ui --grep command-skill`（3/3）与全量 `pnpm e2e:ui`（74/74）覆盖。
3. 结论：`pnpm e2e` 原命令 = **UNVERIFIED（既有 `/compact` P1 缺陷阻塞）**；Lead 提交后需修 compact
   阻塞调用（或复跑确认）再宣称该门禁绿。本 spec 的 skill 功能无失败项。

## 2. P0/P1 交互

| ID | 优先级 | 测试 | 证据 |
| --- | --- | --- | --- |
| CMD3S-SKILL-01 | P0 | `e2e/interaction/command-skill.spec.ts` | `interaction/CMD3S-SKILL-01.json` |
| CMD3S-KEY-01 | P1 | 同上 | `interaction/CMD3S-KEY-01.json` |
| CMD3S-ERR-01 | P1 | 同上 | `interaction/CMD3S-ERR-01.json` |
| CMD3S-VIS-01 | P2 | 视觉基线（未提交快照，需人工确认） | — |

## 3. 手工/复用

- 来源：personal 只读当前 workspace `.agents/skills`（frontmatter `user-invocable != false`）；
  enterprise 只读 `$ZEOKEEL_HOME/skills/<name>/<version>` 已安装（版本序/未安装 `skill/unavailable`）；
  host 经 `zk_composed::zk_tool_skill` facade 调用，**不 direct import** 工具包（C4）；无第二套目录扫描。
- `tool-skill` 单实现：`list` 默认仍是旧换行目录名（agent 工具兼容），`format:"json"` 新格式；frontmatter
  前 4 KiB、坏 frontmatter 降级；`read`/Instructions ≤8 KiB（超限 `skill/too-large`）；名称/版本逐组件校验 +
  `safe_path`/`safe_resolve_under` no-follow。
- UI：复用 Batch1 单一 catalog/runCommand/listbox；`/skill` 从占位改为真实帮助提示；`/` 技能组动态候选
  （loading/空/错误/离线）+ 可移除 Instructions chip（不发送）；发送时 `--- skill instructions ---` 块（≤8 KiB、
  控制字符清理）与引用块共存进 `chat.send`（契约不变）；回放/刷新渲染为 chip；i18n zh/en；六态/键盘/axe。
- 回滚：移除 UI/IPC 即可；无 DB/文件迁移（`SKILL.md` 只读）；无服务端/商店后端变更；生成物由生成器写。

## 4. UNVERIFIED

- enterprise 真实商店安装 + 企业组合下的 `skill.list/read`（本机仅 personal 组合 e2e；enterprise 路径/版本/
  未安装禁用依赖纯函数单测）→ UNVERIFIED。
- 视觉基线 `CMD3S-VIS-01` 需人工确认（未提交快照、未自动接受）。
- 真人用户测试（≥5 人 think-aloud）→ UNVERIFIED；此前不得声称「符合直觉」。
- `pnpm e2e` 原样命令：未提交共享工作树 + **既有 `/compact` P1（async handler 内 `block_on` panic）** 双重阻塞（见 §1.1）；
  skill 功能本身 mock/交互 e2e 全绿，Lead 提交后需先处理 compact 阻塞再重跑确认。

## 5. 结论

- [x] `done`（UNVERIFIED：enterprise 真机商店安装、视觉基线人工确认、≥5 人用户测试、原样 `pnpm e2e` 的既有 compact 失败）

## 6. 交接给 Lead 的 P1（非本 spec 改动）

- `/compact` 在 serve 异步 handler 内直接调用 `Agent::complete_raw` → agent-rig 新建 tokio runtime `block_on`
  → panic「Cannot start a runtime from within a runtime」，请求不返回（仅非 mock provider 的 serve e2e 触发）。
  建议把 `/ipc` dispatch 放进 `spawn_blocking`（或让 summarize 走异步），另开 change 修复；本 spec 未修改 compact。
