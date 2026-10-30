# DSH（DeepSeek Harness）组合模型与可替换性研究

> 研究对象：本机安装的 `@deepseek-ai/dsh@0.2.1-rc.2`
> （`/usr/local/lib/node_modules/@deepseek-ai/dsh/`，305 个 `@deepseek-ai/*` 包）。
> 方法：**只读 node_modules 内的 package.json / lib / dist / README 与 `~/.dsh` 实盘 profile**，
> 并用 `dsh --dump-config`、`--patch` 做过两个替换实验。凡未读到源码的，文中显式标「未验证」。
>
> 路径简写：`$DSH = /usr/local/lib/node_modules/@deepseek-ai/dsh`，
> `$PKG = $DSH/node_modules/@deepseek-ai`，`$HOME_DSH = /root/.dsh`（本机 `$DSH_HOME`）。

---

## 0. 结论先行（TL;DR）

1. **DSH 的"应用"是数据，不是代码。** 一个 profile 目录里 `cordis.yml` 永远是空 entry 列表
   （`[]`），真正的树由按顺序叠的 patch 层合成：`package.json` 的 `dsh.profile.bundles`
   → 每个 bundle 包自带的 patch 文件列表 → profile 自己的 `cordis.patch.yml`
   → `$DSH_HOME/cordis.patch.yml` → `--patch` overlay → telemetry 开关。
   每一层都是对同一份 YAML entry 列表做 **id 定向 patch / insert**。
   证据：`$DSH/lib/profile-boot-C14i-8IN.js:121,216,236`；`$PKG/dsh-app-boot/lib/index.js:551,3993,4125`。

2. **"一切皆包"不是修辞，是三层可替换边界：**
   - 包集合（bundle 列表）：18 个 `dsh.bundle.patch` 包，profile 换 bundle = 换整层能力；
   - 行（entry）：patch 按 id 改 `config` / `disabled` / `inject`，可 insert 新行；
   - 实现（seam）：13 个包以 `Abstract … seam (ctx.X)` 的形式只定义抽象服务，实现在独立包里；
     本次安装共 48 个包的 description 提及 seam
     （fs-local / fs-sandbox、bash-local / bash-sandbox、llm-deepseek / llm-pi-ai …）。
   替换的真实动作是 **disable 旧行 + insert 新行**，或换 bundle；**不是**运行时依赖注入换实现。

3. **patch 里 `name` 是断言，不能改。** 实测：对 `llm-deepseek` 行写
   `name: '@deepseek-ai/dsh-llm-pi-ai'` 会被跳过并告警
   `patch: name mismatch ... skipping`（`$PKG/cordis-plugin-include/src/index.ts:116`）。
   要换包必须 `- id: llm-deepseek / disabled: true` + `- insert: [{id: 新id, name: 新包}]`，
   这正是 DSH 官方实验 bundle 的写法（`$PKG/dsh-experimental-agent-team-profile/cordis.patch.yml`
   注释原文："these replacements keep direct delegation…"）。

4. **前端组件确实也是包，且不做静态 import。** 包在 `package.json` 声明
   `dsh.client: {platform, inject, external, immediately}` 并导出 `./client`（构建产物
   `lib/client.js`）。Host 半 `dsh-client-modules` 增量扫描 loader entry → 组成 boot graph
   → 以 `window.__DSH_BOOT__` 注入 HTML → 浏览器把 `ctx.loader.internal` 换成
   `ClientModuleSystem`，于是 cordis Loader 的每次 `EntryTree.import` 都走懒加载 CJS 表。
   证据：`$PKG/dsh-client-modules/lib/index.js:402,446,460,513,571`；
   `$PKG/dsh-client-modules/lib/client.js:1-30,61,777,985`；构建产物
   `$PKG/dsh-web-frontend/dist/assets/index-CN7NIWNw.js`（`__ModuleLoader__.create({boot:…})`、
   `loader.internal = modules`）。

5. **契约靠字符串服务名 + TypeScript 抽象类 + 生成式 schema，三者分开。**
   服务耦合是字符串（`super(ctx, "fs")`、`inject: ["fs"]`）；方法面靠 seam 包的 abstract class；
   配置校验靠 schemastery `Config`；跨 Host/浏览器 RPC 靠 Typert 从 TS 类型生成的
   InvocationDescriptor + Zod/JSON Schema。**运行时只校验 Config schema 和"服务名是否可用"，
   不校验接口方法面**——这层耦合是编译期的。

6. **版本策略是 lockstep**：294/305 个包同为 `0.2.1-rc.2`，778 条跨包 `peerDependencies`
   **全部是精确 `0.2.1-rc.2`**；启动时按 `engines.dsh`/peer 范围做兼容性 preflight，
   不兼容 bundle 被跳过并列入 `skippedBundles`，profile 可用 `compatibility.json` 做
   逐版本豁免。证据：`$PKG/*/package.json`、`$PKG/dsh-app-boot/README.md`（Profiles 节）。

7. **对 Rust 静态链接最不可移植的一点**：不是 YAML，而是
   **"配置里写包名字符串 → 运行时由 Node ESM resolver 解析并 import 那个包"** 这一层
   （外加它的孪生体：随包发布的 bundle patch 里 65 处 `!!js` 任意 JS 求值，能读 live ctx 服务）。
   Rust 里包名不可能在运行时变成可执行代码；必须把它降级为
   **构建期装配（profile → 生成粘合代码 → 单二进制）**，把"运行时可换"改写成
   "CI 可换 + 换后重建"。其余机制（seam/trait、provider 注册表、patch 层叠、槽位、
   契约生成、fail-loud 策略）都能在 Rust 侧找到强等价物。

---

## 1. 证据基线

### 1.1 包清单与分层（305 个 `@deepseek-ai/*`）

本机 `$DSH/node_modules/@deepseek-ai/` 下 305 个包（顶层只装 `dsh` 一个包，其余是其依赖）。
按包名前缀 + description 归类：

