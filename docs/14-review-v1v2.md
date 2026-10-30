# 14 · 对抗性评审与 v2 修订记录（历史）

> **v3 说明**：v3 已扩展为平台（服务端/双版本/OEM/交付）。**本文件是历史快照，编号对应 v1/v2，不作为现行规范依据；现行结论见 13/15；17 为历史存档。**本文件保留 v1→v2 的评审与修订过程，
> 作为「为什么现在这样设计」的证据；v3 新增决策见 `13-risks-and-adr.md` ADR-020…044，
> 当前最高优先级原则见 `00-design-principles.md`。

> 目的：让"规划被推翻过什么"可追溯。评审独立进行（不参与写作），产出
> [`research/review-plan.md`](../research/review-plan.md)（246 行，含可重跑实验）。
> 本文记录：发现了什么、改了什么、什么还没解决、判定怎么变。

## 1. 评审结论（原文摘要）

- 真正被验证的只有"改 profile → 换 path 依赖 → trait 身份不符则编译失败"，**不含** cordis 注册、
  角色基数、requires 拓扑、UI 注册表、锁文件、CI 矩阵；前者只占后者约三成工作量。
- 两个结构性缺口：**契约的发布形态**与 **profile × Cargo.lock 的互斥**。不解决，M0 第 4 条与整个
  矩阵都不成立。
- 7 项用户要求里：rig、参考 DSH 组织方式是真落地；cordis、前端包是半落地；"一切皆包 / CI 可替换"
  是口径 + 一个过窄的 POC。

## 2. 十条硬伤与处理

| # | 硬伤 | 处理 | 状态 |
| --- | --- | --- | --- |
| S1 | `register_all` 直接调 `register(ctx)`，绕过 cordis fiber，"整体回滚"不成立 | 改为"每个包产出 Plugin 适配器 → `ctx.plugin` → `await_settled` → 查 state"；删掉整体回滚，改为单 fiber 回滚 + `dispose_all`/degraded（02 §5.1、03 §3.1） | ✅ 已修 |
| S2 | 契约唯一性按"版本"去重，三种真实重复全漏 | 改为按 **package id（name+version+source）** 去重，**fail closed**，不得降级；负向用例补三种（01 §5、02 §6.4） | ✅ 已修 |
| S3 | 第三方包在契约未发布前不可实现；POC 的"仓库外"其实在仓库内 | 新增 ADR-016（发布 + `[patch]` 单实例）；S0-P1 spike；POC README 明确边界与锁/第三方实测（02 §6.5） | ✅ 已修（方案待 spike 验证） |
| S4 | profile 与 Cargo.lock 互斥，`--locked` 切 profile 必失败 | 新增 ADR-015：`zk.lock` 组合锁 + default 的 Cargo.lock；变体 `--offline` 重解析 + `zk verify-lock`；pnpm 用"候选包并集"（02 §5.5、05 §8） | ✅ 已修（实测支撑） |
| S5 | UI 贡献运行期声明却要求构建期失败 | 新增 ADR-017：贡献在 `zk.toml` 静态声明，运行期只过滤；删掉"运行期产生贡献"（01 §2、04 §3/§3.1） | ✅ 已修 |
| S6 | 组合输入三套（`[packages]`/`[overrides]`/`[web.roles]`），与"唯一真源"矛盾 | 收敛为唯一 `[packages]`，multi 用数组，删 `[overrides]`/`[web.roles]`/alias/overrides 通道（02 §3、04 §4） | ✅ 已修 |
| S7 | 壳无契约、`apps/desktop` 手写依赖 platform 包、serve 无角色 | 新增 ADR-018 + `zk-contract-platform`；`platform.shell`/`platform.http` 角色；`apps/desktop` 清单生成（01 §4、03、04 §6、05 §3） | ✅ 已修 |
| S8 | M1 约 31 包排 3 人周不现实；包数口径三套；命名不一致 | M1 砍到约 14 包/5–6 周；M2 再 +15（约 30）；M3 只要求 8 个角色各 ≥2 实现；统一包名（06） | ✅ 已修 |
| S9 | 验证预算算术不成立（T3 ≤60 min） | 改 T3 ≤90 min 分片 + 增量，明确"先实测再定"，S0 做基线（02 §6.2、06 §验证成本） | ✅ 已修（待实测） |
| S10 | rig-agent 被无收益钉死；坑表层错 | M1 只用 `rig-core`；`rig-agent`/`zk-agent-rig` 移 M5；坑表标 🤖 分层（03 §5、05 §4、07 R8） | ✅ 已修 |

