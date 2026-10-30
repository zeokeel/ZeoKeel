# 2026-10-31-round7-async-and-determinism · 规格

- 状态：`approved`

## 1. 异步子代理

- kit `SubagentRunner` 新增默认方法：`start(prompt, context, depth) -> Result<String,String>`（默认 Err "async unsupported"）、`status(id) -> Option<AsyncAgentStatus>`、`result(id) -> Option<Result<String,String>>`、`interrupt(id) -> Result<(),String>`；`AsyncAgentStatus { id, depth, status, prompt }`。
- 新工具包 `tool-subagent-start`（或扩展现有 tool-subagent）：`subagent_start {prompt}` 返回 `{id}`；新工具 `tool-subagent-control` 扩展 `list_agents` 返回异步子代理，新增 `subagent_status {id}`/`subagent_interrupt {id}`（工具名按现有矩阵：`list_agents` 保留）。
- agent-rig runner：后台线程执行子代理（复用 run 逻辑），可取消（RigAgent cancel 标志改为 Arc<AtomicBool> 共享）；并发上限 8；完成/失败写结果；线程结束清理 context depth；进程退出/内核无关（工具级）。
- 测试：start→status running→result done；interrupt→cancelled；并发上限；矩阵/README/docs/38 更新。

## 2. 生成物确定性

- `zk` 写 JSON 前递归排序 object keys（`stable_json`），所有 `serde_json::to_string_pretty` 生成物（brand.json、packages.json、roles.json、ui-contributions.json、capability-index.json、api 相关）走同一函数；新增单测：不同插入顺序输入 → 字节一致；compose --check 在 `cargo build -p zk -p zk-server -p zk-desktop` 后直接跑 `target/debug/zk` 也 0。

## 3. 426 残留

- bootstrap（`/config/bootstrap`）与 updates manifest 的 426 改为 `{error:{code:"CLIENT_TOO_OLD",message,retryable:false,request_id,current?,minimum,portal_url}}`，读取/生成 x-request-id 并回显；客户端测试兼容旧 flat。
- 审计：若 audit 表已有 request_id 列，gate/handler 将 request_id 写入审计行；无列则记录后续（不新建迁移？允许新增迁移，但需 db lock 更新与不可变性）。

## 4. open_url

- Rust `system.open_url` spawn 后启动回收线程 wait（避免 zombie），失败通过返回错误/后续事件；web UpgradeGate 打开失败显示 toast/role=status，不阻塞门禁。
