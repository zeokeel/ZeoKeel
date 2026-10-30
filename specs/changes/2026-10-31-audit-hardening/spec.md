# 2026-10-31-audit-hardening · 规格

- 状态：`done`
- 范围：组合基线/内核/agent+rig/T22 后端/服务端安全/web 门禁/交付脚本/文档

## 1. 行为变化（可验证）

1. 唯一入库生成物基线 = personal×default×official；非个人版 `compose --check` 被拒绝（docs/24 §1.2）。
2. `crates/zk-bootstrap` 提供 cordis 宿主；desktop/server 先 boot 内核，server SIGTERM 后 `shutdown()`；周期任务为内核 effect。
3. personal 与 enterprise 的 `agent.loop` 均为 `zeokeel/agent-rig`；personal provider 为 `llm-rig`，enterprise 为 `llm-gateway`（rig Responses 适配，`/v1/responses`）。
4. `agent-rig` 支持 completion 参数配置、usage 采集、rig-memory 多轮、HookEvent、cap-vector 真实 embeddings/检索；cap-* 可用性动态且给出原因。
5. 契约 crate 边界 DTO 全部 `JsonSchema`；`scripts/schema-ts.py` 生成 interfaces + SCHEMAS；`schema-check.sh` 校验覆盖。
6. 服务端：usage 从 response.completed 解析（cached/reasoning 计费）、计费幂等键服务端生成、DB 驱动上游路由/failover、峰谷热配置、refresh channel/claims/TTL/CSRF、审计全业务 fail-closed、迁移 checksum/顺序启动校验、db lock append-only、advisory lock/SIGTERM、streaming overdraft。
7. web：26 个 web 包具备 lint/typecheck/test:unit；C14 地址锁定/首配/持久化/登出；个人包不导入 ui-apps；@source 完整；对话框聚焦/Esc；i18n fallback。
8. 交付：`build-dist.sh` 缺真输入 exit 2 且不产出 release；`upgrade-drill.sh` 无 docker exit 2；release.yml 先 compose edition×channel；Dockerfile 支持 channel build args。
9. `zk docs`/`--check`、`--channel` 必填、protected 角色、capability 结构性校验。

## 2. 错误与边界

- 非基线 compose --check：exit 1 + 明确错误。
- 未配置 rig provider：llm-rig/llm-gateway 返回 boot 占位模型；调用失败显式报错，不回退 mock。
- 未定价模型/余额不足：网关既有错误码；流式透支受配置下限约束。

## 3. 数据/契约兼容

- 只增字段/新增 DTO；`AppSettings` 未改（agent 配置后续接线）；`ModelPrice`/`UsageTotals` 为新增契约。
- 迁移只新增 0014/0015/0016；历史文件未改。

## 4. 验收映射

见 acceptance.md。