## 3. 七个盲点与补充

| 盲点 | 补充 |
| --- | --- |
| 契约最小集与 `requires` 无类型保证 | M0 契约含 `Registrar/Clock/Tasks/Config`；`[contracts]` 登记 id↔crate，`zk validate` 双向校验（01 §5） |
| `zk` 与 cargo 的鸡生蛋 | 包索引读 `zk.toml`，metadata 只做交叉校验且 fail closed（02 §4） |
| 前端 workspace 与 Cargo 目录耦合 | 生成物移到 `apps/desktop/web/src/generated`，不进 pnpm workspace；依赖用并集（02 §5.5、04 §7） |
| 离线构建真实依赖面 | 补 Tauri bundler/Playwright/系统库需预烤镜像；`zk vendor` 并集（05 §8） |
| 跨平台留到 M4 | `.gitattributes` 强制 LF；M1 起 3-OS `cargo check`；修正"Playwright webkit 可替代 WKWebView"的错误说法（04 §8、05 §8） |
| UI props 无类型、流式协议缺 seq/重放 | 槽位 props 类型化，禁 `ComponentType<any>`；事件协议定义 seq/重放/重订阅/per-stream 取消；serve 加 token/Origin（03 §4.1、§6；04 §5） |
| 并发/取消/审批挂起未定义 | 03 §4.1 给出状态机与规则，进 M1 契约测试 |

## 4. v1 → v2 的口径修正

| 位置 | v1 | v2 |
| --- | --- | --- |
| "所有包可在 CI 替换" | 暗示全部角色任意替换 | **实现类包可替换**；契约按 major 演进；产物入口/引导代码不是包；只对 8 个 `swap_required` 角色做矩阵 |
| POC 的作用 | "机制已 POC 验证" | "验证了 trait 身份 + 依赖替换两件事"，其余列为未验证（ADR-019） |
| M0 退出条件 | 7 条，1 条不可达、2 条不可伪证 | 6 条可伪证；第三方改为 vendor + spike；CI 预算改为实测口径 |
| 第三方接入 | 默认支持 | 依赖 ADR-016；否则明确降级为 vendor-only |
| 排期 | M1 3 周 / M2 4 周 | S0 1 周 + M1 5–6 周 / M2 5 周 / M3 4 周 / M4 4 周 |

## 5. 仍未解决 / 需要实测的事

| # | 事项 | 结论在哪 |
| --- | --- | --- |
| 1 | 契约发布 + `[patch]` 单实例能否工作（S0-P1） | ADR-016 |
| 2 | UI 贡献静态化后 props 的灵活度是否够（S0-P2） | ADR-017 |
| 3 | 锁方案在真实工程（30+ 包）的时间成本（S0-P3） | ADR-015 |
| 4 | T0–T3 真实基线（决定 CI 门禁数字） | 06 §验证成本 |
| 5 | Tauri + Playwright + Xvfb 冒烟链路（1 天 spike） | M0 计划 |
| 6 | 是否承接先例用户数据（决定 M1 迁移器） | PLAN Q1 |
| 7 | 免重建换包是否硬需求（决定 ADR-001 存废） | PLAN Q2 |

## 6. 可重跑实验（评审留下）

```bash
cd experiments/composition-poc
python3 resolve.py default     && cargo run -q -p zk   # plain fs
python3 resolve.py hardened    && cargo run -q -p zk   # sandboxed fs
python3 resolve.py vendor-acme && cargo run -q -p zk   # acme fs（workspace 外）

# 锁文件：切 profile 后 --locked 必失败；--offline 成功
python3 resolve.py default  && cargo build -q --locked
python3 resolve.py hardened && cargo build -q --locked   # exit 101
python3 resolve.py hardened && cargo build -q --offline  # ok（重写 Cargo.lock）
```

评审还做了两个反向实验（同名契约副本 → `cargo metadata` 失败；改名副本 → `E0599`），
结论已写进 ADR-016 与 `01 §5`。