| 层 | 数量 | 代表包 / 职责 |
|---|---:|---|
| kernel / cordis | 7 | `cordis`(4.0.5)、`cordis-plugin-loader`、`cordis-plugin-include`、`cordis-plugin-group`、`cordis-plugin-timer`、`cosmokit`、`schemastery` |
| core / boot / seam / 其他 | 111 | `dsh-app-boot`、`dsh-base`、`dsh-fs`、`dsh-shell`、`dsh-llm`、`dsh-tools`、`dsh-session`、`dsh-typert-*`、`dsh-plugin-manager`、`dsh-hmr`、`dsh-web`、bundle 包 |
| client-ui | 54 | `dsh-client-ui-layout`、`-renderer`、`-conversation`、`-chat`、`-settings*`、`-sidebar*` … |
| session | 25 | `dsh-session`、`-persistence-jsonl`、`-projection*`、`-query-sqlite`、`-log-deepseek`、`-format-v0-to-v1`… |
| tool | 25 | 模型可见工具（bash/fs/web/subagent/todo/…），一个工具一个包 |
| experimental | 24 | `dsh-experimental-*-bundle`、`-agent-team`、`-inspector`、`-session-search`… |
| api | 10 | `dsh-api-gateway`、`dsh-api-session-controller`、`dsh-api-remote-access`…（Host↔Client 的 Remote 控制器） |
| host | 9 | `dsh-host-webserver`、`-frontend-static`、`-directory-picker-*`、`-plugin-inventory` |
| client | 8 | `dsh-client-modules`、`-connection`、`-hmr`、`-locale`、`-resources`、`-shortcuts` |
| agent | 7 | `dsh-agent`、`-loop`、`-preset`、`-preset-registry`、`-instructions`、`-default-model` |
| llm | 6 | `dsh-llm`（seam）、`dsh-llm-deepseek`、`-api-key`、`-account`、`-pi-ai`、`-retry` |
| shell / fs / sandbox | 4+4+4 | `dsh-shell`+`bash-local`+`bash-sandbox`+`pwsh-*`；`dsh-fs`+`fs-local`+`fs-sandbox`+`fs-observation-policy`；`dsh-sandbox`+`-local`+`-policy`+`wins-acl` |
| subagent | 3 | `dsh-subagent`(seam) + `-spawn-in-process` + `-fork-in-process` |
| 三方 / vendored | 4 | `libreoffice-kit(-wasm)`、`node-addon-system(-linux-x64)` |

`package.json` 里的 `dsh` 字段分四种用途（可当作"包类型声明"）：
`dsh.bundle.patch`（18 个 bundle）、`dsh.client`（78 个前端半包）、
`dsh.sessionFormatMigration`（4 个会话格式迁移包）、`dsh.profile.bundles`（profile 清单）。
证据：`$PKG/*/package.json`；统计脚本复现见 §9。

### 1.2 本机正在运行的实盘 profile（第一手配置样例）

`$DSH_HOME=/root/.dsh`，实际在跑的是 `web` profile：

```
/root/.dsh/profiles/web/
├── package.json        # dsh.profile.bundles 有序数组（9 个 bundle）
├── cordis.yml          # 空根："[]"（文件头注释说：树由 patch 组成，不要编辑）
├── cordis.patch.yml    # 用户层：1 条 id 定向 override
└── pnpm-workspace.yaml
```

`package.json` 的 bundle 列表（= 应用由哪些包组成，顺序即层序）：

```json
{ "dsh": { "profile": { "bundles": [
  "@deepseek-ai/dsh-base",
  "@deepseek-ai/dsh-web-app",
  "@deepseek-ai/dsh-experimental-agent-team-profile",
  "@deepseek-ai/dsh-experimental-auto-review",
  "@deepseek-ai/dsh-experimental-tool-worktree",
  "@deepseek-ai/dsh-experimental-session-search",
  "@deepseek-ai/dsh-experimental-terminal-bundle",
  "@deepseek-ai/dsh-experimental-session-titles-bundle",
  "@deepseek-ai/dsh-experimental-ralph-bundle"
] } } }
```

`cordis.patch.yml`（用户层）只有一行覆盖，示例价值很大——证明"改配置不动代码"：

```yaml
- id: ui-settings-general
  name: "@deepseek-ai/dsh-client-ui-settings-general"
  config:
    welcomeNoticeVersion: 2026-09-28.1
```

`cordis.yml` 的内容恒为：

```yaml
# dsh profile root — an empty entry list. The tree is composed as patches:
# each bundle in package.json's dsh.profile.bundles, then cordis.patch.yml, then any
# --patch overlays. Edit cordis.patch.yml, not this file.
[]
```

### 1.3 组装后的应用长什么样（实测 dump）

```
dsh --profile web --dump-config            # 含用户层
dsh --profile web --dump-default-config    # 只含 bundle 层（跳过 profile patch）
dsh --profile web --dump-config --patch X  # 叠加 invocation overlay
```

对 live profile 跑 `--dump-default-config` 得到 **1313 行 YAML**：
**200 个顶层 loader entry**（另有 117 条嵌套行位于 `cordis:group` 的 `config` 或
`dsh-agent-preset` 的 `config.plugins` 里，由 group/preset 在挂载时实例化）、
**194 个不同包名**、**33 个顶层行显式 disabled**、**42 处 `!!js` 表达式**。
dump 用 `# == 源层` 注释标出每段来自哪个 bundle / 被哪些层 patch 过，
是可加载的 YAML，也是"组合可审计"的关键设计。

样例（前 20 行，注意 `# == @deepseek-ai/dsh-base` 分段注释）：

```yaml
# == @deepseek-ai/dsh-base
- id: tool-plugin-manager
  name: '@deepseek-ai/dsh-plugin-manager/tools'
  disabled: true
- id: plugin-manager
  name: '@deepseek-ai/dsh-plugin-manager'
  disabled: !!js '!ctx.get(''profileContext'')'
- id: timer
  name: '@deepseek-ai/cordis-plugin-timer'
- id: hmr
  name: '@deepseek-ai/dsh-hmr'
  disabled: !!js '!ctx.get(''profileContext'')'
  config:
    root: []
- id: llm
  name: '@deepseek-ai/dsh-llm'
...
```

