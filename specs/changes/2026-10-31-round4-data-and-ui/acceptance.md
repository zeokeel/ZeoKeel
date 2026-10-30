# 2026-10-31-round4-data-and-ui · 验收

- 状态：`done`（实现+P1/P2 修复完成；C19 复评 APPROVED at HEAD 67a7e7a）
- 验收人：lead；C19 独立评审：fixer-server（非实现者，APPROVED）

## 1. 验收结果

| 命令 | 通过条件 |
| --- | --- |
| cargo fmt/clippy/test --workspace | 0 |
| export_schema/schema-check/generated clean | 0 |
| zk validate/compose personal --check/capability/docs/db lock/lint duplicates | 0 |
| pnpm -r typecheck/lint/test:unit/build | 0 |
| m1/agent/login/admin e2e + workspace.changes 场景 | 全绿 |
| updates/oem/portal e2e | 0 或明确 UNVERIFIED |
| usage/settings/changes 单测 | 通过：store v4 跨实例/损坏降级、真实 IPC 优先级、changes fsmonitor 负向、usage UI 恢复 116 tests | 各 crate tests |
| updates/OEM/portal e2e | updates 0、apps 0、oem 0、admin 11 passed；login 未重跑（脚本端口耦合），m1/agent 另行验证 | target/evidence/audit/round4-delivery.md |

## 1.1 交付复核发现与修复

- P1-A SIGTERM 总预算 30s（默认，可配 clamp 5..120）+ 有界 bootstrap shutdown；实测正常 197ms exit0、DB 竞态 ~30s exit1。
- P1-B updates：ETag/304、forced/min_supported 可配、签名字段+配置密钥 fail-closed、Range/206/416、更新清单 426 CLIENT_TOO_OLD。
- P1-C bootstrap：技能/连接器/应用目录、品牌、权限、config_etag/ETag/304、426/503。
- P1-D portal：`/`（+`/portal`）读取 CLIENT-RELEASE 平台卡片、HTML 转义（OEM 注入测试）。
- P1-E `/api/channel` public+ETag/304。
- C19 P1 修复：usage UI 恢复、tools-only 不再压过 env（registry clear + precedence）、changes git fsmonitor/env 硬化、host DTO 契约序列化。
- 保留 UNVERIFIED：全局业务面 426 层、真实签名密钥/证书、Docker/多平台/安装器、真实 OEM 私有索引/submodule（dir_hash 现状）、cargo-deny 本地、Tauri 窗口行为、超大仓库 changes 行为。
- 非阻塞 P2（review §3）：workspace.changes 无 Playwright e2e（单测/fake transport）；IPC 同步最多 5s；2MiB 截断/超时语义；question_asked null 容忍；v3→v4 无备份说明；From<&AppSettings>/set_default_agent_config 遗留语义。

## 2. UNVERIFIED

- Docker/多平台/SBOM；真实媒体 provider；cargo-deny 本地。
