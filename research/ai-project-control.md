# AI 写新项目：防失控方法论与工具调研

> 调研时间：2026-10-30。目标：**从源头阻断"AI 写歪 → 修修补补 → 上下文越来越乱"**，适用于 ZeoKeel
> 这种"客户端优先 + 一切皆包 + 构建期组合 + 多版本/OEM"的全新项目。
> 结论：单靠更长的 prompt 或更强的模型没用；需要 **规范先行（spec-first）+ 单一真源 + 可执行门禁 +
> 可复用 Skills + 评测集** 五件套。下面是对主流方法与工具的取舍。

## 1. 为什么会失控（根因）

| 根因 | 表现 | 对应解法 |
| --- | --- | --- |
| 需求只在聊天里 | AI 每轮重新猜，前后实现不一致 | **Spec-first**：先把意图写成可评审的 spec |
| 同一事实多份文档 | AI 检索到旧版本，按旧规范写 | **SSOT + docs-check**（本项目已做） |
| 隐式架构假设 | 包边界、契约、生成物被随手绕过 | **可执行架构规则**（dependency rules、生成物 owner） |
| 上下文窗口污染 | 长文档/长历史把关键约束挤掉 | **Agent Skills 渐进披露 + 任务上下文包** |
| 没有验收 | "AI 说完成了"就是完成 | **评测集 + 证据 + CI 门禁** |
| 任务过大 | 一次改 50 个文件，无法 review/回滚 | **小步变更 + 变更预算 + worktree/PR 隔离** |

## 2. 方法论对比

| 方法 | 核心 | 优点 | 代价 | 对 ZeoKeel 的适配 |
| --- | --- | --- | --- | --- |
| **GitHub Spec Kit** | constitution → specify → plan → tasks → implement → converge | 流程完整、模板/扩展/预置可组合；适合绿地项目 | 偏重、Markdown 多、需要 Python/uv；小改动仪式感强 | ★★★★ 适合"大里程碑"（M0/M1），不适合每个小 PR |
| **OpenSpec** | 每个变更一个文件夹：proposal/spec/design/tasks；archive | 轻、可迭代、支持跨仓 specs；适合 brownfield | 生态较新；需要自律 archive | ★★★★★ 最适合 ZeoKeel 的日常变更 |
| **BMAD-METHOD** | 角色化（PM/架构/UX/开发/QA）+ 工作流 + 上下文延续 | 多视角、适合从模糊想法到交付；有测试/架构扩展 | 流程重、角色多；小团队成本高 | ★★★ 适合产品定义期，不适合已定架构的迭代 |
| **Agent OS** | standards + product specs + tasks | 强调标准与任务拆解；轻量 | 社区/文档相对少 | ★★★ 可借鉴 standards/tasks 结构 |
| **Kiro Specs/Hooks** | spec-driven + 文件事件 hooks | IDE 集成好、hooks 可自动触发检查 | 绑定 Kiro IDE | ★★★ 可借鉴 hooks 思路，不绑定工具 |
| **AGENTS.md + SSOT + docs-check（本项目）** | 代理入口 + 单一真源 + 机械门禁 | 已有、与仓库同源、可 CI 化 | 需要持续维护 | ★★★★★ 作为底座 |

**建议**：不整体引入 BMAD/Spec Kit 的完整仪式；采用 **OpenSpec 式变更文件夹 + Spec Kit 的 constitution 思路 +
本项目现有 SSOT/docs-check**。大版本用 Spec Kit 模板，小变更用 OpenSpec 轻流程。

## 3. 工具链：把约束变成可执行门禁

