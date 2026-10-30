# PLAN v1 对抗性评审

> **基线**：2026-10-30 11:32 的工作区快照。评审期间 `docs/` 正被并发修改（11:27–11:29 有 7 篇变更），
> 行号以该时刻为准；若行号漂移，按引文原文搜索。
> **方法**：8 篇规划逐份读，5 份研究简报抽查关键结论；在 `/tmp` 复跑 `experiments/composition-poc`
> 并补了 3 个反向实验（§6）。**结论**：机制方向成立，但被 POC 过度背书；M0 有 1 条退出条件不可达、
> 2 条不可伪证；M1 工作量约为排期的 2–3 倍；"一切皆包 / CI 可替换"目前是口径 + 过窄的验证。

## 0. 三句话总评

1. 真正被验证的只有"改 profile → 换 path 依赖 → trait 身份不符则编译失败"，
   **不含** cordis 注册、角色基数、requires 拓扑、UI 注册表、锁文件、CI 矩阵——而前者只占后者的三成工作量。
2. 最大的结构性缺口不是 `zk` 写不写得出来，而是**契约的发布形态**与**profile × Cargo.lock 的互斥**：
   这两件事没解，M0 第 4 条与整个 T3 矩阵都不成立。
3. 规划已意识到"口号风险"（01:87-96 已有边界表），但 `PLAN:21`、`PLAN:140`、`06:117` 仍是旧口径
   （"所有包 / 任意单例"）。**同一份文档里两套承诺并存**，评审时必须对齐到一套。

## 1. 硬伤（按严重度）

### S1 `register_all` 绕过了 cordis 的 fiber 模型，M0 的契约入口从根上错位

- 证据：`02:146-152` 生成的是 `pkg_tool_fs::package().register(ctx)?;`；而 `03:64-68` 要求
  `ctx.plugin(Arc::new(pkg), config)` 后 `await_settled()`/查 `state()`。同一份规划两个入口。
- 事实：cordis 的插件入口是 `Plugin::apply(&self, ctx: Context, config: Value)`，`name()` 返回
  `&'static str`（`research/kernel.md:79-84`）。`Package` 是平行 trait，`register(ctx)` 一旦被
  glue 直接调用，包申请的资源不归任何 fiber 所有：**无回滚、无卸载、无 `[config].schema` 注入**。
- 连带：`02:161` 的"任一包注册失败时整体回滚"不成立。内核明确"多 key 原子替换"做不到
  （`research/kernel.md:496`），只有单 fiber 激活失败的自回滚。整体失败必须由宿主 `dispose_all` 重建 root。
- 影响 M0：这是 M0 交付物 4/6 的交界，写错要重写契约 + 生成器 + 三个包。

### S2 契约唯一性检查按"版本"去重，实际必须按 package id/来源；三种真实重复它全漏

- 证据：`01:134-136`、`02:120-121`、`02:279` 都写"每个 `zk-contract-*` 只有一个版本"。
- 实测（§6）：外部包自带**改名**的契约副本 → `zk-contract-*` 里根本看不到它，检查通过，构建报 E0599；
  自带**同名**副本 → `cargo metadata` 直接报 `two packages named ... in this workspace`，`zk` 的
  "降级为清单级校验"（`02:120-121`）恰好跳过唯一性检查；**git 源 vs path 源**（同名同版本不同 source）
  则被 Cargo 视为两个 crate，版本检查通过、构建 E0599。
- 正确判据是 `cargo metadata` 里的 package id（name+version+source）集合，而不是版本号；且必须 fail closed，
  不能降级。

### S3 "第三方包可接入"在契约未发布前不可实现，而这正是 M0 第 4 条

- 证据：`02:286-288` 列了 vendor/git/crates.io 三种来源；`PLAN:180`（Q5）却说"先 git-only"。
  第三方包要依赖同一份契约 crate，只有三条路：契约发布到 crates.io；第三方用与你**完全相同**的 git URL+rev；
  或 vendor 进你的仓库。前两条当前都不成立，第三条就不是"仓库外"。
- POC 恰好回避了这点：`external/acme-tool-fs/Cargo.toml:7` 用 `path = "../../packages/contract-tool"`，
  只因为它在仓库里。`README:64` 还写"仓库外 `/tmp/acme-tool-fs`"——与事实不符（实际是 `experiments/composition-poc/external/`），
  这条"证据"会被外部评审直接打回。