---

## 2. 组合机制：从"一堆包"到运行中的应用

### 2.1 五个可替换的组成要素

| 要素 | 位置 | 作用 |
|---|---|---|
| profile 目录 | `$DSH_HOME/profiles/<name>/` | 一份 `package.json`（`dsh.profile.bundles` 有序数组）+ `cordis.patch.yml` + `cordis.yml` |
| bundle 包 | npm 包，`dsh.bundle.patch: "./cordis.patch.yml"` 或文件数组 | 自带 patch 文件；装进 profile 的 bundle 列表即叠一层 |
| patch 层 | 每层是 YAML 数组：`{id, config, disabled, inject, insert, group…}` | 按 id 覆盖 / 插入 entry |
| loader entry 树 | cordis Loader + `cordis:include` | entry = `{id, name, config, disabled, inject, group}`；`name` 是待 import 的包名/文件 URL |
| 运行时解析 | app-boot 的 runtime resolution + `PluginPackages` 服务 | 让 `name` 从安装目录或 profile 目录按 Node 规则解析到真实包 |

### 2.2 boot 的实际调用链（读源码得出）

1. `dsh` CLI（`$DSH/lib/bin.js`）解析自己拥有的 flag（`--profile` / `--patch` / dump），
   剩余参数原样交给 app（`dsh-cmdline`）。启动走 `$DSH/lib/profile-boot-C14i-8IN.js:255 runProfile()`。
2. `prepareProfile()` 读 profile、`reportSkippedBundles()`、把 `cordis.yml` 重写成空根
   （`:216`；原因是 Loader 会把自 dispose 的树回写进文件，空根可避免每次 boot 重复 insert）。
3. `composeProfile()` 计算运行期包解析（`createRuntimeResolution`）并读 overlay（`:236`）。
4. `boot()`（`$PKG/dsh-app-boot/lib/index.js:4507`）：
   `new Context()` → `ctx.plugin(Loader)` → `prepare(ctx)`（provides `profileContext`、
   `dshHomePath`、`PluginPackages`、`cmdlineArgs`）→ `mountRootInclude()`
   （`:4125`；注册 `cordis:include` / `cordis:group` 两个 builtin，把 rootInclude 的
   `config.patches` 设成**压平后的 patch 列表**）→ `loader.await()` → `auditStartupEntries()`。
5. 每个 entry 由 Loader `import(name)`；`name` 是包名时走 app-boot 安装的 runtime resolution
   （`loader.internal.import(specifier, bareModuleBaseUrl)`）；bundle 包按"安装目录优先、其次 profile 目录"
   解析，entry 的普通包名由 runtime resolution 的映射决定（profile 自己 pnpm 装的包会成为 profile 作用域映射）。
6. 失败策略：required entry 失败 → 整个启动 dispose + 非零退出；optional 失败 → 告警继续；
   注入服务缺失的 entry **保持等待**，provider 出现后自动激活。required 列表硬编码在
   app-boot（`agent-loop`, `webserver`, `web-runtime`, `modules`, `connection`,
   `headless-runner`, `acp`, `sdk-jsonrpc-server`），来源 `$PKG/dsh-app-boot/README.md`。

### 2.3 装配的两种粒度

- **扁平的 entry 列表**：base bundle 就是"一个 insert"，96 个 entry（529 行 YAML），把所有核心能力挂上
  （`$PKG/dsh-base/cordis.patch.yml`）。
- **递归组合**：`dsh-agent-preset` 的 `config.plugins` 是**一份子 entry 列表**；
  `cordis:group` 的 `config` 是子行 + `isolate` 域。Web profile 在 host 层 disable 掉
  base 的 agent 平面行（tool-bash/tool-fs/plan-mode/…），改由每个会话按 preset 挂载。
  证据：`$PKG/dsh-web-app/cordis.patch.yml` 尾部 "the agent plane moves behind agent
  presets" 段落；`$PKG/dsh-web-app/presets/standard.patch.yml`；
  `$PKG/dsh-agent-preset/README.md`。这是"应用内还有应用"的关键：preset 是一等公民配置。

### 2.4 同一 launcher 产出多个产品形态

`PROFILE_TEMPLATES`（`$PKG/dsh-app-boot/lib/index.js:551`）：

```js
acp:  { bundles: ["dsh-base", "dsh-acp-app"] }
web:  { bundles: ["dsh-base", "dsh-web-app"] }
headless: { bundles: ["dsh-base", "dsh-headless"] }
sdk:  { bundles: ["dsh-base", "dsh-sdk-app"] }
"sdk-minimal": { bundles: ["dsh-sdk-minimal"] }   // 唯一不基于 base 的独立树
```

`dsh-web-app` 自己再声明 5 个 patch 文件（`package.json` 的 `dsh.bundle.patch` 数组）：
`cordis.patch.yml` + `presets/{standard,ptc,minimal,cordis}.patch.yml`。
所以"换产品形态"= 换 bundle 列表 / 换 bundle 的 patch 列表，代码零改动。

---

## 3. 「替换实现」的准确机制（本次最硬的结论）

### 3.1 替换发生在四层，不是"DI 容器"层

