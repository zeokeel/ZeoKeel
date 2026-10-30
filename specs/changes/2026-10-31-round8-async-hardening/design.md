# 2026-10-31-round8-async-hardening · 设计

- 状态：`approved`

## 1. async
agent-rig runner: `live: Arc<AtomicUsize>` + `CapGuard` Drop 减；`AsyncChild { state: ChildState, result }`，state 枚举 atomic/CAS under mutex；线程 catch_unwind；完成后 retain ≤64。测试用 runner 内部 fake spawn hook 或直接调用 start + 等待。
## 2. request-id
apps/server/main.rs 或 zk-srv-http 加 `request_context_layer`（最外层，包裹全部 routes + CORS 顺序明确）；kit task-local set；gate/bootstrap/updates 不再自生成；gateway gate layer on `/v1/*`；llm-gateway 在请求头加 env header。
## 3. writer
docs/24 §3 改一行；README；check.sh/ci.yml 增加 matrix check。
