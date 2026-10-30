# WASM 应用语言范围调研（2026-10-30）

> 结论先行：Wasmtime 50 同时支持 wasip1 core module 与 wasip2 component；但 v1 建议只把 **Go(wasip1) + Rust(wasip1)** 作为 Tier-1，TinyGo 作为预览，其余语言等 p2/WIT 工具链成熟后再开。理由是 p1 的扁平 ABI 最容易做到语言无关，而 p2 各语言工具链完成度差异很大。
>
> 下文“实测”均在本机 `/tmp/wasm-research` 完成，Wasmtime 50.0.1、Rust 1.99.1、Go 1.27.3、TinyGo 0.44.0、wasi-sdk 34、Zig 0.17.0；未测项明确标“未验证”。
>
> **最大风险**：Go 每请求实例的启动/内存成本（2.4 MB、45 ms、+9 MB RSS）叠加高并发，以及 Go/Rust 两套 SDK 的 ABI 漂移。前者用配额/缓存/池化，后者用同一组 golden frame conformance 测试兜底。

## 1. Wasmtime 50 的目标支持

| 目标/能力 | 支持情况 | 稳定性与限制 | 本机核验 |
|---|---|---|---|
| `wasm32-wasip1` | 支持 core module，导入 `wasi_snapshot_preview1` | WASI p1 已冻结、不再新增 API；Rust 目标 Tier 2；Go/TinyGo/C/Zig 的主目标 | Rust/Go/TinyGo/C/Zig hello 均跑通 |
| `wasm32-wasip2` | 支持 component；`wasi:cli/run@0.2`；Rust 产物导入 `wasi:cli/*@0.2.12` | component-model 在 Wasmtime 是 Tier 1，但提案未到 phase 4，fuzzing 仍 WIP；Rust p2 目标 Tier 2 且“not tested in CI” | Rust p2 hello 52,639 B 跑通 |
| `wasm32-unknown-unknown` | 支持 core module，但无 WASI | 无 `_start`、文件/网络/时钟；`std::fs`/`println!` 等大量失效；只能 `--invoke` 或自定义 host | `add` 导出经 `--invoke` 返回 5 |
| threads（shared memory + atomics） | 支持，Tier 2 | 共享内存默认关闭，需 `-W shared-memory=y`；pooling allocator 不支持；fuzzing 缺；wasi-threads 需 `-S threads`（实验，p1） | WAT atomic 在默认配置下实例化失败，加 `-W shared-memory=y` 后返回 0 |
| SIMD | 支持，Tier 1，默认开 | Rust 默认不生成 SIMD，需 `-Ctarget-feature=+simd128`；Go 1.27 的 `simd/archsimd` 仍是实验 | `v128` WAT 返回 6 |
| memory64 | 支持，Tier 1，默认开 | Rust `wasm64-unknown-unknown` 是 Tier 3、无预编译 std；p1 + memory64 不是主流组合 | 64-bit memory WAT 返回 42 |
| wasip3 | 有 `-S p3` 入口，组件异步 | 2026 年才起步；Rust 1.100 才 stable `wasm32-wasip3`；生态少 | 未测；不纳入 v1 |

补充：