| 层 | 动作 | 证据 |
|---|---|---|
| A. 包集合 | 改 profile `dsh.profile.bundles`（增删/换序）；`dsh plugin add` 走 pnpm 装包后写回列表 | `$PKG/dsh-plugin-manager/README.md`（bundle toggle 段落）；`$PKG/dsh-app-boot/lib/index.js:611 initProfile` |
| B. bundle 内 patch 列表 | bundle 包自己的 `dsh.bundle.patch` 换文件/换内容（如 web preset） | `$PKG/dsh-web-app/package.json` |
| C. entry 行 | patch 按 id 改 `config` / `disabled` / `inject`；`insert` 新行；**`name` 是断言不可改** | `$PKG/cordis-plugin-include/src/index.ts:57-130`；实测见本节下文 |
| D. 服务/提供者 | 同一 seam 内多个实现按"注册名"并存或互斥，config 选具体 provider | `dsh-llm` 的 provider 路由名、`dsh-subagent` 的 `providerName`、`dsh-web` 的 `searchProvider/fetchProvider` |

**第 C 层的实测（2026-10-30，本机）：**

实验 1（换名，期望失败）：

```yaml
- id: llm-deepseek
  name: '@deepseek-ai/dsh-llm-pi-ai'
```

```
dsh --profile web --dump-config --patch ./swap-test.yml
# stderr: patch: name mismatch for "llm-deepseek"
#         (expected "@deepseek-ai/dsh-llm-deepseek-api-key", got "@deepseek-ai/dsh-llm-pi-ai"), skipping
# 结果：该行仍是 dsh-llm-deepseek-api-key
```

实验 2（disable + insert，期望成功）：

```yaml
- id: llm-deepseek
  disabled: true
- insert:
    - id: llm-mine
      name: '@deepseek-ai/dsh-llm-pi-ai'
```

```
# 结果（dump）：
# == @deepseek-ai/dsh-base, patched by /tmp/dshresearch/swap-test2.yml
- id: llm-deepseek
  name: '@deepseek-ai/dsh-llm-deepseek-api-key'
  disabled: true
# == /tmp/dshresearch/swap-test2.yml
- id: llm-mine
  name: '@deepseek-ai/dsh-llm-pi-ai'
```

**结论：DSH 的"换包"是组合层的替换（旧行关掉、新行插入），不是运行时按接口签名自动注入。**
官方实验 bundle 的注释直接把它叫 replacement：
`$PKG/dsh-experimental-agent-team-profile/cordis.patch.yml`
"Apply after dsh-base so these replacements keep direct delegation and coordination on the Team tools."

### 3.2 "seam + 多实现"清单（可替换性的物理载体）

本次安装 **48 个包的 description 提到 seam**：13 个自述为 `Abstract … seam (ctx.X)` 的定义包，
9 个自述为 `implementation of … seam` 的实现包，其余 26 个是围绕 seam 的 provider/tool/周边包。
一个能力被拆成两个包：**seam 包**（abstract service + `super(ctx, "<key>")` + 错误/词表）
和**实现包**。下表是已确认的服务 key / seam 包 / 实现包组合（seam 措辞有 `Abstract … seam`、
`… seam`、`… registry`，判断标准是"服务 key 由谁 `super(ctx, key)` 定义 + 谁提供实现"）：

| 服务 key | seam 包 | 已安装的实现包 |
|---|---|---|
| `fs` | `dsh-fs` | `dsh-fs-local`、`dsh-fs-sandbox`（后者 `class SandboxedFileSystem extends LocalFileSystem`，装饰而非平行实现） |
| `shell` | `dsh-shell` | `dsh-bash-local`、`dsh-bash-sandbox`、`dsh-pwsh-local`、`dsh-pwsh-sandbox`（sandbox 版 `extends LocalBashExecutor`） |
| `sandbox` | `dsh-sandbox` | `dsh-sandbox-local`（bwrap / Landlock / Windows ACL 由 `-local` 内部分派） |
| `llm` | `dsh-llm` | `dsh-llm-deepseek-api-key`、`dsh-llm-deepseek-account`、`dsh-llm-pi-ai`（路由名注册；重复路由报 `DUPLICATE_ADAPTER`） |
| `web` | `dsh-web` | `dsh-web-search-deepseek`、`dsh-web-deepseek-api`、`dsh-web-fetch-http`（按 provider id 注册，config 里 pin 或自动选唯一可用） |
| `subagents` | `dsh-subagent` | `dsh-subagent-spawn-in-process`、`-fork-in-process`（`ctx.subagents.registerProvider(name)`） |
| `sessionPersistence` | `dsh-session-persistence` | `dsh-session-persistence-jsonl` |
| `compaction` | `dsh-compaction` | `dsh-compaction-basic`、`-tool-result-pruner`、`-image-offload` |
| `credentials` / `settings` / `authorization` / `approval` / `userQuestions` | 各自 seam 包 | `dsh-credentials-local` 等 |
| `storage` / `spillStore` / `ptcRuntime` / `jobs` / `terminals` / `sessionQuery` | 各自 seam 包 | `dsh-storage-json`、`dsh-spill-local`、`dsh-jobs-local`、`dsh-terminal-bash`、`dsh-session-query-sqlite` |

证据：`$PKG/dsh-fs/lib/index.js:60`（`super(ctx, "fs")`）、
`$PKG/dsh-fs-sandbox/lib/index.js:103`、`$PKG/dsh-subagent-spawn-in-process/lib/index.js:11,36`、
`$PKG/dsh-web-app/cordis.patch.yml`（`bash-sandbox` / `pwsh-sandbox` 按平台 disabled 的成对行）。

**未验证/需修正的常见说法：** 任务背景里提到的 `dsh-llm-replay`、`dsh-llm-mock-server`
在本次安装中**不存在**（它们是 dsh 的 `devDependencies`，只在源码仓库/测试环境安装）。
因此"用 replay/mock-server 换掉真 provider"这一条无法在本机取证；能取证的是
llm-deepseek 与 llm-pi-ai 可在同一组合内并存、按路由名互不冲突（`$PKG/dsh-llm-deepseek/README.md`）。

### 3.3 替换的语义边界（重要）

- **同名服务通常只能有一个 provider**（cordis `Service` 以 ctx 上的 key 暴露；重复 provide 的
  具体异常文本未逐行验证，但组合里不会挂两个同名实现）。
  所以"替换"= 组合里只留一个实现，不是同时挂两个让调用方选。
