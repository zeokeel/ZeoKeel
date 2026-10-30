# composition-poc — "构建期换包"最小验证

这不是产品代码，是规划文档 `docs/03-composition-and-ci.md` 的可运行证据。
~200 行，三条命令证明同一个角色可以被三种实现替换，且业务代码零改动。

## 结构

```
Cargo.toml                  workspace: members = ["crates/*", "packages/*"]   ← glob，加包不用改根清单
packages/contract-tool/     契约缝：Tool / ToolPackage trait + ToolSpec
packages/tool-fs/           实现 A（明文 fs），[package.metadata.zk] role = "tool.fs"
packages/tool-fs-sandboxed/ 实现 B（沙箱 fs），同一个 role
profiles/default.toml       "tool.fs" = "zk-tool-fs"
profiles/hardened.toml      "tool.fs" = "zk-tool-fs-sandboxed"
profiles/vendor-acme.toml   "tool.fs" = { name = "acme-tool-fs", path = "external/acme-tool-fs" }
resolve.py                  最小 zk 解析器：profile + 包索引 → 生成 crates/zk-composed
crates/zk-composed/         生成物：Cargo.toml（只依赖被选中的包）+ lib.rs（角色别名 + 注册）
crates/zk/                  消费者：只调用 zk_composed::tools()
```

## 运行

```bash
python3 resolve.py default     && cargo run -q -p zk   # tool=fs impl=plain fs      call=Ok("plain-fs")
python3 resolve.py hardened    && cargo run -q -p zk   # tool=fs impl=sandboxed fs  call=Ok("sandboxed-fs")
python3 resolve.py vendor-acme && cargo run -q -p zk   # tool=fs impl=acme fs       call=Ok("acme-fs")
```

`external/acme-tool-fs` 是**仓库外形态**的第三方包：它不在 Cargo workspace 里（根 `Cargo.toml` 用
`exclude = ["external/*"]`），由 `profiles/vendor-acme.toml` 显式按路径引入。它的 `Cargo.toml` 通过
**相对路径**依赖同一份 `contract-tool`。若它自带一份私有契约副本（改名 `acme-contract-tool`），编译报错：

```
error[E0599]: no method named `tools` found for opaque type
              `impl acme_contract_tool::ToolPackage` in the current scope
```

这正是我们要的安全属性：**契约身份由类型系统保证，不靠 manifest 字符串自证。**

## POC 的边界（别把它当"机制已全部验证"）

**它证明了**：
1. 角色→包 的映射能被生成成粘合代码，业务代码零改动；
2. 依赖替换后 trait 身份不匹配会在编译期失败（fail closed）；
3. 同一个 workspace 里，切换 profile 后 `cargo build`（含 `--offline`）可以通过。

**它没有证明**（这些是 M0 真正的工作量）：
- cordis 集成（`ctx.plugin`/fiber/effect/生命周期）：POC 里根本没有 `Context`；
- 角色基数、`requires` 拓扑排序、可选依赖、平台条件；
- 契约唯一性校验（按 package id/source，不是版本）；
- 第三方包的契约发布/`[patch]` 方案（见下）；
- UI 注册表、锁文件与 profile 的关系、任何 CI。

## 关于锁文件的实测（重要，影响 CI 设计）

```
$ python3 resolve.py default  && cargo build --locked     # ok（首次）
$ python3 resolve.py hardened && cargo build --locked     # exit 101: cannot update the lock file
```

原因：生成的 `crates/zk-composed/Cargo.toml` 的**依赖边**随 profile 变化，而 Cargo.lock 记录了
每个包的依赖列表，所以锁文件必然变化。实测 `cargo build --offline` 在三 profile 间切换均成功
（所有源都在本地），但会重写 `Cargo.lock`。因此产品的策略是：
**`default` profile 用 `--locked`；变体在 CI 里 `--offline` 重解析 + 用 `zk.lock` 校验解析结果**，
而不是假装一份锁文件能覆盖所有 profile（详见 `docs/03` §6 与 `docs/10`）。

## 关于"仓库外第三方包"的实测

`external/acme-tool-fs` 位于仓库内、但**不属于 Cargo workspace**（根 `Cargo.toml` 有
`exclude = ["external/*"]`），它通过路径依赖复用同一份契约 crate。这模拟了"外部包"的形状，
但**不等于真正的仓库外**：真正的第三方包需要一个稳定的契约身份（crates.io 发布，或与你完全相同的
git URL+rev），再用根 `[patch]` 强制单实例。这一点在 M0 需要 2 天 spike 验证，规划里已列为前置项。

## 由 POC 推到产品的差距（规划里必须补齐的部分）

| POC 里是玩具 | 产品要求 |
| --- | --- |
| `resolve.py` 读 `[package.metadata.zk]` | `zk` CLI：`validate` / `compose` / `plan` / `explain` / `diff` / `--check`，`zk.lock` 锁定 |
| 一个角色一个包 | 角色基数（singleton/multi）、requires 图、可选依赖、平台条件 |
| 只有 Rust facet | 同一个包可带 `web/` facet（React 组件）、assets、config schema、i18n |
| 手写 `serde_json::Value` 输入 | 契约测试套件（conformance kit）对每个实现跑同一批断言 |
| 无 CI | 组合矩阵 + 换包矩阵 + 第三方 fixture + 负向测试 + SBOM |
