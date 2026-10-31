# 2026-10-31-round11-e2e-and-sbom · 验收

- 状态：`done`（C19 独立终评 APPROVED at HEAD 78c259c；首评 P1 文档命令门禁已修）
- 验收人：lead；C19 独立评审：auditor-build

## 1. 验收结果

| 命令 | 通过条件 |
| --- | --- |
| cargo fmt/clippy/test --workspace | 0 |
| zk validate/compose/capability/docs/db lock/lint/sbom | 0 |
| docs-command-check + docs/spec/review gates | 0 |
| pnpm gates + schedule e2e | 0 |
| async turn integration | 通过 |
| C19 评审 | APPROVED（auditor-rust 独立；P0/P1=0；P2 见 review §12.5） | docs/reviews/2026-10-31-round11-verification.md |

## 1.1 实现与复评记录

- `zk sbom`：CycloneDX 1.5 离线生成（Cargo.lock 全 crate + pnpm direct workspace），确定性 serial/components 排序，`--check` 逐字节；CI docker job 产出 artifact；docs/09 记录覆盖/限制（edition 过滤、pnpm transitive、tar/镜像 UNVERIFIED）。
- schedule auto-run e2e：独立 serve-schedule 脚本 + spec，真实 SQLite/假上游/到期 schedule，断言自动 turn 消息并 reload 持久化；1/1 通过。
- async real-turn 集成：真实父 turn 驱动 subagent_start，真实子线程 status/interrupt，live_count 归零；5× 稳定。
- docs-command-check：校验顶层命令与 db/channel 子命令，未知/计划命令需 backlog 标注，扫描 docs+AGENTS+README，带 `--self-test` 负向用例；check.sh/CI 接入。首评 P1（跳过未知命令/子命令）已修复；计划命令文档全部标注 backlog。
- 非阻塞 P2（review §12.5）：无官方 CycloneDX schema 校验、schedule e2e 未入 CI、schedule/m1 并发隔离、async Phase B 直接调用控制工具、marker 500 字符启发式。

## 2. UNVERIFIED

- Docker 镜像构建/SBOM 打包进 tar；真实 hosted runner；Tauri/Windows；真实模型调度压测；zk api 组合锁 backlog。
