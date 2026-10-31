# 2026-10-31-round11-e2e-and-sbom · 提案

- 状态：`approved`
- Owner：lead（auditor-web / auditor-rig / auditor-build）
- 日期：2026-10-31
- 关联：round-10 review §8.3；docs/09 §7；docs/12 M6；docs/38
- 评审：docs/reviews/2026-10-31-round11-verification.md
- 变更预算：schedule auto-run e2e + async 真实 turn 集成 + `zk sbom` + docs 命令语义门禁

## 1. 背景

- round-10 P2：schedule e2e 只验证设置持久化，未验证 executor 实际 auto-run；async 无真实 agent turn 集成；docs-check 无法发现语义 false command/gate；交付 SBOM 文档要求 `zk sbom`（当前未实现）。
- 本轮补齐可自动化的证据，并把"命令存在性/文档声称"纳入脚本门禁，防止再出现 round-10 的 false claim。

## 2. 目标

- 新增 `e2e/serve-schedule.sh` + `e2e/schedule-autorun.spec.ts`：真实 serve/SQLite、确定性因到期 schedules.json、自动 turn 产生消息；清理 fixture；不干扰其他套件。
- agent-rig 真实 turn 集成测试（fake model）：模型先调用 `subagent_start`，再 `subagent_status`/`subagent_interrupt`，断言真实异步子线程/状态/取消；不改生产语义。
- `zk sbom --edition E --profile X --channel C [--output path]`：生成确定性 CycloneDX JSON（Rust workspace + pnpm direct packages；含 name/version/type/purl；按名排序；不联网），`--check` 校验输出与已存文件一致；CI/docker job 产出 artifact；docs/09 更新实现状态。
- `scripts/docs-command-check.py`：扫描 `docs/*.md`（排除 reviews/历史）中的 `zk <cmd>` 引用，与 `zk` 已实现命令表/backlog 标注对照，未标注未实现的未知命令 → 失败；接入 check.sh/CI。
- 全量门禁 + C19 APPROVED。

## 3. 非目标
- `zk api`/组合锁（仍 backlog）；Docker 镜像构建（环境无 Docker）；真实模型调度压测。