- 同理 `06:47` 的"仓库外第三方包 `fixtures/external/...`"与 `02:286` 的 `vendor/`、`05:24` 的 `fixtures/`
  三处命名不一致，说明第三方路径根本没想清楚。

### S4 profile 与 Cargo.lock 互斥：`--locked` 下连续构建两个 profile 必失败

- 证据：`05:153` "Cargo.lock + zk.lock + pnpm-lock 三者入库；CI 全部 `--locked`"；`02:197-202` 要求
  非 default profile"在 CI 里先 compose 再 build"；`02:220` 又有 `cargo nextest --workspace --locked`。
- 实测（§6）：compose 切换 profile（增删外部依赖）后 `cargo build --locked` 直接 exit 101
  （`cannot update the lock file because --locked was passed`）。`02:223-224` 的 5 profile 矩阵必然踩到。
- 这不是"再跑一次 `zk lock`"能解决的：一个 workspace 只有一个 Cargo.lock，而 profile 决定依赖图。
  要么每 profile 独立 target-dir + 独立 lock（临时 workspace），要么维护"并集 lock"（见建议 2）。
- `pnpm-lock.yaml` 同理，且更糟：`ui/composed/package.json` 是生成物（`05:120`），`05:13` 又把 `ui/*`
  纳入 pnpm workspace → 生成物进了 lockfile 的 importer 列表，换 profile 即破坏 `--frozen-lockfile`。
  另外 `04:42-43` 的 glob 不含 `ui/*`，与 `05:13` 直接冲突。

### S5 UI 贡献的真源自相矛盾：运行期声明，却要求构建期失败

- 证据：`04:125` 明确"贡献实例 = Rust `register()` 里的 `UiContribution`，运行期随包启停"；
  但 `04:136`、`02:280` 又写"`UiContribution.component` 不在 web 侧 → **构建失败**"。
- 运行期才知道的值无法在构建期校验。`04:114` 生成的 `slots` 还要求含 `order / component id / props schema`，
  这三样全部来自运行期（`04:80-85`）。生成器只能生成"槽位定义 + 组件表"。
- 影响 M1：`ui-settings-general` 就要往 `ui.slot.settings.section` 贡献，第 1 天就会撞上；
  而它同时决定 `zk-contract-ui` 的形状（`UiContribution` 是否必须是 const），是 M0 契约决策，不能拖到 M1。
- 正确做法只有两条：把贡献静态声明进 `zk.toml`（可生成 const、可校验），或降级为运行期 + 显式 fallback
  并删掉"构建失败"的承诺。`04:127-130` 已经设计了后者，但没删前者。

### S6 组合输入有三套，profile"唯一真源"不成立

- 证据：`02:79` "`[packages]` 是角色→包 id 的唯一映射"；`02:72-74` 又有 `[overrides]`
  （"CI 的换包矩阵就生成这个"）；`04:153-157` 还有 `[web.roles]`（直接写 npm 包名 + `version = "1.4.2"`）。
- 三套输入互相冲突：`[web.roles]` 允许**单独**换 UI facet，违反 `01:159-166`/ADR-011 的"两个 facet 原子替换"；
  把版本写进 profile 违反 `02:81`"版本由 `zk.lock` 解析"。
- `04:155` 的 `"ui.slot.settings.section" = [...]` 与 `02:64-66` 的 `[[packages.multi]]` 是同一件事的两种写法；
  `04:144-148` 的 `pnpm.overrides` / `vite alias` 是第三、第四条换包通道，且都会绕过 lockfile 与生成物 diff 校验。

### S7 "一切皆包 / 任意单例可替换"被现有多处事实否证

- `01:78` 契约不替换、`01:83` product 不替换、`05:60-63` 有 4 个非包 crate（`zk-composed`/`zk-bootstrap`/
  `zk-testkit`/`tools/zk`）、`04` 还有 `ui/` 基建与 `scripts/`——但 `07:75-80` 的 ADR-009 仍叫"唯一例外"。
- 更要命的是 `platform.shell`：`02:55` 由 profile 选 `platform-tauri`，但 `05:65` 的
  `apps/desktop/Cargo.toml` **手写依赖"被选中的 platform 包"**，且 `zk-contract-*` 里没有 platform/shell 契约
  （`01:143-152`）。换壳 = 改 apps/desktop + 改 bootstrap，不是改 profile 一行。`PLAN:140`
  "任意单例角色都能被替换"在此失效，而这恰好是最重要的一个单例。
