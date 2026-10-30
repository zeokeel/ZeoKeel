# 11 · 测试分层与证据政策

> 只有**可执行、可复现、外部化**的证据才算"完成"；模型自述不算。
> 本文件定义平台（客户端 + 服务端 + 交付物）的测试层级、矩阵与门禁。

## 1. 分层

| 层 | 范围 | 成本目标 | 何时跑 |
| --- | --- | --- | --- |
| T0 组合快照 | 清单校验、角色/契约、路由/权限矩阵、UI 闭包、迁移哈希 | < 10s | 每次提交 |
| T1 单包 | `cargo check -p` + 该包单测 + conformance | < 3 min | PR 受影响包 |
| T2 组合构建 | edition/profile 构建（client/server/admin）+ 启动自检 | 5–15 min/profile | PR（enterprise + personal） |
| T3 集成 | 真 PG + 服务端全包；真客户端 + 真服务端 | 10–20 min | PR / 合并前 |
| T4 矩阵 | `swap_required` 角色（清单由 `zk validate` 生成）× 替代实现；OEM 渠道；WASM 应用样例 | < 90 min（分片） | nightly |
| T5 交付物 | 三平台安装包 + docker 镜像 + 门户 + 升级/回滚 | 30–60 min | nightly / 发布前 |

预算在 **M0 骨架**（真实 workspace）标定一次，之后作为门禁；S0 只做机制 spike，不测 T0–T5 基线；超预算先优化再接受。

## 2. 客户端测试

- 单包：逻辑单测 + 组件测试（Vitest）。
- 契约：`zk-contract-*` 的 conformance 套件对每个实现跑同一批断言（工具、provider、存储、UI 贡献）。
- IPC：从 `http-routes.json`/`permissions.json`/`api-catalog` 生成"全量命令正向 + 缺参 + 越权"用例。
- e2e：Playwright 跑 `serve` 载体（HTTP+WS，免 WebDriver），按 edition/profile 分 project。
- 真应用：Xvfb/openbox 下启动 Tauri 二进制冒烟（窗口创建、登录、一轮对话）。
- 前端：组件 id 闭包、React 单实例、bundle 体积、CSP、i18n key 覆盖。

## 3. 服务端测试

| 类型 | 内容 |
| --- | --- |
| 单测 | 仓储/服务/权限/错误路径；每个包独立 |
| 迁移 | 空库全量迁移 → schema 快照比对；旧版本库升级；**改历史迁移必须红** |
| 路由契约 | 生成的全量端点：401/403/参数校验/请求体上限/幂等 |
| 集成 | 临时 PG + 全包组合：登录、刷新、网关转发（假上游）、计费、技能审批、连接器下发、应用执行 |
| 安全 | 认证绕过、CSRF、SSRF、归档解压路径穿越、WASM 逃逸/资源耗尽、越权 |
| 钱包 | 并发扣费、超额阻断、月发放幂等、**累加/覆盖两种模式**、峰谷价结算、流式透支欠费、余额不足回滚、结算与 usage 一致性 |
| 网关 | 流式透传、工具调用、上游故障转移、未定价模型拒绝、限流 |
| WASM | 帧 ABI、每应用 DB 隔离、限额、超时、上架下架生命周期 |

## 4. 交付物测试

- 镜像：`docker load` → compose up → `/readyz` → 门户可访问 → 客户端安装包校验和。
- 升级：旧镜像 + 旧数据 → 新镜像 → 迁移 → 业务可用；回滚到旧镜像 + 备份。
- OEM：每个渠道产物跑品牌断言（按 `brand.powered_by`：`hide` 断言零官方品牌；`show` 只允许固定 “Powered by ZeoKeel”）、深链 scheme、应用 origin、数据目录、包名。
- 安装包：三平台安装/卸载/自动更新；**Windows 仅校验和、macOS 校验签名/公证**。
- 离线：无外网环境部署与运行（除客户端 AI 请求）。

## 5. 证据与报告

- 每次 CI 产出 `target/evidence/<run>/`：测试报告、端点矩阵、覆盖报告、SBOM、产物哈希。
- "完成"判据：对应层的命令在 CI 绿 + 证据文件存在 + 可复现（README 记录命令）。
- 手工验证也必须落成脚本（否则下次无法复现）。
- 文档里的数字（包数/端点/测试数）一律生成，不手写。

## 6. 关键门禁（必须失败才算通过）

| 门禁 | 失败条件 |
| --- | --- |
| `zk validate` | 缺角色/重复 singleton/契约两个 package id/迁移哈希不符 |
| `zk compose --check` | 生成物与提交不一致 |
| 迁移不可变 | 历史迁移文件哈希变化 |
| 权限矩阵 | 有端点没声明权限点，或管理台页面权限不在集合内 |
| 品牌残留 | `hide` 渠道出现官方品牌字符串/资源；`show` 渠道出现非固定 Powered by 文案 |
| 保留期 | 审计 <180 天或用量 <12 个月（配置项低于下限） |
| 未签名/未公证 | Windows 不签名（已定）；macOS 证书配置前不公证（发布 job 标记，不阻塞内部构建） |
| 体积预算 | 客户端/管理台 bundle、镜像体积超阈值 |
| 供应链 | 新增高危漏洞/禁用许可 |
| 企业边界（弱管控） | 企业版出现 provider/key/base_url 可编辑入口；客户端可绕过服务端网关直连上游；禁用用户 JWT 仍可用；上述任一出现即失败（对应 ADR-030/031/027） |

## 6.5 产品矩阵与失败注入（必须与层矩阵同时跑）