- 生产版本不要直接跟 `50.0.1`：Wasmtime 普通版本只支持 2 个月，LTS 是 12 的倍数（当前 `48.0.8` LTS，支持 24 个月）。建议锁 LTS 48 或 50 并每月跟，先用同一套 conformance 复测（[发布流程](https://docs.wasmtime.dev/stability-release.html)）。
- p1 与 p2 在 Wasmtime 里是两种装载形态：p1 是 core module，p2 是 component；CLI 会自动接 WASI，但**只接 WASI**，自定义 import 会直接实例化失败（[CLI 文档](https://docs.wasmtime.dev/cli-options.html)）。
- 另外已验证 `-W timeout=100ms`、`-W fuel=...` 能在死循环上触发 trap；`-W max-memory-size` 让 `memory.grow` 返回 -1。服务端应使用 embedding 的 epoch/fuel/StoreLimits，而不是依赖 CLI。

## 2. 语言对比

“hello 大小/冷启动”是 hello world 实测：冷启动含 wasmtime 进程与 JIT，基线约 11–15 ms，所以差异主要来自 guest 运行时。大小均已 strip（Rust/Go/C 显式 strip，Zig ReleaseSmall）。

| 语言/工具链 | 目标与官方程度 | p2 | hello 大小 | 冷启动 | 调试 | 主要限制 |
|---|---|---|---|---|---|---|
| Rust 1.99 + rustup | `wasm32-wasip1`/`wasip2` 均 Tier 2；p1 自带 sysroot | 原生 p2 | 37.6 KB / 52.6 KB | 15.9 / 16.0 ms | DWARF、wasmtime gdbstub/LLDB、wasm-tools 好 | 学习曲线；依赖审计；p2 目标未进 CI |
| Go 1.27.3 | `wasip1` 官方端口（1.21 起） | componentize-go | 2,426,805 B | 45.5 ms | Delve 的 wasip1 调试未验证；backtrace 可用 | GOMAXPROCS=1；`net` 是 fake 实现；体积/启动/内存最大 |
| TinyGo 0.44 | p1/p2 文档均支持 | 原生 p2 | 657,945 B | 18.1 ms | GDB/DWARF，但语言特性缺失 | wasm 上 `recover` 不支持；reflect/std 不全；GC 较慢 |
| C/C++ wasi-sdk 34 | p1 主目标；C/C++ 无集成工具链 | wit-bindgen + wasm-tools adapter | 4,201 B | 15.7 ms | DWARF、gdb/lldb 好 | 内存安全；p2 需手工 adapter；生态要自己移植 |
| Zig 0.17 | `wasm32-wasi`，std 一等支持 | 无官方 WIT 工具链 | 31,437 B | 12.4 ms | DWARF/gdb | 生态小；p2 未覆盖 |
| AssemblyScript | 官方文档未列 WASI 目标；WASI 靠第三方 `as-wasi` | 无 | 未测 | 未测 | source map | 非官方 p1/p2；不适合作为应用中心主语言 |
| Kotlin/Wasm 2.4.x | `wasmWasi` 仅 WASI 0.1；Kotlin/Wasm 仍 Beta | 计划中 | 未测（运行时大） | 未测 | 有限 | 依赖 exception-handling；p2 未到；运行时大 |
| .NET 10 + componentize-dotnet | WASI 0.2 组件；NativeAOT-LLVM 实验 | 原生 p2 | 未测（大） | 未测 | 有限 | 实验工具链；编译/体积重 |
| Swift 6.2 | 官方 Wasm SDK；WASI（按 p1 使用） | 未见官方 p2 | 未测 | 未测 | Swift DWARF | p2 未验证；运行时较大；Embedded Swift 可缩小 |
| Python CPython WASI | Tier 2；需从源码交叉编译 | componentize-py | 未测（解释器大） | 未测 | 有限 | Pyodide 是 Emscripten，不能直接跑在 wasmtime；启动/体积大 |
| JS StarlingMonkey/jco | WASI 0.2 组件；Bytecode Alliance | 原生 p2 | 未测（引擎大） | 未测 | JS 工具链 | 引擎大；p1 需 QuickJS 等额外方案 |
| Scala | 未找到官方 wasip1/p2 后端 | 无 | — | — | — | Scala.js 产 JS；不适用（未验证） |

分语言要点：

- **Rust**：p1/p2 都是 Tier 2；p1 自带 wasi-libc sysroot，`rustup target add` 即可，不需要 wasi-sdk；p2 支持 std 但文档明确“未在 CI 测试”。`wasm32-wasip1-threads` 也是 Tier 2，但标注实验（[p1](https://doc.rust-lang.org/rustc/platform-support/wasm32-wasip1.html)、[p2](https://doc.rust-lang.org/rustc/platform-support/wasm32-wasip2.html)、[threads](https://doc.rust-lang.org/rustc/platform-support/wasm32-wasip1-threads.html)）。
- **Go**：1.21 加 wasip1；1.24 加 `go:wasmexport`、`-buildmode=c-shared` reactor、显著降低初始内存；1.26 继续缩小 heap chunk；1.27 才把 wasm SIMD 放进实验性的 `simd/archsimd`。实测 `GOMAXPROCS=1`，`net` 是 `net_fake.go` 的进程内假网络，不能做真实 socket（[1.21](https://go.dev/doc/go1.21)、[1.24](https://go.dev/doc/go1.24)、[1.26](https://go.dev/doc/go1.26)、[1.27](https://go.dev/doc/go1.27)、[net_fake.go](https://github.com/golang/go/blob/master/src/net/net_fake.go)）。
- **TinyGo**：p1/p2 都支持，`GOOS=wasip1 GOARCH=wasm` 即可；但要 reactor 导出时用 `-buildmode=c-shared`，否则 `//go:wasmexport` 会在运行时初始化前被调用。`recover` 在 wasm 不支持，reflect/std 有缺口，GC 明显慢（[WASI 指南](https://tinygo.org/docs/guides/webassembly/wasi/)、[语言支持](https://tinygo.org/docs/reference/lang-support/)）。
- **C/C++**：wasi-sdk 34 是标准 p1 工具链；p2 要 `wit-bindgen` + `wasm-tools component new`，C/C++ 没有一体化工具链。优势是 4 KB 级体积和成熟调试，代价是内存安全与移植成本（[wasi-sdk](https://github.com/WebAssembly/wasi-sdk)、[组件文档](https://component-model.bytecodealliance.org/language-support.html)）。
- **Zig**：0.17 用 `-target wasm32-wasi`，std 对 WASI 是一等支持；p2 没有官方 WIT 工具链（[Zig 文档](https://ziglang.org/documentation/0.17.0/)）。
- **Kotlin/Wasm**：`wasmWasi` 目前只支持 WASI 0.1，0.2 计划中；Kotlin/Wasm 仍是 Beta，且默认使用 exception-handling proposal（[Kotlin WASI](https://kotlinlang.org/docs/wasm-wasi.html)、[概览](https://kotlinlang.org/docs/wasm-overview.html)）。
- **.NET**：componentize-dotnet 把 NativeAOT-LLVM、wit-bindgen、wasm-tools、wasi-sdk 串起来，输出 WASI 0.2 组件；底层 NativeAOT-LLVM 仍是实验分支（[componentize-dotnet](https://github.com/bytecodealliance/componentize-dotnet)、[NativeAOT-LLVM](https://github.com/dotnet/runtimelab/tree/feature/NativeAOT-LLVM)）。
- **Swift**：Swift 6.2 起官方分发 Wasm SDK，文档示例是 WASI 命令程序；Embedded Swift 可显著缩小体积，但 p2 未在文档中承诺（[Swift Wasm](https://www.swift.org/documentation/articles/wasm-getting-started.html)）。
- **Python**：CPython 的 WASI 构建是 Tier 2，需 wasi-sdk + wasmtime 从源码交叉编译；componentize-py 输出 p2 组件，但会把解释器打进 wasm。Pyodide 是 Emscripten 构建，不能直接作为 wasmtime 的 p1 guest（[CPython WASI](https://github.com/python/cpython/blob/main/Platforms/WASI/README.md)、[componentize-py](https://github.com/bytecodealliance/componentize-py)、[Pyodide FAQ](https://pyodide.org/en/stable/usage/faq.html)）。
- **JS**：StarlingMonkey（SpiderMonkey）以 WASI 0.2 组件为目标，jco/componentize-js 负责组件化；适合 p2，不适合 v1 的 p1 扁平 ABI。QuickJS 可编到 p1，但不是 turnkey 方案（[StarlingMonkey](https://github.com/bytecodealliance/StarlingMonkey)、[jco](https://github.com/bytecodealliance/jco)）。
- **AssemblyScript / Scala**：AssemblyScript 官方文档未列 WASI 目标，WASI 仅第三方 `as-wasi`；Scala 无官方 wasip1/p2 路径。两者都不建议进入 v1。

## 3. 关键工程问题

| 问题 | 推荐做法 | 对语言选择的影响 |
|---|---|---|
| 文件系统 | 默认零 preopen；UI 资源走 `asset.read`；如必须 preopen，只读、按应用目录、canonicalize | 所有 p1 语言都用同一套 WASI fd；Go/Rust/C 的 `std::fs` 行为一致，无需语言特判 |
| 数据库 | **宿主提供 `db.exec`/`db.query`**；宿主持有每应用 SQLite，WAL、busy_timeout、5s、语句闸门、表/列上限 | 语言只看见 JSON 行；应用内 SQLite 会破坏“零 preopen”并引入 VFS 差异，不建议 |
| 网络 | 默认关闭；应用不能直接 socket；需要外部 HTTP 时由宿主白名单转发并审计 | Go 的 `net` 在 wasip1 是 fake；p1 无 socket；p2 的 `wasi:sockets` 也要显式 allow |
| HTTP/UI 资源 | 服务端只执行 wasm；UI 是 HTML/CSS/JS，默认放 release 的 assets manifest；若必须“编进 wasm”，用 `include_bytes!`/`go:embed` 打进模块再由 `asset.read` 导出；独立 origin + CSP | 语言选择只影响服务端逻辑，不会让 UI 自动变成 Go/Rust；浏览器 wasm 是另一个目标 |
| 长任务/超时 | 每请求新实例；端到端 60s、guest 30s、单 SQL 5s；epoch/fuel + 硬 kill | Go 冷启动 45ms、Rust 16ms；高频请求要配额/缓存/池化，语言影响成本 |
| 内存上限 | 按档位 64/128/256 MB，`StoreLimits`/`-W max-memory-size`；Go 预留更多 | Go 实测 +9 MB RSS；Rust/C 很小；TinyGo 居中；解释型语言更大 |
| 并发模型 | Engine/Module 共享，Store/Instance 每请求独立；tokio 中用 `spawn_blocking` 或 async；全局/每应用/每用户配额 | guest 内不要依赖 CPU 并行：Go 的 GOMAXPROCS=1；Rust 默认无 threads；p1 的 wasi-threads 实验 |
| 调试/可观测 | 生产捕获 stdout/stderr、wasm backtrace、duration、error_code；gdbstub 只用于开发 | Rust/C/Zig 调试最好；Go/TinyGo 较弱；解释型语言需自带日志 |

宿主边界建议（与语言无关）：

- 不授予 `--dir`、不继承 env、不开 socket、不暴露时钟/AI；只保留 `db.exec`、`db.query`、`log`、`asset.read` 四类 host function（必要时加 `time.now`）。
- 所有 host function 用扁平 `i32/i64`：guest 指针 + 长度 + 输出缓冲区；host 做内存边界检查、SQL 单语句闸门、响应大小上限、每次调用 fuel。
- 应用内 SQLite 不可行的硬证据：wasi-libc 的 `fcntl` 文件锁返回 `ENOTSUP`，SQLite 的 unix VFS 依赖锁/WAL 共享内存；要用就得自带 no-lock VFS，退化成单写、无 WAL，且需要 preopen（[wasi-libc commit](https://github.com/WebAssembly/wasi-libc/commit/2c5d9a57f6bd22bda363f131948743b4c5a94f55)、[SQLite 论坛](https://www.sqlite.org/forum/forumpost/6884c01399)）。

## 4. 只允许 Go + Rust 的损失

| 维度 | 损失 | 缓解 |
|---|---|---|
| 可写人群 | C#/Java/Kotlin、Python、JS/TS、Swift 的开发者不能直接写；企业客户常见 .NET/Java 团队被挡 | v1 用 HTTP/连接器接入；p2 成熟后按 WIT 加 C#/JS/Python |
| 易用性 | Go 明显比 Rust 好上手；Rust 学习曲线/编译时间/依赖审计是支持成本 | Go 做默认推荐，Rust 作为性能/安全选项；两套 SDK 共用 ABI |
| 体积/启动 | Go 2.4 MB、45 ms、+9 MB RSS；Rust 38 KB、16 ms | 模块编译缓存、实例配额、Go 应用限额；高频小应用优先 Rust |
| TinyGo | 不能替代完整 Go：无 wasm `recover`、reflect/std 缺口、GC 慢 | 只作为“预览/小工具”支持，不承诺完整 Go 兼容 |
| C/C++/Zig | 失去 4 KB 级、无 GC、可复用 C 库的选项 | vendor-only 或后续 Tier 2；默认不开放，避免内存安全/支持负担 |
| UI 预期 | 两种语言都不让 UI 变成“Go/Rust UI”；浏览器里仍是 HTML/JS | 明确 UI 规范；Kotlin/Compose、Rust/Leptos 的浏览器 wasm 是另一条目标链，不是服务端 p1 |
| Rust 编译链 | 技术上不重：`rustup target add wasm32-wasip1` 即可，自带 sysroot，产物 38 KB | 真正成本是语言与编译时间，不是工具链；CI 提供模板与缓存 |

## 5. 推荐与 ABI

| 层级 | 语言 | 承诺 | 理由 |
|---|---|---|---|
| 最小可行（M4 demo） | Go 1.27 wasip1 | 唯一 Tier 1 | 与参考实现同语言，最快打通生命周期/DB/UI |
| v1 正式 | **Go 1.27 wasip1 + Rust 1.99 wasip1** | 双 Tier 1，同一 ABI，同一 conformance | Go 覆盖可写人群，Rust 覆盖体积/启动/安全；两者 p1 都成熟 |
| 预览（不承诺） | TinyGo 0.44 wasip1；C/C++ wasi-sdk 34 | best-effort | TinyGo 适合小应用；C/C++ 只给有移植能力的 vendor |
| 暂不支持 | wasip2、unknown-unknown、Kotlin/.NET/Swift/Python/JS/AS/Scala | 明确拒绝 | p2 工具链/版本差异大；unknown-unknown 无 WASI；其余生态/体积/稳定性不足 |

ABI 设计（v1，p1 core module）：

1. **入口**：`_start`；从 stdin 读一帧，向 stdout 写一帧，然后退出。每请求新实例，不依赖全局状态。
2. **帧格式**：`RS` + 十进制字节长度 + `\n` + UTF-8 JSON，一次读满；请求/响应同构；限制请求 1 MiB、响应 8 MiB；多字节长度拒绝。
3. **请求 JSON**：`{"abi":"zeokeel-app/1","id":"...","method":"GET","path":"/","headers":{},"body_b64":""}`；响应含 `status`、`headers`、`body_b64`、`error_code`。ABI 版本由 guest 声明、host 拒绝未知版本。
4. **host function**：`db.exec(sql_ptr i32, sql_len i32, out_ptr i32, out_cap i32) -> i32`，`db.query(...)` 同理返回 JSON 行；`log(level i32, ptr i32, len i32)`；`asset.read(path_ptr, path_len, out_ptr, out_cap) -> i32`。返回值 `>=0` 为写入字节数，负数为错误码；全部显式长度、无 NUL、无跨语言 struct。
5. **语言无关性**：guest 只需要“stdin/stdout + 线性内存 + flat import”；Rust 用 `extern "C"`，Go/TinyGo 用 `//go:wasmimport`，C 用 `__attribute__((import_module))`，Zig 用 `extern "zk"`。JSON 是唯一跨语言编码，禁止 gob/bincode/Java serialization。
6. **SDK 与测试**：Rust crate `zk-app`、Go package `zkapp` 各自封装帧读写和 DB 类型；CI 跑同一组 golden frame + DB/asset/error/timeout 用例；两 SDK 的 import 名和错误码必须逐字节一致。
7. **p2 迁移**：把同一 frame 作为 `zeokeel:app/handler` 的 `list<u8>` 参数/返回值，WIT 只做类型化外壳；等 componentize-go、Rust wasip2、C#/JS/Python 的 p2 工具链稳定后再开，不改变业务帧。

## 6. 最小验证命令（/tmp，不写仓库）

以下命令均在 `/tmp/wasm-research` 下执行；海外下载加 `proxychains -q`。已实测通过。

如果环境缺工具链：Rust 只需 `rustup target add wasm32-wasip1`；Go 用官方 tarball；Wasmtime 用 release tarball 或安装脚本（锁版本）；DB host function **不能**只靠 wasmtime CLI，必须加 wasmtime-py/Rust embedding。完全离线且没有预编译产物时，本文的 DB 验证不可行。

### 6.1 工具链

```bash
export W=/tmp/wasm-research; mkdir -p "$W"; cd "$W"
# Wasmtime 50.0.1（当前最新；生产可换 48.0.8 LTS）
curl -sSL -o wasmtime.tar.xz \
  https://github.com/bytecodealliance/wasmtime/releases/download/v50.0.1/wasmtime-v50.0.1-x86_64-linux.tar.xz
tar xf wasmtime.tar.xz
export PATH="$W/wasmtime-v50.0.1-x86_64-linux:$PATH"
# Go 1.27.3
curl -sSL -o go.tgz https://go.dev/dl/go1.27.3.linux-amd64.tar.gz
mkdir -p go && tar -C go --strip-components=1 -xzf go.tgz
export PATH="$W/go/bin:$PATH"
# Rust wasm targets
rustup target add wasm32-wasip1 wasm32-wasip2
# DB host demo 需要 wasmtime-py 50.0.1
pip3 install --target "$W/pylib" wasmtime==50.0.1
```

### 6.2 Rust → wasip1 hello

```bash
cd "$W" && cargo new --bin rust-hello && cd rust-hello
cat >> Cargo.toml <<'EOF'
[profile.release]
opt-level = "s"
lto = true
strip = "symbols"
panic = "abort"
codegen-units = 1
EOF
cargo build --release --target wasm32-wasip1
wasmtime target/wasm32-wasip1/release/rust-hello.wasm
# 输出: Hello, world!
# 可选 p2: cargo build --release --target wasm32-wasip2 && wasmtime target/wasm32-wasip2/release/rust-hello.wasm
```

### 6.3 Go → wasip1 hello

```bash
cd "$W" && mkdir go-hello && cd go-hello && go mod init hello
cat > main.go <<'EOF'
package main
import "fmt"
func main() { fmt.Println("hello from go wasip1") }
EOF
GOOS=wasip1 GOARCH=wasm go build -trimpath -ldflags='-s -w' -o hello.wasm .
wasmtime hello.wasm
# 输出: hello from go wasip1
```

### 6.4 DB host function（Rust guest）

```bash
cd "$W" && cargo new --bin rust-db && cd rust-db
cat > src/main.rs <<'EOF'
#[link(wasm_import_module = "env")]
unsafe extern "C" { fn db_exec(ptr: *const u8, len: usize) -> i32; }
fn main() {
    let sql = "INSERT INTO notes(body) VALUES ('hello from rust guest')";
    let rc = unsafe { db_exec(sql.as_ptr(), sql.len()) };
    println!("rust guest: db_exec returned {rc}");
}
EOF
cargo build --release --target wasm32-wasip1
```

### 6.5 DB host function（Go guest）

```bash
cd "$W" && mkdir go-db && cd go-db && go mod init godb
cat > main.go <<'EOF'
package main
import ("fmt"; "unsafe")
//go:wasmimport env db_exec
func dbExec(ptr unsafe.Pointer, n uint32) int32
func main() {
    sql := "INSERT INTO notes(body) VALUES ('hello from go guest')"
    rc := dbExec(unsafe.Pointer(unsafe.StringData(sql)), uint32(len(sql)))
    fmt.Printf("go guest: db_exec returned %d\n", rc)
}
EOF
GOOS=wasip1 GOARCH=wasm go build -trimpath -ldflags='-s -w' -o go-db.wasm .
```

### 6.6 host.py（wasmtime embedding；CLI 不能定义自定义 import）

```python
import sys, sqlite3, wasmtime
con = sqlite3.connect(sys.argv[2])
con.execute("CREATE TABLE IF NOT EXISTS notes(id INTEGER PRIMARY KEY, body TEXT)")
def db_exec(caller, ptr, n):
    sql = bytes(caller.get("memory").read(caller, ptr, ptr + n)).decode()
    before = con.total_changes
    con.execute(sql); con.commit()
    return con.total_changes - before
engine = wasmtime.Engine(); store = wasmtime.Store(engine)
wasi = wasmtime.WasiConfig(); wasi.argv = ["app.wasm"]
wasi.inherit_stdout(); wasi.inherit_stderr(); store.set_wasi(wasi)
linker = wasmtime.Linker(engine); linker.define_wasi()
ty = wasmtime.FuncType([wasmtime.ValType.i32(), wasmtime.ValType.i32()],
                        [wasmtime.ValType.i32()])
linker.define_func("env", "db_exec", ty, db_exec, access_caller=True)
module = wasmtime.Module.from_file(engine, sys.argv[1])
inst = linker.instantiate(store, module)
try:
    inst.exports(store)["_start"](store)
except wasmtime.ExitTrap:
    pass  # Go 用 proc_exit(0) 正常退出
print("host rows =", con.execute("SELECT id, body FROM notes").fetchall())
```

```bash
cd "$W"
PYTHONPATH="$W/pylib" python3 host.py rust-db/target/wasm32-wasip1/release/rust-db.wasm app.db
PYTHONPATH="$W/pylib" python3 host.py go-db/go-db.wasm app.db
# 期望:
# rust guest: db_exec returned 1
# host rows = [(1, 'hello from rust guest')]
# go guest: db_exec returned 1
# host rows = [(1, 'hello from go guest')]
```

**为什么不是 `wasmtime` CLI 直接跑 DB guest**：CLI 会自动接 WASI，但“if your module imports anything else, it will fail instantiation”。实测 `wasmtime target/wasm32-wasip1/release/rust-db.wasm` 报 `unknown import: env::db_exec has not been defined`。因此自定义 host function 必须用 embedding（上面的 wasmtime-py 50.0.1；生产用 Rust `wasmtime` crate 或 C API 同理）。替代方案是写一个 ~40 行的 Rust host，用 `Linker::func_wrap("env","db_exec",...)`；但验证成本高于 Python binding。

### 6.7 其他已测

```bash
# 预打开目录：用一个 std::fs::read_to_string 的 p1 程序（rust-fs）对比
mkdir -p "$W/data"; echo hello > "$W/data/notes.txt"
wasmtime run --dir "$W/data::/data" rust-fs.wasm /data/notes.txt
wasmtime run rust-fs.wasm /data/notes.txt
# SIMD / memory64 / threads
wasmtime run --invoke add features/simd.wat          # 6
wasmtime run --invoke store_load features/memory64.wat  # 42
wasmtime run -W shared-memory=y --invoke inc features/threads.wat  # 0
# 资源限制
wasmtime run -W timeout=30s -W max-memory-size=134217728 app.wasm
```

## 7. 证据与未验证项

- Wasmtime：[v50.0.1 release](https://github.com/bytecodealliance/wasmtime/releases/tag/v50.0.1)、[proposal tiers](https://docs.wasmtime.dev/stability-wasm-proposals.html)、[CLI](https://docs.wasmtime.dev/cli-options.html)、[release process/LTS](https://docs.wasmtime.dev/stability-release.html)、[stability tiers](https://docs.wasmtime.dev/stability-tiers.html)。
- Rust：[wasip1](https://doc.rust-lang.org/rustc/platform-support/wasm32-wasip1.html)、[wasip2](https://doc.rust-lang.org/rustc/platform-support/wasm32-wasip2.html)、[wasip1-threads](https://doc.rust-lang.org/rustc/platform-support/wasm32-wasip1-threads.html)、[wasm64](https://doc.rust-lang.org/rustc/platform-support/wasm64-unknown-unknown.html)。
- Go：[1.21 wasip1](https://go.dev/doc/go1.21)、[1.24 wasmexport/reactor](https://go.dev/doc/go1.24)、[1.26 wasm memory](https://go.dev/doc/go1.26)、[1.27 SIMD](https://go.dev/doc/go1.27)、[net_fake.go](https://github.com/golang/go/blob/master/src/net/net_fake.go)。
- TinyGo：[WASI 指南](https://tinygo.org/docs/guides/webassembly/wasi/)、[语言支持](https://tinygo.org/docs/reference/lang-support/)。
- C/C++/组件：[wasi-sdk](https://github.com/WebAssembly/wasi-sdk)、[Component Model language support](https://component-model.bytecodealliance.org/language-support.html)。
- 其他：[Zig 0.17](https://ziglang.org/documentation/0.17.0/)、[Kotlin WASI](https://kotlinlang.org/docs/wasm-wasi.html)、[componentize-dotnet](https://github.com/bytecodealliance/componentize-dotnet)、[Swift Wasm](https://www.swift.org/documentation/articles/wasm-getting-started.html)、[CPython WASI](https://github.com/python/cpython/blob/main/Platforms/WASI/README.md)、[componentize-py](https://github.com/bytecodealliance/componentize-py)、[StarlingMonkey](https://github.com/bytecodealliance/StarlingMonkey)、[jco](https://github.com/bytecodealliance/jco)、[AssemblyScript status](https://www.assemblyscript.org/status.html)、[as-wasi](https://github.com/jedisct1/as-wasi)。
- 文件锁/DB：[wasi-libc fcntl ENOTSUP](https://github.com/WebAssembly/wasi-libc/commit/2c5d9a57f6bd22bda363f131948743b4c5a94f55)、[SQLite WASI 讨论](https://www.sqlite.org/forum/forumpost/6884c01399)。

未验证/边界：Kotlin/.NET/Swift/Python/JS 的 hello 大小、冷启动、实际 Wasmtime 兼容性均未在本机跑；AssemblyScript 未测；Scala 未找到官方 wasip1/p2 后端；Go 的 Delve 调试、Rust SIMD 实际代码生成、p2 在 48 LTS 上的 WASI 版本未验证。所有“未验证”不应写成承诺。