- `host-webserver`（`04:188`，M1 的 e2e 载体）在 `02:55-62` 的 `[packages]` 里没有角色可挂：
  `platform.*` 只定义了 `platform.shell`/`platform.ipc`（`01:112`）。**profile 无法表达"加一个 serve 载体"**，
  一个没有角色的包在"角色→包 id"的映射里不存在。

### S8 工作量与包数在 M1/M2 处对不上

- `06:60-73` 的 M1 交付物按包计：宿主 5 + 模型/契约 5 + agent 3 + 能力 5 + 存储 2 + 信任 2 + UI 9 + 壳 2 ≈ **31 个包 + Tauri + Playwright e2e**，
  排期 3 人周 = 15 人日 → 0.4 人日/包（还要写契约与 conformance）。这不可能。
- `06:92-94` M2 再加 30 个 → 总数 60+，而 `PLAN:66` 说 M3 才 45–60、M2 ≥30；**M2 结束就超了 M3 上限**。
- `06:139` "每个角色都有 ≥2 个实现被验证过" vs `PLAN:82-83` 只列 8 个多实现角色 vs `06:128-132` 又只列 8 组。
  三个口径。若按"每个角色"字面执行，40+ 角色 × 至少 2 实现 = 80+ 个实现包，与 70 包的目录上限矛盾。
- 名称也不一致（`tool-job` vs `tool-jobs`、`compact-tool-result-pruner` vs `compact-pruner`、
  `ui-i18n-zh/en` vs `ui-i18n`、`platform-transport-*` vs `transport-*`、`host-tauri-ipc` vs `platform-ipc`）。
  规划说"计数一律生成、禁止手写"，自己却手写了 5 处不一致。

### S9 验证预算算术不成立

- `02:250-251`：T2 = 5–10 min/profile，T3 = 所有角色 × 所有实现 ≤60 min。仅 5 个 profile 就是 25–50 min；
  再加 8 个角色的换包变体（至少 8–12 次构建），**T3 必然超 60 min**。`06:199-202` 同一组数字。
- `06:144` 的 "PR <15 min" 也紧：`02:220-221` 的 rust + frontend + contracts 三个 job 本身就要 10+ min 冷跑。
- 建议直接把预算改成"T3 = 增量构建 + 每变体只重编 2 个 crate"，并先做一次真实测量；现在的数字是愿望。

### S10 内核坑表放错层，rig-agent 被无收益地钉死

- `03:185-188` 的四个坑（`max_turns=1`、`Agent`/`AgentRunner`、工具批原子提交、`ToolExecutionCommitted` 非实时）
  全部属于 **rig-agent**；而 `06:63-64` 明确 M1 的 `agent-loop` 用自有状态机、**不用 rig-agent**。
- 但 `03:154/167-168`、`05:87`、`07:129` 仍钉 `rig-core + rig-agent + rig-memory = 0.45.0`。
  rig-agent 无条件依赖 rig-memory（`research/rig.md:58`），等于为一个"可选第二实现"引入 memory 与约 100+ 个包、
  以及版本漂移面。M1 只需要 `rig-core`。
- 连带：`03:157` 说工具包"由 `zk-agent-rig` 在装配期转成 DynamicTool"，与 M1 不走 rig-agent 冲突；
  `03:179` "rig 做能力提供方"也没有任何接口设计（`Handlers` 不进契约，那谁做 cordis→rig 的桥？）。

## 2. 是否回答了用户要求（逐条）

| # | 要求 | 判定 | 依据 / 差在哪 |
| --- | --- | --- | --- |
| 1 | Rust 桌面客户端 | 部分 | 选型（Tauri 2 + serve 副载体，`04:177-203`）有研究支撑；但壳无契约、`apps/desktop` 手写依赖具体包（S7），三平台到 M4 才验，M1 的"首屏<1.5s"只在 Linux 测 |
| 2 | 用 rig | **真落地** | 版本/钉法/坑表来自实测（`research/rig.md`），契约零 rig 依赖（`01:130-131`）是对的决定；但 M1 不用 rig-agent 却钉它（S10） |
| 3 | 用 zeokeel-cordis | 部分 | 研究到代码行号，H1–H14 映射最扎实；但入口绕开 `ctx.plugin`（S1），且 `Package` 让所有契约绑死 cordis 0.6 major |
| 4 | 参考 DSH 组织方式 | **真落地** | `02:301-312` 的对照有据；明确"借组织、不借运行时装配"是清醒取舍。小缺口：DSH"一个包可带 patch/默认 entry"被压成单链 base 继承，bundle 分层丢了 |
| 5 | 前端组件也是包 | 半落地 | 形态、静态注册表、baseline、闭包校验都设计了；但贡献真源冲突（S5）、`shell-kit` 是事实契约却建模成可替换 ui 包、渲染契约只有 `ComponentType<any>`（`04:93`） |
| 6 | 一切皆包 | 口径成立、事实不成立 | `01:87-96` 已把边界讲清（这是本版最大进步）；但 `PLAN:21/140` 仍是"所有包/任意单例"，且 serve 载体无角色（S7）、`tools/zk`/`zk-testkit`/`ui/` 都是例外而 ADR-009 说"唯一" |
| 7 | 所有包可在 CI 阶段替换 | 只落地最便宜一层 | T0 思路 + 编译期契约失败是真的；锁文件（S4）、第三方（S3）、矩阵预算（S9）三处不成立。POC 不含 cordis/UI/基数/requires/锁/CI |