- **多 provider 的场景是显式注册表**（llm 路由、subagent providerName、web provider id、
  storage backend 名），不是隐式 DI；重复注册同名 = 启动失败。
- **调用方只认服务名 + 方法面**：`inject: ["fs"]` 只校验 key 是否存在，方法面由 TS 抽象类约束；
  运行时没有结构校验。换一个方法签名不匹配的实现，启动不会报错，调用时才炸
  （`$PKG/dsh-fs-sandbox` 这类"继承 local 再覆写"的写法正是为了避免这一点）。

---

## 4. 前端 = 包

### 4.1 一个前端包长什么样（三个真实例子）

`dsh-client-ui-renderer`（基础设施包，`immediately: true`）：

```json
{ "main": "lib/index.js",
  "exports": { ".": "./lib/index.js", "./client": "./lib/client.js", "./locale/*.json": "…" },
  "dsh": { "client": { "platform": "web", "immediately": true } },
  "files": ["lib/index.js", "lib/client.js", "locale/*.json"] }
```

`dsh-client-ui-goal`（业务叶包，声明它依赖的包）：

```json
"dsh": { "client": { "platform": "web",
  "inject": ["@deepseek-ai/dsh-api-remotes", "@deepseek-ai/dsh-api-session-controller",
             "@deepseek-ai/dsh-client-locale", "@deepseek-ai/dsh-client-ui-chat",
             "@deepseek-ai/dsh-client-ui-conversation", "@deepseek-ai/dsh-client-ui-renderer",
             "@deepseek-ai/dsh-client-ui-session"] } }
```

`lib/client.js` 是构建产物，形状固定（不是 ESM）：

```js
window.__ModuleLoader__.load({
  id: "@deepseek-ai/dsh-client-ui-goal",
  factory: (require) => {
    let react = require("react");
    let primitives = require("@deepseek-ai/dsh-client-ui-primitives");
    const inject = ["slots", "sessions", "remote", "remote.goals", "locale", "uiConversation"];
    function apply(ctx) {
      ctx.slots.inject("conversation.user.text.node", () => ctx.slots.register({
        name: "conversation.user.text.node", key: "command", priority: 100
      }, GoalCommandReference));
      ctx.slots.inject("conversation.input.dock", () => ctx.slots.register({…}, GoalBar));
    }
    exports.apply = apply; exports.inject = inject;
  }
});
```

各字段的精确语义（`$PKG/dsh-client-modules/lib/client.js:61 parseDshClient`、
`lib/index.js:402 graphRow`）：

- `platform: "web"`：仅浏览器面；
- `inject: [包名…]`：**模块图顺序**，这些包的行要先注册/加载（不等于 cordis 服务注入；
  服务注入写在该插件自己导出的 `inject: ["slots",…]` 里）；
- `external: [包名…]`：允许 require 的非基线模块（共享模块表之外的），Host 侧校验供给者存在；
- `immediately`：boot 期间预取该 bundle，而不是等首次使用。

### 4.2 发现 → 注入 → 加载 → 渲染（完整链路，有源码/产物证据）

1. **Host 扫描**：`ClientModuleRegistry` 订阅 loader 的 `internal/plugin` 事件，按 entry 的
   `name` 找到所属包的 `package.json`，读 `dsh.client`；同时把 `lib/client.js` 快照、
   按 `mtime/ctime/size` 计算 rev，按目录分组为 combo URL
   `/plugins/??a,b&rev=…`（≤3KiB 分区）。
   `$PKG/dsh-client-modules/lib/index.js:513`（class）、`:571`（`webserver/index-inject`）、
   `:992 serveBundle`。boot 期间/热更时增量 flush，不做全量重扫。
2. **注入 HTML**：`bootInjections()`（`:460`）产生有序 head 行：
   ① `window.__ModuleLoader__` 队列 facade（纯内联 JS）
   ② application combo preload
   ③ parser-blocking bootstrap script（只有 `dsh-client-modules/client.js`）
   ④ `window.__DSH_BOOT__` 全局图（`{release, plugins, modules, batches}`）。
3. **浏览器起 loader**：构建产物 `$PKG/dsh-web-frontend/dist/assets/index-*.js` 里可见
   `window.__ModuleLoader__.create({boot, staticModules, ...seams})`，随后
   `ctx.loader.internal = modules`、`entries.start(loader, manifest)`、`loader.await()`。
   于是 **cordis Loader 的 `EntryTree.import`（唯一代码到达点）被替换成懒加载 CJS 表**；
   entry 治理（fiber、inject 等待、update）仍是同一份 loader。
   `$PKG/dsh-client-modules/lib/client.js:9-30` 的自述原文；`createClientModuleSystem` 在 `:985`。
4. **渲染**：`dsh-client-ui-renderer` 在浏览器提供 cordis 服务 `slots`，并把 React 挂载面暴露为
   `ctx.reflect.provide("uiRenderer", {mount})`（`$PKG/dsh-client-ui-renderer/lib/client.js`
   尾部 `apply`）；`ui-layout` 注册 `name: "root"` 并一次声明它的子槽
   （`sidebar`/`main`/`rightbar`/`shell.bottom`…，每个子槽带 `kind: single|keyed` 与 `scope`），
   业务包再往命名槽注册（`conversation.input.dock`、`conversation.chat.node`、
   `conversation.user.text.node`…）。没有静态 import 业务组件，shell 只 import 自己的
   布局/基础设施；槽位是前端侧的"契约面"，等价于 Rust 那边要生成的"槽位清单"。
