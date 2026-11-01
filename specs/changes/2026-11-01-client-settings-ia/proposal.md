# 2026-11-01-client-settings-ia · 提案

- 状态：`draft`
- Owner：lead
- 日期：2026-11-01
- 关联：`research/client-dsh-parity-2026-11-01.md` §4/P1-1、§7 Phase 1；`docs/16-ui-packages.md`、`docs/28-ui-module-map-and-skeleton.md`（设置分页口径需同步）
- 评审：待独立评审（C19；approved 前补 `docs/reviews/<date>-client-settings-ia.md`）
- 变更预算：约 15 个文件（生成器 + ui-kit 类型 + ui-settings 重构 + 新增 1 个 pane 包 + 测试/文档）；超过 10 文件但为同一目标，不拆分

## 1. 背景 / 问题

用户反馈「UI 上都不能配置」。现状根因不在字段少（`AppSettings` 40 个叶子字段、personal 可编辑 37 个），
而在**设置面结构**：

- 全部字段挤在一个 618 行单页 dialog（`packages/ui/ui-settings/web/src/index.tsx:165-606`），无分区导航，
  字段一多就无法发现/定位；`docs/28:86` 的「分页/插件清单」从未落地。
- 唯一扩展点是 provider 表单的 `renderProvider` 注入（`ui.settings.provider` slot），其余分区写死在 dialog 内，
  新增能力（MCP、子代理、终端、jobs、审批策略）只能继续往单页里堆，形成结构债。
- `settings.set` 提交**整份文档**且无 revision 栅栏（`packages/contracts/ipc/src/lib.rs:240-248`），
  多窗口/并发保存会静默覆盖。
- 对比 DSH（0.2.1-rc.2）设置面：5 个导航分区 + 12 个「通用」行 + 侧栏插件页 4 个配置页 + 1 个只读清单，
  每个配置页由包贡献、可独立替换。ZeoKeel 当前为 0 分区，后续每加能力都要改同一个 dialog。

## 2. 目标

- 设置窗口改为**左侧分区导航 + 右侧内容**（Radix Dialog 内双栏；窄窗退化为顶部 tab），
  分区：通用/外观、模型与 Provider、Agent、工具与审批、网络与检索、嵌入与媒体（personal）、企业（enterprise）、关于与更新。
- 新增 `ui.settings.pane` **多贡献 slot**：pane 由 UI 包在 `zk.toml` 的 `[[ui.contributions]]` 声明
  （`slot = "ui.settings.pane"`、`component`、`order`），`zk compose` 生成有序 `SettingsPanes` 数组；
  `ui-settings` 只负责外壳/导航/保存，不再硬编码业务字段。
- 现有字段按 pane 拆分迁移，行为与校验不回归：provider（personal-only 包）、embedding/media（personal-only）、
  通用/Agent/工具/检索（ui-settings 或独立 pane 包）、企业 server_url/登出/网关模型。
- 保存语义升级：`settings.get` 返回 `revision`，`settings.set` 携带 `expected_revision`（serde default，
  旧客户端/旧文档兼容）；冲突时保留草稿并提示重新加载，不静默覆盖。
- 保持既有 a11y/六态/i18n/键盘行为（`e2e/m1-a11y.spec.ts` 覆盖的设置路径不得退化）。

## 3. 非目标

- 不新增配置语义（MCP server、审批策略、子代理上限、终端/Shell、jobs、schedule 任务管理）——这些属于后续 spec。
- 不改运行时/agent 行为；不改企业商店（skills/connectors/apps）面板。
- 不引入运行期插件加载；pane 仍是构建期组合（C2）。
- 不做设置搜索/快捷跳转（可后续增强）。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 个人用户 | 想改模型参数/主题/语言/检索端点 | 左侧分区一眼找到，改完该 pane 保存即生效 |
| 个人用户 | 同时开两个窗口改设置 | 后保存者收到冲突提示，不覆盖先保存者的改动 |
| 开发者/OEM | 新增一个设置页（如 MCP） | 只新增/替换 UI 包并声明 contribution，不改 `ui-settings` 外壳 |
| 企业用户 | 改服务端地址/登出/网关模型 | 在「企业」分区可见；与 personal 分区互不干扰 |

