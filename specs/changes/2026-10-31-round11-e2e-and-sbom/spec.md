# 2026-10-31-round11-e2e-and-sbom · 规格

- 状态：`approved`

## 1. schedule auto-run e2e
- 独立脚本 `e2e/serve-schedule.sh`：临时 ZEOKEEL_HOME/SQLite、workspace 含 `.zeokeel/schedules.json`（`origin:"tool-schedule"`、due 时间/interval=60、prompt）、fake OpenAI-compatible upstream（返回固定文本）、`ZEOKEEL_ENABLE_SCHEDULE_AUTORUN=1`、`zk-desktop --serve`；Playwright 等待自动 turn 产生 assistant 消息/会话；结束后清理。
- `e2e/schedule-autorun.spec.ts` 断言至少一条自动生成的 assistant 消息；不依赖外网；超时合理。

## 2. async 真实 turn 集成
- agent-rig 测试：fake/replay model 返回 `subagent_start` 工具调用 → 断言返回 id/status running；随后模型调用 `subagent_status`/`subagent_interrupt`（或直接 kit registry 验证）；真实子线程执行 fake model 并结束；断言终态/取消、live 计数归零；不要求浏览器 e2e。

## 3. zk sbom
- 新命令 `zk sbom --edition E --profile X --channel C [--output FILE] [--check]`：
  - 读取 workspace Cargo metadata（离线）生成 Rust components：name/version/type=library/purl=`pkg:cargo/<name>@<version>`；license 若 metadata 提供则填。
  - 读取 `pnpm-workspace.yaml`/各 package.json 生成 pnpm direct components：name/version/type=library/purl=`pkg:npm/...`。
  - 输出 CycloneDX 1.5 JSON：bomFormat/specVersion/serialNumber 确定性（nil uuid 或稳定 hash）、metadata.timestamp 固定？为可复现，不写时间或从 SOURCE_DATE_EPOCH；components 按 name/version 排序；`--check` 与 output 文件逐字节比较。
  - 不联网、不执行第三方工具；文档说明只覆盖工作区/直接 pnpm 包（transitive 由 Cargo.lock/pnpm-lock 后续）。
- CI docker job 生成 SBOM artifact；docs/09 标注实现范围/限制。

## 4. docs command gate
- `scripts/docs-command-check.py`：已知实现命令集合（validate/compose/plan/docs/capability/find/lint/channel/db lock/sbom），扫描 docs（排除 docs/reviews/**、research/）中 `zk <token>` 模式；若命令未实现且同一段落/表格未出现“未实现/backlog/计划”，报错。接入 check.sh/CI。