产品矩阵：`edition ∈ {personal, enterprise} × channel ∈ {official, beta, oem-demo}`，每格至少跑：启动/登录/首配/对话/更新/回滚。

失败注入清单（每项必须有期望输出与断言）：

| 场景 | 期望 |
| --- | --- |
| 网关上游 5xx/超时/首字节后断流 | 结算一次、错误码封闭、不重试双扣 |
| DB 不可用/磁盘满/审计写失败 | 全业务 fail-closed（含 AI）；告警；禁止补偿后写 |
| 客户端断线/取消/崩溃 | usage pending 收敛、会话恢复、无重复消息 |
| 更新下载/校验/安装失败 | 回滚上一版、诊断可导出 |
| 迁移 checksum 变/重复版本/顺序漂移 | 拒启；`zk db lock` 不能洗白 |
| 技能 zip 穿越/zip bomb/symlink | 客户端拒绝安装、清理临时目录 |
| 应用窗口调用 `ipc_call` | 必失败（capability 隔离） |
| `serve` Host/Origin 异常/DNS rebinding | 拒绝；不执行 agent 命令 |
| 钱包并发/重放/负余额 | 不超发；余额=Σ账本；hold 无泄漏 |

> 技能解压安全用例属于**客户端**（服务端不解压）；不要放在服务端测试层。

## 7. 真实模型与外部依赖

- 默认走 mock/replay（确定性、离线）；真模型测试单独 job，密钥隔离，按需跑。
- 网关测试用假上游（本地 mock server），覆盖 OpenAI 兼容协议与流式。
- e2e 不依赖公网；OEM 渠道测试用本地 fixture 客户仓。

> 静态检查/类型/边界校验的全局规则见 `18-strictness.md`；本节只讲 UI 层。

> 客户端交互验收见 `19`；运维/安全/钱的门禁见 `20/21/22`。

## 8. 客户端 UI 质量保障（自研聊天层）

**前提**：聊天 UI 的 bug 集中在少数"看起来简单"的地方；不靠小心，靠结构 + 测试。

### 8.1 先买后写（不重造难的部分）

| 部分 | 策略 |
| --- | --- |
| 基础原语（弹窗/菜单/滚动/焦点） | Radix UI（无样式、可访问性已验证） |
| 流式 Markdown | 现成库（react-markdown + remark/rehype + sanitize；或 Vercel Streamdown），**不自己写解析器** |
| 代码高亮 | Shiki（构建期/懒加载） |
| 长会话列表 | TanStack Virtual，**不自己写虚拟滚动** |
| 聊天层参考/起步 | assistant-ui（Radix + Tailwind，MIT）作为 `ui-chat` 的初始实现，包在我们的契约后面；按需替换 |
| 自己写 | 只写差异化部分：工具调用时间线、审批卡片、diff/文件预览、企业内容卡片、OEM 主题 |

原则：**能包一层就不 fork；fork 后必须能独立测试与替换。**

### 8.2 结构：协议 → reducer → 视图

- 服务端/agent 事件 schema 由 Rust 契约生成 TS（ts-rs），UI 只消费生成类型。
- 所有流式事件进入**纯 reducer**（`agentTurnReducer(state, event)`），UI 渲染只读 reducer 结果。
- reducer 无 DOM/时间/网络依赖 → 可用"golden traces"（录制的事件序列）做确定性测试。
- 未知事件/未知字段忽略并记日志，不崩溃（前向兼容）。

### 8.3 必测的"事故高发区"

| 区域 | 测试方式 |
| --- | --- |
| 流式分片（含中文/emoji 跨 UTF-8 边界） | 把完整响应按随机边界切块，断言渲染结果一致 |
| 中断/取消（每个事件点都取消） | 在每个事件索引处 abort，断言状态一致、无重复消息 |
| 重连/重放/seq 断档 | 注入丢包、重复、乱序，断言 resync 行为 |
| 中文 IME 组合输入 | Playwright 键盘序列（compositionstart/end、Enter 行为） |
| 滚动锚定 | 在底部时跟随、用户上滚后不跳；长会话+流式组合 |
| 不完整 Markdown（未闭合代码块/表格/链接） | 渲染快照 + XSS sanitize 用例 |
| 工具调用时序/并行/失败 | golden traces + 错误注入 |
| 附件上传/进度/失败/大文件 | 组件测试 + e2e |
| 主题/OEM/深浅色 | 每个主题/渠道截图对比（Playwright screenshot） |
| 可访问性 | axe-core（焦点、aria、对比度） |
| 平台差异 | Linux WebKitGTK / Windows WebView2 / macOS WKWebView 冒烟 |

### 8.4 CI 门禁（客户端 UI）

1. `tsc --noEmit` + ESLint（含 react-hooks 规则）
2. Vitest：reducer golden traces + 组件六态（loading/empty/error/offline/streaming/cancelled）
3. Playwright：mock IPC 下跑聊天/取消/重试/附件/审批；按 theme × profile 截图
4. axe-core 无 serious/critical 违规
5. bundle 体积预算 + React 单实例校验
6. 真应用冒烟（Tauri + Xvfb）：窗口、发送、流式、取消
7. 错误边界覆盖率：每个 slot 必须有 fallback（缺一个即失败）

### 8.5 线上保障

- 每个 UI 包有 error boundary + 可读 fallback（componentId + 包 id），错误上报（GlitchTip/Sentry 可选）。
- 灰度：按渠道/人群开启新版 UI；关键路径埋点（首 token、取消、重试、错误）。
- 组件状态清单进 PR 模板：空/加载/错误/离线/流式中/已取消 六态必须覆盖。
- 每个 UI 包 README 写"已知限制 + 不支持场景"，避免误用。