## 5. 成功指标

- 设置窗口分区数 ≥ 6（personal）/ ≥ 7（enterprise），每个 pane 可独立替换（换包只需改 edition 映射）。
- `zk compose --check` 对新增 slot 生成 `SettingsPanes` 且 `--check` 绿；`pnpm -r typecheck/lint/test:unit/build` 绿。
- 保存冲突用例：`expected_revision` 过期时返回可读错误且不写盘（Rust 单测 + 前端单测）。
- e2e：设置打开→切换每个 pane→改一个字段→保存→刷新后保持；键盘 Tab 序、Esc 关闭、焦点回位、axe 0 serious/critical。
- 现有设置相关单测/e2e 全绿，无 i18n 硬编码新增。

## 6. 风险与未知

| 风险/未知 | 影响 | 缓解/验证 |
| --- | --- | --- |
| `zk compose` 目前一个 slot 只绑定一个组件（`tools/zk/src/*.rs` `slot_binding`），多贡献需生成器改动 | 高 | 先改生成器 + `--check` 快照；新增 conformance 测试 |
| pane props 需统一类型，`SettingsForm` 现在私有在 `ui-settings` | 中 | 把表单类型/校验入口提升到 `@zeokeel/ui-kit`（共享层，C17） |
| Radix Dialog 双栏 + 窄屏 | 中 | 窄窗退化为顶部 tab；e2e 覆盖 1280/900/375 宽度 |
| revision 栅栏改 IPC 契约 | 中 | 新增字段 `#[serde(default)]`；旧前端不传 = 不校验，保持兼容 |
| 迁移期间字段回归 | 高 | 字段清单 checklist + 每个 pane 单测 + 现有 e2e |

## 7. 需要用户拍板

- [ ] **A. 设置字段组件归属**：拆成多个 UI 包（provider/embedding/media 已有包；其余拆 `ui-settings-agent`/`ui-settings-tools` 等），
      还是本轮先保留在 `ui-settings` 内部按 pane 分文件、只把扩展点做成 slot？推荐后者（小步、少建包）。
- [ ] **B. revision 栅栏是否本轮做**：推荐做（否则多窗口覆盖是真实风险）；若不做的代价是 Spec B 之后仍要返工。
- [ ] **C. 设置是否保持 Dialog**：推荐保持 Dialog（与现有焦点/axe 行为一致），不做独立页面。

> 未拍板前状态保持 `draft`，不进入实现。

## 8. 变更预算与拆分

- 预计改动文件：生成器 1、契约 1、ui-kit 1、ui-settings 5–8、pane 包 1–2、测试 3、文档 2 → 约 15。
- 拆分：不拆；本 spec 只做 IA/扩展点/保存语义；配置语义与能力面另立 spec（见 parity 报告 §7）。

## 9. 复用与已有实现（C17）

- 已搜索：`rg "settings.dialog|settings.provider|slot_binding"`；`docs/generated/CAPABILITY-INDEX.md`（ui.kit/ui.slot.* 角色）。
- 可复用：`@zeokeel/ui-kit`（UiProvider/Button/icons/guards）、`SlotErrorBoundary`、`useDialogFocus`、`zk compose` 的
  `[[ui.contributions]]` 机制、`shared/contracts/ui-contributions.json` 生成物。
- 本次新增可复用能力：`ui.settings.pane` slot 类型与 `SettingsPane` props 契约（登记在 `ui-kit/types.ts` + capability 注记）。
- 为什么不能复用/扩展：现有 `ui.settings` 单组件 + 单 `renderProvider` 注入无法表达多 pane 的有序贡献；
  在 `ui-settings` 内硬编码会继续违反「UI 也是包」的可替换目标。