5. **共享模块基线**：shell 冻结一张模块表。构建产物里 `function SZ(){return{…}}` 就是这张表，
   精确条目为：`react`、`react/jsx-runtime`、`react-dom`、`react-dom/client`、
   `@deepseek-ai/cordis`、`zod`、`@tanstack/react-virtual`、`@deepseek-ai/dsh-client-store`、
   `@deepseek-ai/dsh-client-ui-slots`、`@deepseek-ai/dsh-client-ui-primitives`、
   `@deepseek-ai/dsh-client-ui-dockkit`；动态 bundle 的 `external` 只允许解析到这张表或某个动态包行。
   这保证"React 单实例 + 包可独立构建"。证据：
   `$PKG/dsh-web-frontend/dist/assets/index-CN7NIWNw.js`（minified，`function SZ`)）。

### 4.3 `client-modules` / `client-connection` / `client-hmr` 的分工

| 包 | 角色 | 证据 |
|---|---|---|
| `dsh-client-modules` | 双面：Host 半 = `dsh.client` 扫描/图合成/`/plugins` 路由/index 注入/`ctx.clientModules`；浏览器半 = `ctx.modules`（懒加载 CJS 表，作为 loader 的 `internal` seam） | `$PKG/dsh-client-modules/README.md`、`lib/index.js`、`lib/client.js` |
| `dsh-client-connection` | 浏览器↔Host 传输：`ctx.connection`（fetch/SSE、重连、generation、trust fence），Host 半把 gateway 挂到 `/api`；不负责代码加载 | `$PKG/dsh-client-connection/README.md`；`$PKG/dsh-web-app/cordis.patch.yml` 的 `webserver`/`connection` 行 |
| `dsh-client-hmr` | 已打开页面的**图同步 + 组件代码热替换**：Host 侧推 graph，页面把变化行换成 `rev` 化 URL 重载，组件重挂载保留其他插件；开发时配 `pnpm run dev:web` 重建 bundle | `$PKG/dsh-client-hmr/README.md` |

补充：`dsh-client-connection` 的 `./client` 在 Node 下解析到 `lib/client.node.js`、
浏览器下到 loader factory；同一逻辑入口按环境分流，这是"前端包也能被 Node 组合"的手法。

---

## 5. 契约与稳定性

### 5.1 包之间的耦合面（四类，严格分开）

| 耦合 | 形式 | 运行时是否校验 |
|---|---|---|
| 服务 | 字符串 key（`fs`/`llm`/`tools`/`sessions`/`subagents`/`slots`/`remote`/`modules`…）+ `inject: ["key"]`；缺失则 fiber 等待 | 只校验"key 是否存在" |
| 接口方法面 | seam 包里的 abstract class（TS） | 不校验（编译期） |
| 配置 | 每个插件导出 schemastery `Config`（`import z from "@deepseek-ai/schemastery"`；如 `z.object({providerName: z.string().default("spawn")})`） | 校验，失败 → 行不激活（required 则启动失败） |
| 事件 | 字符串事件名 + TS module augmentation 声明（如 `internal/plugin`、`loader/config-update`、`webserver/index-inject`、`slots/changed`、`connection/reset`、`app-boot/config-reload`、`plugin-manager/changed`、`hmr/change`、`workflow/*`） | 不校验（字符串约定） |
| 跨端 RPC | Typert：由 TS 类型生成 InvocationDescriptor + Zod/JSON Schema，Host 端 `ctx.typertGateway`、浏览器端 `ctx.remote`；`@Remote` 方法标注 | 有：参数严格校验、descriptor 查找、schema 惰性物化 |

证据：`$PKG/dsh-typert-registry/README.md`（"generated package reflection, lazy Zod schema
factories, and Remote invocation descriptors"）；`$PKG/dsh-api-gateway/README.md`；
`$PKG/dsh-fs/lib/index.js`、`$PKG/dsh-subagent-spawn-in-process/lib/index.js`。

**一句话**：DSH 的包间契约是"字符串服务名 + TS 类型 + 生成式 schema"的混合物；
真正被运行时强制的是 Config 与 Typert 参数，服务接口本身只靠编译期。

### 5.2 版本策略

- **lockstep 发布**：305 个安装包中 294 个是 `0.2.1-rc.2`；其余是 vendored 三方
  （`cordis@4.0.5`、`cordis-plugin-*@1.x`、`schemastery@3.18.5`、`cosmokit@1.8.6`、
  `libreoffice-kit@0.1.5`、`node-addon-system@0.1.2`）。
- **精确 peer**：778 条跨包 `peerDependencies` 全部是精确版本 `0.2.1-rc.2`（不写 `^`）；
  消费方与提供方版本必须严格同批。
- **启动兼容性闸门**：bundle 加载前检查 `engines.dsh`（若有则只认它）或所有
  `dsh-*` peer 范围；不兼容 bundle 记入 `skippedBundles` 并告警，其余照常启动；
  `compatibility.json` 允许逐 `package@version` × `runtimeVersion` 精确豁免（不继承）。
  证据：`$PKG/dsh-app-boot/README.md`（Profiles 节 + Version compatibility and exemptions）。
- **能力分级**：stable（base 默认）/ 实验（`dsh-experimental-*-bundle`，默认关，用户自开）/
  optional bundles（shipped 但不在任何模板里，`OPTIONAL_BUNDLES` 列表，`dsh-app-boot/lib/index.js:579`）。
- **会话格式迁移**也走包：`dsh.sessionFormatMigration` 声明 from/to，4 个迁移包 +
  一个 `dsh-session-format-catalog`（**生成出来的静态目录**，注释写明"direct imports make
  historical readability independent of mounted plugins" —— 这是 DSH 里少见的"构建期生成 +
  运行期静态链接"先例，可被 Rust 直接借鉴）。

---

## 6. 启停与热更

- **Loader 是唯一生命周期权威**：entry 增删改 → fiber 激活/更新/dispose；
  `Entry.update` 对 raw options 做严格比较，只改 `config` 且 schema 标了 volatile 的字段
  走 `loader/volatile-update` 原地提交，不重挂；否则重挂 fiber。
  证据：`$PKG/cordis-plugin-loader/README.md`。