**一句话**：7 项里 rig 与"参考 DSH 的组织方式"是真落地；cordis 与前端包半落地；
"一切皆包 / CI 可替换"是**口径 + 一个过窄的 POC**，其中最关键的第三方包在契约未发布前不可实现。

## 3. 最大盲点（规划未覆盖、M0/M1 就会卡）

1. **契约最小集的真实边界**：M0 只有 package/kernel/tool 三个契约（`06:33-34`），但 `03:54-55` 说每个包都
   `requires host.tasks / host.clock / host.config`。这些契约 M0 不存在，requires 只能靠字符串匹配，拓扑排序
   无类型保证。且 `01:43` 的 `zk.contract.fs-access@1` 在 `01:141-153` 的契约清单里没有对应 crate（应属
   `contract-host`），契约 id 与 crate 名不是一一对应。
2. **zk 与 cargo 的鸡生蛋只解了一半**：`02:126-128` 已承认"解析器不依赖编译"，这是对的；但 git/crates.io
   外部源的 `zk.toml` 仍需先 fetch 才能读到——这一步在"离线 + `--locked` + 首次接入"下无解，规划没写。
3. **前端 workspace 与 Cargo 目录耦合**：`packages/*/web` 既是 pnpm 成员又由 ts-rs 写入（`05:123`），
   非 UI 包也会长出一个 `web/` 目录；`ui/composed` 是生成物却带 `package.json`（`05:120`）并落在 pnpm glob 里。
   换 profile 会同时动 Cargo 图与 pnpm importer 列表（S4）。
4. **离线构建的真实依赖面**：`05:154` 只写了 `cargo vendor` + `pnpm fetch`。漏掉：Tauri bundler 在 Windows
   下载 NSIS/WiX、Linux 下载 linuxdeploy、Playwright 浏览器二进制、`libwebkit2gtk-4.1-dev` 等 apt 包。
   这些都不是 lockfile 能锁的，必须预烤镜像。`02:226` 又要求 CI 构建"仓库外路径 + git 源"，与离线直接冲突。
5. **跨平台从 M4 才开始**：`02:213-229` 的 CI 没有 OS 维度，`04:235` 却把"每平台 smoke job"当守卫。
   Linux 之外的问题（WebView2/WKWebView、路径、CRLF）会集中砸在 M4。`--check` 是字节比对，仓库没有
   `.gitattributes` 强制 LF，Windows checkout 会让生成物 diff 永远为红。
   另外 `04:235` 用 "Playwright webkit project" 缓解 WKWebView 差异是**无效的**：Linux 上它跑的是
   WebKitGTK，`research/frontend.md:529` 自己写明"只是近似，不能替代真机"。
6. **UI 槽位渲染契约 = 无契约**：`props` 是不透明 JSON（`04:84`），组件签名是 `ComponentType<any>`（`04:93`），
   没有 `ComponentProps` 类型、没有 props schema 校验、没有版本。流式事件只有 `{stream, seq, kind, payload}`
   （`03:218`），没有重连/重放/seq 断档语义，也没有"前端会话状态归谁"的设计（只有 `inject: ["session"]`
   字符串，`04:63`）。M1 做流式聊天必然要重设计这一层。
7. **IPC 单分发的类型损失只补了一半**：`03:222-225` 承认了损失，但命令 schema 仍手写在 `[[commands]]`
   （`03:215`）与 Rust handler 类型（`03:211`）两处；`03:224` 说签名统一为 `(Json)->Result<Json>`，
   `03:211` 却是闭包泛型 `|payload: ChatSendArgs|`。serve 载体在 M1 就上线（`06:72`），鉴权/CSRF/TLS
   留到 M5（`06:168`）。