| 层 | 工具 | 用途 |
| --- | --- | --- |
| 文档 | `scripts/docs-check.py`（已有） | H1/重复标题/链接/锚点/SSOT/废弃口径/ADR-D 编号 |
| 文档 | markdownlint、lychee、Vale（可选） | 风格、死链、术语一致性 |
| 组合 | `zk validate/compose --check/plan`（规划中） | 包集合、契约单实例、生成物漂移、闭包断言 |
| 契约 | ts-rs + schemars + `trybuild` | Rust→TS/JSON Schema 单源；编译失败用例 |
| Rust | clippy `-D warnings`、cargo-deny、cargo-machete、cargo-geiger | 严格 lint/依赖/许可证/unsafe 审计 |
| Rust | `proptest`、`cargo-mutants`、`insta` | 属性测试、变异测试、快照测试 |
| Rust | `cargo public-api`、`cargo-semver-checks` | 契约 API 破坏检测 |
| TS | typescript-eslint strict、`knip`、`dependency-cruiser` | 严格类型、死代码、**架构边界规则** |
| 测试 | Vitest、Playwright、axe-core | 单测、e2e、无障碍 |
| 安全 | Semgrep/CodeQL、secret scan | 注入/SSRF/密钥泄漏 |
| 性能 | size-limit、启动/首 token 基准 | 预算门禁 |
| 评测 | 自建 `evals/`：10–30 个代表性任务 + 期望 diff/测试 | 每次模型/提示变更跑回归 |