- **HMR 一个队列管两件事**：
  1. 配置重组合（profile `package.json` bundle 列表变化、profile/home patch 变化）：
     `readProfilePatches` → `recomposeProfile`（先移除旧行并等 dispose，再按新 generation 启动）；
  2. 模块代码替换：监听文件，比较 inode/size/mtime，evict 模块缓存，重启受影响 entry。
  证据：`$PKG/dsh-hmr/README.md`（Understand the implementation）。
- **HMR 强依赖 Node 私有 API**：模块替换需要 Node loader internals
  （`node-addon-require-builtin` / `internal/modules/esm/loader`），README 的
  Known Limitations 明确写了"Module replacement requires Node loader internals"。
  对 Rust 桌面应用不可移植；但**配置级重组合**（profile 变化 → 重挂 entry）这一半的语义
  可以用进程内"重建 registry / 重新装配"来复刻（代价是状态迁移要自己写）。
- **失败不静默**：启动失败 → 结构化诊断 + 非零退出；HMR 替换失败 → 记 `failed`，
  旧实现**不保留**，下次重组合再 import 当前文件（可能成功）。

---

## 7. 映射表：DSH 机制 → Rust 静态链接下的等价物

> 前提（团队已定，见 `docs/02-composition-and-ci.md` ADR-001）：**构建期组合 + 单二进制、
> 无动态加载**。下表据此逐条给等价物，并标注"不可行/需降级"。

| # | DSH 机制 | DSH 证据 | Rust 等价物 / 替代方案 | 判定 |
|---|---|---|---|---|
| 1 | 包 = npm 包，`name` 字符串在运行时 import | `cordis-plugin-loader` `EntryTree.import`；`loader.internal` | 包 = crate；静态链接。profile 里写**包 id 字符串**只作为构建输入 | **降级**：名字→crate 的解析从运行时移到生成器/CI；换包必须重建 |
| 2 | profile = `package.json` + `cordis.patch.yml` 有序 patch 层 | `$DSH/lib/profile-boot-*.js`；`$PKG/dsh-app-boot/lib/index.js` | `profiles/<name>.toml` 作为唯一输入；`zk` 生成 `composition.rs`（`Registry::install::<Impl>()` 顺序调用）。保留 `--dump-config` 等价物 + 快照测试 | **可复刻**（把 patch 语义移到代码生成） |
| 3 | entry 行 = `{id,name,config,disabled,inject,group}`，patch 按 id | 本报告 §3.1 实测 | 生成期概念：`Slot`/`Role` 的 config struct（serde 解析 `config.yaml` 片段）+ `bool` 开关；`name` 不可改的语义自然成立（实现类型在生成代码里） | **可复刻** |
| 4 | `!!js` 表达式（bundle patch 里 65 处 / 合成后的 web 树 42 处）在配置解析时求值，可读 `ctx`/`process.env`/`dshHomePath` | `$PKG/*/cordis.patch.yml` | 只保留数据插值：`${env:VAR}`、`${platform}`、`${home}/sessions`、`${cwd}`；复杂的少量行改成配置项。**不要**在 Rust 里嵌 JS 引擎（体积/沙箱/启动） | **不可移植**（有意放弃） |
| 5 | seam 包 = abstract service + 字符串 key；实现包独立 | `$PKG/dsh-fs`…见 §3.2 | `trait` 定义在 `contract` crate；impl crate 依赖 contract；`ServiceKey(&'static str)` + typemap（`TypeId`→`Arc<dyn Any>`）或直接 `Arc<dyn Fs>` 字段 | **可复刻，且更强**（编译期签名校验） |
| 6 | 同 key 单 provider；多 provider 走显式注册表 + 名字 | `dsh-llm` 路由、`subagent.registerProvider`、`web` provider id | `ProviderRegistry<T>`：`register(name, impl) -> Result`（重复名 = 启动错误）；singleton 角色由生成器保证只装一个 | **可复刻** |
| 7 | 递归组合：`dsh-agent-preset.config.plugins` + `cordis:group` isolate 域 | §2.3 | `Scope`/`SubRegistry`：会话创建时按 preset 配置文件构建子 `Registry`；`isolate` → 子 scope 的 key 解析规则 | **可复刻**（需设计 scope 生命周期） |
| 8 | 前端包：`dsh.client` + `./client` + slot 注册；Host 扫描 | §4 | 前端保留 TS/pnpm 包体系；`ui-shell` 用构建期生成的组件注册表（不 import 业务组件）；Rust 侧生成"槽位清单 + 契约 JSON" | **可复刻**（前端仍是 JS；Rust 只做 host） |
| 9 | 浏览器 loader 热替换：`loader.internal = modules` + combo bundle | §4.2 | Rust 无对应物；前端可保留 Vite/动态 import + 自己的模块表（或直接 Vite 产物多入口） | **前端可复刻**；Rust 侧无关 |
| 10 | 模块级 HMR（evict + 重启 entry） | §6；依赖 Node internals | 不做模块级热替换。替代：配置变更 → 进程内重启 host（状态持久化）或整进程快速重启；开发期用前端 HMR + `cargo watch` 重启 | **不可行/有意放弃** |
| 11 | 运行时装包（pnpm、profile node_modules、runtime resolution） | `dsh-plugin-manager`、`createRuntimeResolution` | 不在运行时安装。发布期决定组合；"装插件"= 下载新的 wasm 组件（M5 扩展点）或换二进制 | **不可移植** |
| 12 | 版本 lockstep + 精确 peer + `engines.dsh` + 豁免 | §5.2 | 单一 workspace 版本；`build.rs`/`xtask` 校验 profile 声明的版本与契约 major；CI 做矩阵编译 | **可复刻**（且 CI 更容易） |
| 13 | 启动失败矩阵：required 失败=停，optional=告警，缺服务=等待 | `$PKG/dsh-app-boot/README.md` | `Registry::start()` 返回结构化 `StartupReport`；`Required`/`Optional` 标记；缺依赖 → 启动错误（Rust 静态组合里"等待"没有意义，直接编译期/启动期报错更好） | **可复刻**（语义简化） |
| 14 | Typert：TS 类型 → Zod/JSON Schema + Remote descriptor | `$PKG/dsh-typert-registry` | `serde` + `schemars` 生成 JSON Schema；`ts-rs` 生成 TS 类型；RPC over WebSocket/JSON-RPC（或 tarpc/tonic）；契约 JSON 入库 + CI 漂移检查 | **可复刻**（前端已有同类方案见 `research/frontend.md`） |
| 15 | 会话格式迁移包 + 生成式 catalog | `dsh-session-format-catalog` | 版本化 serde enum + 每版本 `migration` 模块 + `build.rs` 生成静态 catalog（与 DSH 同构） | **可复刻，直接借鉴** |
| 16 | 包可被禁用/启用（`disabled`，行级开关） | `$PKG/dsh-base`、web patch 大量 `disabled: true` | 生成期 `enabled: bool`（profile 里可关）；契约测试覆盖"关掉后仍可运行" | **可复刻** |