8. **并发与取消没有定义**：一个会话能否并行多 turn、审批挂起时 turn 占什么、`dispose_all` 时 agent 任务
   如何与 `task-service` 的 cleanup 交互——`03:131-138` 只说了"per-consumer token"。M1 退出条件只测单轮，
   这些坑会推迟到 M2 才炸。

## 4. M0 可执行性（2 人周）

`06:42-50` 实际是 7 条（不是 6 条）。逐条判定：

| 退出条件 | 判定 | 说明 |
| --- | --- | --- |
| validate 空仓库可读错误 | 可达 | 1–2 天，前提是"空仓库"指没有 `zk-composed` |
| compose --check 绿 / 手改变红 | 紧但可达 | 需先解决确定性生成 + 与 cargo metadata 解耦；3–5 天 |
| default vs mock 各构建成功且输出不同 | 可达 | 1 天，但 CI 里必须解决 S4（锁） |
| **仓库外第三方包可构建** | **不可达** | 契约未发布（S3）；POC 的"外部"其实在仓库内。这是 M0 最大的隐含前置 |
| 负向 5 条全红 | 4 条可达 | "私有契约副本"那条实际以 `cargo metadata` 的 workspace 重名错误出现（不可读），且最关键的"契约两个版本"不在 5 条里（`02:279` 有、`06:48` 没有） |
| CI <10 min（冷构建除外） | **不可伪证** | "冷构建除外"使条件恒真；应改为"热缓存下 T0+T1 < 10 min"并给出缓存定义 |
| 新人 30 分钟换包并看到变化 | **不可伪证** | 起点未定义（是否含装 1.97.1 工具链 + pnpm + 编译 zk）；M0 没有预编译 `zk`，README 也没写安装步骤 |

**结论**：7 条里 1 条不可达、2 条不可伪证、3 条紧。若坚持 2 人周，必须：砍掉 `zk plan/explain` 与
`zk.lock v0`（移到 M1）；第三方包改为"2 天 spike 验证契约发布方案"，不计入退出条件；把"30 分钟上手"
改成"clone → `scripts/bootstrap.sh` → `cargo run` 一次成功"，并提供预编译 `zk` 或 `cargo xtask`。

**应前移**（决定 M0 契约形状，不能留到 M1）：① UI 贡献是静态声明还是运行期（S5）——它决定
`zk-contract-ui` 的 `UiContribution` 形态；② 契约发布/patch 方案（S3）——它决定第三方能否接入；
③ 锁文件方案（S4）——2 小时实验，失败则整个矩阵设计要改；④ Tauri+Playwright+Xvfb 冒烟（1 天），
确认 e2e 路线可行。**应砍**：`04:144-148` 的 layer 2/3（overrides/alias，绕过唯一真源）、`02:324-340`
的 `[alternates]`（M3 之后再说，现在只会分散"编译期可替换"的叙事）、`oem-demo`（Q4 未确认前不排期）。

## 5. 修改建议（Top 10，按优先级）

1. **先解决契约身份**：把 `zk-contract-*` 发布到 crates.io（或用固定 git URL+rev），并在 workspace 根**生成**
   `[patch]` 段把任何外部包的契约依赖重定向到同一实例；否则 M0 第 4 条与 M3 的 git 源接入都不可实现。
2. **给每个 profile 独立构建上下文**：临时 workspace + 独立 `--target-dir` + 独立 lock，或引入
   `crates/zk-lock-union`（依赖全部候选包、永不构建）稳定 Cargo.lock；同时把 `--locked` 限定在 default 单 job，
   并把 `ui/composed` 移出 pnpm workspace glob。
3. **`Package` 不持有 cordis Context**：改为 `install(&mut dyn Registrar)`，`Registrar` 定义在
   `zk-contract-kernel`、宿主实现为 `ctx.plugin` 包装；生成的 `register_all` 为每包产出一个 Plugin 适配器 +
   逐包 `await_settled` + 失败 `dispose_all` 重建 root，删掉"整体回滚"的表述。
4. **UI 贡献静态化**：在 `zk.toml` 加 `[[ui.contributions]]` 并生成 const，运行期只做"启用/禁用"；
   `registry.gen.ts` 只生成组件表与槽位定义，不生成 order/props——这样 `04:136` 的构建期闭包校验才成立。
