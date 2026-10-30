# 2026-10-30-agent-dsh-parity · 提案

- 状态：`approved`
- 依据：用户指令“对照 dsh 把 agent 所有能力补齐，所有工具模块都要支持，提示词直接用 dsh 的”；DSH 本地安装 `/usr/local/lib/node_modules/@deepseek-ai/dsh`；`docs/05`、`docs/19`、`docs/21`、`docs/25`、C2/C3/C4/C9/C10/C16/C17
- 关联：M1b provider 运行时、审批、工具扩展；企业版 gateway 兼容

## 1. 背景

当前个人版 agent 只有 `TOOL fs.read` 一行字符串指令、单工具、无审批、无 JSON Schema、无多步循环。用户明确要求：**必须用 rig 框架把整个工作流串成一个完整 agent；DSH 只作参考；所有能力模块化；释放 rig 的全部能力**。

## 2. 目标

- 以 `rig-agent`/`rig-core` 为 agent 内核：AgentBuilder、DynamicTool/ToolSet、hooks、streaming、extractor、memory、vector store、loaders、多 provider；不自研第二套 loop。
- 移植 DSH 系统提示/工具提示词/工具描述（本地 bundle 可提取部分逐字使用，来源记录），作为 rig preamble/tool descriptions。
- 工具契约 JSON Schema；实现 DSH 全部 agent 工具模块（按 DSH 包清单），每个工具独立包/角色，统一错误/结果枚举。
- 审批：可配置危险工具需批准；UI 审批卡 + `approval.decide` 真实恢复。
- 个人版 provider 设置真正驱动运行时（M1b），支持 OpenAI 兼容 tools。
- 全部工具有单测/负向；agent 多步/审批/取消有 mock e2e；真实 UI 用 Playwright 人机操作 + axe。
- 独立子代理反证（工具矩阵/提示词/安全/测试假绿）。

## 3. 非目标

- 不引入 DSH Node 运行时/插件加载；只移植提示词与工具语义（C2）。
- rig 的图像/音频/转写等 provider 能力以模块/角色暴露，但只有配置了对应 provider 时启用；不伪造本地能力。
- 不承诺 MCP/远程插件市场；企业网关 Responses tools 只做兼容子集。
- 不做部门/强管控（C6）。

## 4. 成功指标

- DSH 工具包清单逐项在 ZeoKeel 有对应包/角色/测试，矩阵无缺口（可标明平台不适用项与原因）。
- `zk find`/capability 索引含全部工具角色；personal compose 选中；企业版不误选 personal-only。
- `cargo test`/clippy、agent mock e2e、Playwright 工具/审批/取消流程通过；独立评审 APPROVED。

## 5. 风险

- DSH bundle 是编译 JS，部分 prompt 由运行时配置/远端 profile 提供，本地不可得；需标注“未找到/替代”.
- 工具数量大，拆包/注册/测试成本高；按 P0/P1 矩阵推进，不得把未实现工具登记为可用。
- 平台工具（pwsh）在 Linux 只能逻辑测试/标记平台不支持；Worktree/子代理/工作流需本地状态存储。

## 6. 需用户拍板

- 是否接受“DSH prompt 本地可提取部分逐字移植；不可提取的 profile persona 用 DSH identity + 明确替代”？（默认接受并记录）
- 工具范围是否包含 experimental（agent-team/ralph/worktree）？默认包含注册与基础实现，深度能力按 DSH 语义。