**映射表的一句话总结**：DSH 里"可替换"由 **运行时按字符串装配** 实现；Rust 里同样的
可替换性由 **构建期代码生成 + trait + 显式注册表 + CI 矩阵** 实现。前者换包=改 YAML 重启，
后者换包=改 profile 重建；两者都把"包名"收敛到唯一输入（profile），区别只是解析时刻。

---

## 8. 对 Rust 最不可移植的一点（展开）

**不可移植的是机制，不是配置格式：**

```
DSH:  YAML 里的 name 字符串
        └─ Node ESM resolver（含 profile 目录 / pnpm / 私有 node_modules / runtime resolution）
             └─ 运行时 import 任意已安装包
                  └─ 包体可以在安装后替换（plugin-manager 安装/升级）
```

这条链让 DSH 能做到"改一行 YAML、重启进程，实现就换了"，甚至"在运行中的 Web 页面里
启用/禁用插件"。Rust 静态链接没有等价物：包名在编译后不指向任何可执行代码，
`dylib` 方案又与"单二进制、无 ABI 版本矩阵、无 unsafe"冲突（`docs/02` §1 已否决）。
**因此产品上必须把承诺从"运行时可换"改为"CI/构建时可换"**：
`profile → 生成器 → cargo build`，换包 = 改 profile + 重建 + 跑契约/换包矩阵。
第二不可移植的是同一机制的学生体：**`!!js` 让配置文件成为可执行程序**
（能读 live ctx、`process.env`、平台），Rust 侧只能保留受限插值/表达式子集。

反面：DSH 里真正的"胶水"（service 注册、entry 生命周期、故障策略、槽位渲染、版本闸门、
格式迁移 catalog）都能在 Rust 侧做得更严——因为它们本来就是编译期可验证的。

---

## 9. 复现命令（本报告用到的检查）

```bash
DSH=/usr/local/lib/node_modules/@deepseek-ai/dsh
# 包清单与版本
ls "$DSH/node_modules/@deepseek-ai" | wc -l
node -e 'for(const d of require("fs").readdirSync("'"$DSH"'/node_modules/@deepseek-ai")){try{console.log(require("'"$DSH"'/node_modules/@deepseek-ai/"+d+"/package.json").version)}catch{}}' | sort | uniq -c | sort -rn
# 实盘 profile
cat /root/.dsh/profiles/web/{package.json,cordis.yml,cordis.patch.yml}
# 组装结果
dsh --profile web --dump-config > /tmp/composed.yml
grep -c '^- id:' /tmp/composed.yml        # 200 顶层行
grep -c '!!js' /tmp/composed.yml          # 42
# 统计提及 seam 的包（13 个 Abstract 定义包 + 9 个实现包 + 26 个周边）
D="$DSH/node_modules/@deepseek-ai"
grep -l -i seam "$D"/*/package.json | wc -l   # 48 个包 description 提到 seam
# 替换实验（见 §3.1）
dsh --profile web --dump-config --patch ./swap-test.yml
```

---

## 10. 未验证 / 无法取证清单

1. `dsh-llm-replay`、`dsh-llm-mock-server` 在本机未安装（devDependencies），无法验证
   "测试双替身"用法；替代证据是 llm-deepseek 与 llm-pi-ai 并存。
2. `@deepseek-ai/dsh-package-manifest`（`dsh` 字段的 TS 类型定义）、
   `@deepseek-ai/dsh-client-web`（浏览器 shell 库源码）、`dsh-client-ui-slots`、
   `dsh-client-ui-primitives`、`dsh-client-store` 未随安装落地（或已被编译进
   `dsh-web-frontend/dist`）；前端壳的源码级交接只能从构建产物与 README 反推。
   结论中"shell 只 import 基础设施、不 import 业务组件"来自 dist 里的模块表与
   `dsh-client-modules/README.md`，**未读到 shell 源码**。
3. DSH 官方仓库的 CI 实践（是否有"包替换矩阵"）本机只有 GitHub URL，无 CI 配置，
   **未验证**；本报告能证明的是机制上"组合是数据、可被任意工具生成/替换"。
4. `plugin-manager` 的完整安装链路（pnpm 调用、registry 回退）未逐行读，只读了 README。
5. 前端 `dsh.client.inject` 在浏览器端是否还有第二重"服务注入"含义：从
   `lib/client.js:777 arriveGraphRow` 看只用于**模块到达顺序**；该字段是否也会被写进
   浏览器 loader entry 的 `inject`（服务名）未在源码中找到明确证据 → 按"模块顺序"表述。

---

## 11. 给架构决策的一句话

> DSH 用"**YAML 组合 + 运行时按包名 import**"换来"改配置即换实现"；
> ZeoKeel 若坚持静态链接，就必须把同一份组合数据变成**构建期输入**，
> 用代码生成 + trait + 显式注册表 + CI 换包矩阵，换来"换包即重建、但换错编译不过"的
> 更强保证。可替换性本身不丢——丢掉的只是"不重建就换"。