5. **组合输入收敛为一套**：删 `[overrides]` 与 `[web.roles]`，UI 角色并入 `[packages]`（facet 原子替换）；
   `brand` 只写包 id，版本一律进 `zk.lock`；同时删 `04:144-148` 的 overrides/alias 两条换包通道。
6. **给 `platform.shell` 与 serve 载体定契约**（`ShellHost` + `platform.http`），并让 `apps/desktop/Cargo.toml`
   由 `zk compose` 生成；否则"任意单例可替换"在壳与 M1 的 e2e 载体上都不成立。
7. **重排 M1/M2**：M1 砍到 12 个包（宿主 4 + llm-rig/deepseek/mock + agent-loop + tool-fs/shell + store-session
   + ui-shell/chat + transport-*），排期改 5–6 人周；M2 的"30 个包"降到 15；M3 退出条件改为
   "`PLAN:82-83` 的 8 个角色各 ≥2 实现"，与包目录上限对齐。
8. **契约最小集按宿主服务面重切**：M0 只要 `contract-package`（install/descriptor）+ `contract-kernel`
   （Registrar/Clock/Tasks/Config 四个 trait）+ `contract-tool`；把 `01:43` 的 `zk.contract.fs-access` 改成
   `zk.contract.host`，并加一条校验"profile 里出现的每个契约 id 都必须有同名 crate"。
9. **离线/跨平台前置**：加 `.gitattributes`（`* text=auto eol=lf`）；`zk vendor` 生成"全 profile 并集"的
   vendor 目录 + `[source] replace-with`；CI 镜像预装 webkit2gtk/Playwright/tauri bundler；M1 起加 3-OS 的
   `cargo check` job，别等 M4。
10. **IPC 与流式先定协议再写 UI**：命令 schema 从 Rust handler 类型用 schemars 生成（单源），由生成的
    typed router 分发；定义 `ipc://event` 的 seq/重放/重订阅与 per-stream 取消；serve 载体在 M1 就加
    token 鉴权与绑定 `127.0.0.1`。

## 6. 本次复现与反向实验（可重跑）

| 实验 | 命令 | 结果 |
| --- | --- | --- |
| POC 三 profile | `python3 resolve.py {default,hardened,vendor-acme} && cargo run -q -p zk` | 三种输出均正确（plain/sandboxed/acme） |
| 私有契约副本（改名） | 复制 `contract-tool` 为 `acme-contract-tool` 并让 acme 依赖它 | `error[E0599]`，与文档描述一致，但提示信息误导为"import trait" |
| 同名契约副本 | 复制为第二份 `zk-contract-tool`（不同 path） | `cargo metadata` 直接失败：`two packages named zk-contract-tool in this workspace`；`resolve.py` 的降级扫描（`:25-33`）看不到它 |
| profile 切换 + --locked | 先 compose 一个 profile 建 lock，再换 profile 后 `cargo build --locked` | **exit 101**，`cannot update the lock file because --locked was passed` |

POC 的边界（必须写进 README 的"已验证"清单）：无 cordis、无 `ctx`、无异步/取消/审批、无角色基数、
无 requires 图、无 UI、无锁、无 CI；契约 `Tool::call(args) -> Result<String, String>`
（`experiments/composition-poc/packages/contract-tool/src/lib.rs:15-23`）比产品契约薄一个数量级。
建议把 `README:59-68` 的"证明了三件事"改成"证明了 trait 身份 + 依赖替换两件事"，并把
"外部包"改为"仓库内 `exclude` 的 path 依赖"。

## 7. 评审未覆盖 / 需要产品确认

- `PLAN:173-181` Q1–Q6 未答：数据承接、免重建、serve、OEM、发布形态、rig 升级节奏——其中 Q2/Q5 直接
  决定 ADR-001 与 S3 的走向，建议在评审会上先拍这两个。
- 未评估：许可证兼容策略（`zk.toml` 有 `license` 字段但没有策略）、遥测/隐私（`telemetry` 包无 opt-in 设计）、
  密钥库跨平台依赖（`host-secrets` 选型未定）、i18n 的包粒度（`01:115` 把 `i18n.*` 定为 singleton，
  但 `06:96` 要 `ui-i18n-zh`/`ui-i18n-en` 两个包同时存在）。
- 本次只做了 2 个反向实验，未验证：`cargo vendor` 的并集可行性、Tauri 2.12 + Xvfb、Playwright webkit 在
  Ubuntu 的实际覆盖率、`--check` 在 Windows 换行下的行为。这些建议在 M0 第 1 周各花半天实测。
