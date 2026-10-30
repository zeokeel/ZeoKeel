# 2026-10-31-audit-hardening · 设计

- 状态：`done`

## 1. 组合与生成物

- `zk compose` 对非基线 edition 只写工作区（不提交），`--check` 仅校验基线；admin registry 在个人版生成确定性空实现，避免企业生成物泄漏。
- `zk docs` 生成 CONTRACT-INDEX/PROFILE-MATRIX；compose 仍是 PACKAGE-INDEX 唯一 writer；capability 索引扫描所有 package/kit crate 并要求 @capability。

## 2. 内核

- `crates/zk-bootstrap` 包装 `zeokeel_cordis::Context::root()`：plugin 注册、服务注入、`spawn_task`（effect + abort-on-dispose）、幂等 shutdown；实现 `zk-contract-kernel` 的 Clock/ConfigStore/TaskScheduler。
- desktop/server 在 main 中 boot；server 关闭顺序 = drain → pending → audit verify → pool close → Bootstrap::shutdown。模板 apps/*/Cargo.toml 增加 zk-bootstrap 边。

## 3. Agent/rig

- 默认映射 agent-rig + llm-rig；企业 llm-gateway 实现 `model()/model_from_config()`（`OpenAIConfig::with_base_url({server}/v1).responses(model)`），未登录返回占位模型，登录后 `Runtime::configure_llm(..., "gateway")` 热切换。
- `AgentRuntimeConfig` 优先级 显式 > env > 默认；usage 来源于 streaming FinalResponse/unary PromptResponse；rig-memory 按 conversation 持久化；HookSink 结构化事件（可选 JSONL）。cap-vector 用 rig embedding + 进程内余弦检索。

## 4. T22

- 契约 DTO derive schemars；每 crate `tests/schema.rs` export 到 shared/contracts/schema；schema-ts.py 解析 $ref/definitions/enums/AgentEvent tagged union，输出 interfaces + SCHEMAS map；admin 现有 Ajv 消费保持。

## 5. 服务端

- 计费：hold+usage_pending 单事务，usage id 服务端 UUID；settle 幂等；SSE usage 取最后一个非空 usage；cached/reasoning 分开计费。
- 审计：`audit_tx` 在业务事务内写哈希链（canonical JSON + advisory lock），所有业务域接入。
- 迁移：启动校验 checksum/顺序/重复/缺号；db lock 只追加。
- 网关路由：默认配置 → DB providers/models（priority/timeout/retries）→ 重试/failover；env 仅 bootstrap fallback。
- 认证：refresh exp/channel/device、family revoke、禁用即撤销、admin TTL/idle 设置、常量时间 CSRF、完整 claims、permissions。

## 6. 前端

- 每包 tsconfig + lint/typecheck/test:unit 脚本；根 eslint strictTypeChecked + 规则；app surface 通过生成 slot；C14 单一 effective URL（channel > settings > localStorage），登录持久化 settings+localStorage；SSE open 信号用于确定性 e2e。

## 7. 交付

- build-dist：真输入契约，缺输入 UNVERIFIED+exit2，占位/非法 exit1；upgrade-drill：--docker 真演练/无 docker exit2；release 每步 compose；Dockerfile build args channel/edition；deploy PG18/8787//data/TLS 模式。

## 8. 回滚

- 代码级 revert 分域 commit；迁移只能前向修复（回滚需 N-1 镜像兼容 expand）；无数据回滚。