**重点**：`dependency-cruiser`（TS）和 `zk validate`（Rust/组合）是"架构 fitness function"，比文档写"禁止跨层依赖"有效得多。
[依赖规则示例](https://github.com/sverweij/dependency-cruiser)

## 4. Agent Skills：把"怎么做"变成可复用、可发现的能力

Anthropic 的 Agent Skills 是一个目录 + `SKILL.md`（YAML frontmatter：`name`/`description`），
采用**渐进披露**：启动只加载 name+description，命中任务才读正文，复杂内容再拆 `reference.md`/脚本。
[官方说明](https://www.anthropic.com/research/equipping-agents-for-the-real-world-with-agent-skills)

### 4.1 ZeoKeel 建议的 Skills 清单

```text
.agents/skills/
├─ zeokeel-change/SKILL.md            # 任何变更的入口：读 SSOT → 建变更文件夹 → 小步实现 → 证据
├─ zeokeel-add-package/SKILL.md       # 新增包：zk.toml/契约/conformance/README/edition 注册
├─ zeokeel-change-api/SKILL.md        # 改 API：路由 manifest/契约/客户端/契约测试/版本
├─ zeokeel-db-migration/SKILL.md      # 迁移：全局号/expand-contract/哈希/回滚
├─ zeokeel-client-ui/SKILL.md         # 页面/槽位/六态/a11y/i18n/审批卡
├─ zeokeel-release/SKILL.md           # 版本/渠道/更新清单/镜像/签名/演练
├─ zeokeel-oem/SKILL.md               # OEM intake/RACI/UAT/交接
└─ zeokeel-docs/SKILL.md              # 改文档：SSOT→改权威→删重复→docs-check
```

每个 SKILL.md 必须：
- 明确**触发条件**（description 里写"当用户要…时使用"）；
- **只读入口**（先读哪些文档，禁止先读整仓）；
- **步骤 + 命令**（`python3 scripts/docs-check.py`、`cargo nextest` 等）；
- **完成判据**（测试/证据/文档更新）；
- **禁止项**（不改生成物、不跨层依赖、不新增平台命令）。
- 复杂内容拆 `references/*.md`，脚本放 `scripts/`，避免 SKILL.md 膨胀。

### 4.2 Skills 的安全

Skills 可带脚本/资源，属于**供应链**：只从可信来源安装；本仓 Skills 必须 CODEOWNERS 评审；
禁止在 Skill 中写网络上传/密钥读取；安装前审脚本。Anthropic 原文也明确提示恶意 Skill 风险。

## 5. 推荐落地：五层防失控

### 第 1 层：规范先行（Spec-first）

每个变更一个目录（OpenSpec 风格）：

```text
specs/changes/<change-id>/
├─ proposal.md     # 为什么、范围、非目标
├─ spec.md         # 需求 + WHEN/THEN 场景 + 验收
├─ design.md       # 技术方案、取舍、影响的包/契约/数据
├─ tasks.md        # 可勾选任务，每项映射测试
└─ acceptance.md   # 证据路径/CI 命令/回滚
```

规则：**没有 spec 不开工**；spec 必须先评审；改 spec 与改代码同 PR；完成后 archive。
大版本可借用 [Spec Kit](https://github.github.com/spec-kit/) 的 constitution/specify/plan/tasks 模板；
日常用 [OpenSpec](https://github.com/Fission-AI/OpenSpec) 的轻流程。

### 第 2 层：SSOT + 上下文预算

- `docs/README.md` 是唯一索引；每篇头 3 行声明 `Owner / 定义 / 不定义`。
- 一个规范只写一次；摘要 ≤5 行 + 链接；`docs-check` 阻断重复/失效。
- 任务上下文包：给 agent 的入口固定为 `AGENTS.md → docs/README.md → 该任务权威文档 → 相关源码`，
  不把整仓塞进 prompt。
- 生成索引（包/契约/矩阵/schema）代替手写清单。

### 第 3 层：可执行架构

- `zk`：角色/契约单实例/闭包/生成物 `--check`；`dependency-cruiser`：TS 层边界；`trybuild`：契约误用编译失败。
- 生成物一路径一 writer；hooks 阻止手改 `@generated`。
- 迁移全局号 + 内容哈希 + expand/contract。

### 第 4 层：小步 + 隔离

- 一个 change = 一个分支/worktree/PR；**变更预算**：默认 ≤10 文件/≤400 行，超了先拆。
- 禁止 agent 一次做"重构 + 功能 + 依赖升级"混合 PR。
- 大重构先写 RFC/ADR，再分多个机械 PR。
- 每个 PR 必须：改了什么、为什么、验证命令、证据路径、影响的契约/数据。

### 第 5 层：评测 + 证据

- `evals/`：10–30 个代表性任务（加包、改 API、迁移、UI 六态、OEM 渠道、更新回滚），
  每次换模型/提示/Skills 跑一遍，比较通过率与 diff 大小。
- 禁止"模型自述完成"；完成 = CI 绿 + 证据文件 + 可复现命令。
- 监控指标：一次通过率、回归率、平均 PR 大小、文档重复率、逃逸缺陷数。

## 6. 不建议做的事

- 不要把所有规范塞进一个超长 `AGENTS.md`（上下文会被挤掉）；用索引 + 按需加载。
- 不要同时引入 BMAD + Spec Kit + Agent OS 三套流程；选一套轻流程 + 本项目 SSOT。
- 不要让 agent 自己决定架构/契约/数据格式；这些必须人工拍板并落 ADR。
- 不要用"更强模型"替代门禁；模型越强，越需要可执行边界。
- 不要让 Skills 成为新的重复文档源；Skills 只写"步骤/命令/判据"，规范仍指 SSOT。

## 7. 对 ZeoKeel 的最小落地清单（建议顺序）

1. **变更模板**：`specs/changes/<id>/` 五件套 + PR 模板。
2. **3 个 Skills**：`zeokeel-change`、`zeokeel-add-package`、`zeokeel-docs`（先覆盖最高频）。
3. **门禁**：`docs-check`（已有）+ `zk validate/compose --check` + `dependency-cruiser` + `trybuild`。
4. **评测集**：先 10 个任务（M0 骨架、加包、改 API、迁移、UI 六态、渠道、更新、OEM、钱包、文档变更）。
5. **上下文包**：`docs/README.md` 任务路径 + 每篇 Owner 头 + 生成索引。
6. 运行 2 周后复盘：一次通过率、PR 大小、重复文档、返工原因，再决定是否引入 Spec Kit/BMAD 完整流程。

> 结论：**Spec-first（OpenSpec 轻流程）+ SSOT/docs-check + Skills + 可执行架构门禁 + 评测集**，
> 是当前成本/收益最好的组合；Spec Kit/BMAD 只在"大版本定义期"借用，不整体套用。


## 8. 已落地（2026-10-30）

- `specs/README.md`：变更规格生命周期/五件套/与 docs 的边界。
- `specs/templates/`：proposal/spec/design/tasks/acceptance。
- `scripts/new-change.sh`：一键生成 change 目录。
- `scripts/spec-check.py`：模板/五件套/状态/任务 verify 门禁。
- `.agents/skills/`：`zeokeel-change`、`zeokeel-add-package`、`zeokeel-docs`。
- `AGENTS.md` C16：无 spec 不开工；`docs/README.md` 已加变更入口。
