# 2026-10-31-round7-async-and-determinism · 设计

- 状态：`approved`

## 1. 异步子代理
RigAgent 增加 `cancel_flag() -> Arc<AtomicBool>`；RigSubagentRunner 持有 `children: Mutex<HashMap<String, Child>>`，Child {flag, result: Option<Result<String,String>>, started}; start 线程 spawn，用 `RigAgent::new(...).with_cancel_flag(flag)`，完成后写 result；interrupt 置 flag；thread 结束 join 不等待（detached，结果通过 map）。工具通过 kit runner 调用。同步 subagent 仍 join。
## 2. 确定性
`fn stable_json(value:&Value)->String` 递归把 Object 转 BTreeMap；替换所有生成写点；测试。
## 3. 426
settings/updates 的 426 构造统一 helper（可放 zk-kit-server `upgrade_required_body`），读取 X-Request-Id；request_id 审计：若 migration 0006 有列则写入。
## 4. open_url
`Command::spawn` 后 `std::thread::spawn(move||{ let _=child.wait(); })`；错误返回；web catch toast。
