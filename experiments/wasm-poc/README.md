# wasm-poc — S0-P2 原型：Go/Rust(wasip1) + 帧 ABI + 宿主 DB + 限额

> 目的：验证应用中心（`docs/07-server-domains.md` §7、`docs/04-contracts-and-api.md` §9、ADR-035）
> 的最关键技术风险：**Rust 宿主（wasmtime）能否用同一套 ABI 同时跑 Go 与 Rust guest，
> 并让 guest 通过宿主函数读写每应用 SQLite、受 fuel 限额约束**。
>
> 这是实验代码，不是产品代码；产品实现会进入 `packages/server/app-*`。

## 已验证

| # | 验证项 | 结果 |
| --- | --- | --- |
| 1 | 同一份 `RS<len>\n<JSON>` 帧协议 | Rust guest 与 Go guest 都能收发 |
| 2 | 宿主函数 `db.exec` / `db.query` / `log` | 两种 guest 都能 INSERT/SELECT 到同一个 SQLite 文件 |
| 3 | 每应用一个 SQLite，跨进程持久 | alice/rust、bob/go 写入后重启 host 仍可读 |
| 4 | fuel 限额 | 低 fuel 时 guest 被 trap，host 返回错误（非静默成功） |
| 5 | Go `proc_exit` 处理 | wasmtime 返回 `I32Exit(0)`，host 视为成功而不是错误 |
| 6 | golden conformance | `python3 golden/run.py` 断言两种 guest 语义一致 |

实测（本机）：Rust guest 107 KB；Go guest 4.49 MB（`-s -w` 后）；host 二进制 28 MB（wasmtime 静态链接）。

## 怎么跑

```bash
./build.sh            # 构建 host + rust guest + go guest + golden
# 手动：
target/release/zk-wasm-host --wasm guest-go/guest-go.wasm --db /tmp/app.db \
  --request '{"body":{"op":"add","name":"alice"}}'
target/release/zk-wasm-host --wasm guest-go/guest-go.wasm --db /tmp/app.db \
  --request '{"body":{"op":"list"}}'
```

## 实测输出（golden）

```json
{"ok": true, "failures": [], "rows": [{"id": 1, "name": "rust"}, {"id": 2, "name": "go"}]}
```

即：先用 Rust guest 写入 `rust`，再用 Go guest 写入 `go`，最后 Go guest 查询到两条记录——
证明两种语言共用同一帧协议、同一宿主 DB、同一 SQLite 文件。

## 工具链

- Rust 1.99.1 + `wasm32-wasip1` target
- Go 1.27.3（`GOOS=wasip1 GOARCH=wasm`）
- 首次 `cargo build --release` 约 6–7 分钟（wasmtime 依赖树大），增量约 5 秒。

## 结构

```
host/            Rust 宿主：wasmtime 50 embedding + WASI p1 + SQLite + 限额
guest-rust/      Rust wasip1 guest（serde_json）
guest-go/        Go wasip1 guest（//go:wasmimport）
golden/run.py    双 guest 语义一致性测试
```

## ABI（与产品文档一致）

- 帧：`RS<len>\n<json>`；请求/响应各一帧。
- 宿主函数（模块 `zeokeel`）：`db_exec(ptr,len) -> i32`、`db_query(ptr,len,out,cap) -> len`、
  `log(ptr,len)`、`asset_read(name,out,cap) -> len`。
- 无 AI、无 socket、无文件、无 env（WASI 未 preopen 任何目录）。

## 本原型**没有**验证（产品前必须补）

1. **超时**：当前只有 fuel（CPU 指令预算），没有墙钟超时；产品需要 epoch 中止 + 硬 kill。
2. **内存上限**：没有 `StoreLimits`；产品需要按档位限制（64/128/256 MB）。
3. **每应用并发配额**：未做；产品需要每应用/每用户/全局信号量。
4. **SQL 闸门**：当前允许任意 SQL；产品需要单语句、仅 DML、禁 DDL/ATTACH/PRAGMA、超时。
5. **asset.read**：host 里是演示资产；产品需要从 release 资源清单加载。
6. **Wasmtime LTS 锁定**：原型用 50.x；产品建议锁 LTS 48/50 并每月跟。
7. **性能**：未测高并发下的实例成本（Go 冷启动/内存显著高于 Rust）。
