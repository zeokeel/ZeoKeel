# 2026-10-31-config-and-media · 设计

- 状态：`approved`

## 1. 配置流

settings.json（store-settings-json）→ `settings.get` → AppSettings（含 agent）→ 桌面启动/settings.set
→ `zk_agent_rig::set_default_agent_config` → 每轮 `AgentBuilder` options。优先级：settings/显式 > `ZEOKEEL_AGENT_*` env > 内置默认。

## 2. usage 流

rig streaming `FinalResponse`/unary `PromptResponse` → `UsageSnapshot` → `RigAgent::last_usage()` → `AgentReply.usage`
→ 桌面 reducer 保存到 assistant message → UI 展示（tokens/缓存/推理/成本）。

## 3. 媒体

rig 媒体/embedding 适配统一在 `crates/zk-kit-media`（`kit.media`）：MediaConfig/EmbeddingConfig、模型 builders、opaque handles、调用助手；
cap-* 只依赖 kit 并做错误映射；available() 依据配置动态判断；CAPABILITY_UNAVAILABLE 保留给未配置场景。
rig-core 只启用 features：`audio,image`（pdf 依赖未缓存、epub 为 GPL-3.0，均不启用）。

## 4. Ajv

shared/contracts/ts/contracts.ts 的 `SCHEMAS` 在桌面构建期导入；新增 `validate.ts` 封装 Ajv（`strict:true`，显式注册 schemars 数字格式；未知 keyword/format 直接失败），
protocol.ts 边界先 validate 再映射；保留现有 reader 作字段级二次保护，避免一次性大改。

## 5. 回滚

契约只增字段（serde default）；媒体能力可回退为 unavailable；Ajv 校验可按函数级开关关闭并保留旧 reader。
