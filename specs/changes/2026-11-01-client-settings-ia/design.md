# 2026-11-01-client-settings-ia · 技术设计

> 规范细节链接 `docs/`，不在这里复制第二份规范。组合/manifest 以 `24` 为准，API 以 `04` 为准。

## 1. 影响面

| 类型 | 变更 |
| --- | --- |
| 包 | 修改 `zeokeel/ui-settings`（外壳+内建 pane）、`zeokeel/ui-settings-provider`（改为 `ui.settings.pane` 贡献）、`zeokeel/ui-kit`（共享表单类型/slot props）；不新增 Rust 包 |
| 契约 | `zk-contract-ipc`：`SettingsGetResult.revision`、`SettingsSetPayload.expected_revision`；`zk-contract-ui` 不变 |
| 路由 | 无（本地 IPC） |
| 迁移 | 无数据库迁移；设置 JSON 文档 `schema` 不变，新字段 `#[serde(default)]` |
| 生成物 | `apps/desktop/web/src/generated/registry.ts`（新增 `SettingsPanes`）、`shared/contracts/ui-contributions.json`、schema/TS（由生成器写） |
| edition/profile/channel | personal 与 enterprise 的 pane 集合由 `editions/*.toml` 映射决定；`official` 渠道不变 |
| 文档 | `docs/16` slot 表、`docs/28` 设置分页口径、`docs/19` 设置状态；`docs/README.md` 无需新主题 |

## 1.5 复用检查（C17，必填）

| 要实现的点 | 现有能力（capability id / 路径） | 结论 | 归属 |
| --- | --- | --- | --- |
| 设置弹窗外壳/焦点/Esc | `@zeokeel/ui-kit` `useDialogFocus`、`SlotErrorBoundary`；`ui-settings` 现有 Radix Dialog | 复用 | ui-kit |
| 表单字段/校验 | `packages/ui/ui-settings/web/src/settings-validation.ts`（表单↔`AppSettings` 映射） | 扩展（提升类型到 ui-kit） | ui-kit + ui-settings |
| pane 扩展机制 | `[[ui.contributions]]` + `zk compose` registry（`tools/zk/src/*.rs` `slot_binding`） | 扩展（多贡献 slot） | 生成器 |
| provider 表单 | `zeokeel/ui-settings-provider`（已存在 `ui.settings.provider` 贡献） | 复用（改挂到新 slot） | ui-settings-provider |

- 搜索证据：`rg -n "settings.provider|slot_binding" tools/zk packages/ui`；`shared/contracts/ui-contributions.json`。
- 为什么不能复用/扩展：现有 slot 生成器「一个 slot 一个组件导出」，无法表达多 pane 有序贡献。
- 新增可复用能力：`@capability ui.settings.pane`（slot 类型 + `SettingsPane` props）；登记位置 `packages/ui/ui-kit/web/src/types.ts` + `docs/generated/CAPABILITY-INDEX.md`（由 `zk capability` 生成）。

## 2. 方案

1. **契约**：`SettingsGetResult` 返回 `revision`（保存成功后自增，持久化到设置文件旁的 `revision` 字段或存储层元数据；
   旧文件按 0 处理）；`SettingsSetPayload.expected_revision: Option<u64>`，`Some` 且不匹配时返回 `settings/revision-conflict`。
2. **生成器**：`slot_binding` 增加 `ui.settings.pane` 的 list 类型，生成
   `export const SettingsPanes: ReadonlyArray<SettingsPaneContribution> = [{ id, order, Component: cN["component"] }, ...]`，
   按 `order` 升序 + 包 id 字典序稳定排序；`--check` 逐字节比较。
3. **ui-kit**：`SettingsForm`/`SettingsErrors`/`SettingsPaneContribution` 类型、`SettingsPaneProps`
   （`t`/`edition`/`draft`/`update`/`errors`/`saving`）；`ui-settings` 与 pane 包只依赖 ui-kit（C4）。
4. **ui-settings 外壳**：Dialog 内容改为 `flex` 双栏；左列 `role="tablist"` 导航；右列渲染选中 pane；
   窄窗（<720px）导航转顶部 tab；底部保存/取消统一由外壳管理；草稿状态在窗口生命周期内保留。
5. **pane 拆分**：`panes/general.tsx`（主题/语言/更新/关于）、`panes/agent.tsx`（completion/memory/hooks）、
   `panes/tools.tsx`（超时/自动批准只读等待接线字段）、`panes/web-search.tsx`、`panes/enterprise.tsx`；
   `provider`/`embedding`/`media` 保持现有 personal-only 包/分区，通过贡献挂入。
6. **前端 revision**：`controller.saveSettings` 记录 `revision`，冲突时 `toast`/内联提示并重新 `settings.get`。
7. **不再新增全局 `SettingsDialog` props**：新增 pane 通过 contribution，不再加 `renderXxx` 注入。

### 2.1 关键流程

```text
打开设置 → settings.get {settings,revision} → 渲染 pane 列表 → 编辑草稿
        → 保存：validate → settings.set{settings, expected_revision}
        → ok: 回写 revision/表单；conflict: 提示重载不写盘；invalid: 内联错误
```

### 2.2 数据结构

```text
SettingsGetResult { settings: AppSettings, revision: u64 }
SettingsSetPayload { settings: AppSettings, expected_revision: Option<u64> }
SettingsPaneContribution { id: string, order: number, Component: ComponentType<SettingsPaneProps> }
```

## 3. 备选与取舍

| 方案 | 优点 | 缺点 | 结论 |
| --- | --- | --- | --- |
| A. 多贡献 slot + ui-settings 外壳 | 可替换、构建期组合、与 DSH 结构对位 | 生成器需改动 | 采用 |
| B. 在 ui-settings 内硬编码分页 | 改动小 | 无法换包/贡献，继续结构债 | 放弃 |
| C. 设置独立窗口/路由 | 空间大 | 改 Tauri 窗口与 a11y/焦点路径，超出本轮 | 放弃（后续可评估） |

## 4. 安全与隐私

- 设置含 api key（write-only）；revision 冲突不写盘，避免静默覆盖密钥配置。
- pane 贡献在构建期确定，无运行期加载（C2）；新 pane 包必须走 `zk validate`/conformance。
- 无新增网络调用；无审计要求（本地）。

## 5. 发布与回滚

- 无灰度；随客户端版本发布。
- 回滚：代码回退即可；设置文档新字段向后兼容（旧版忽略未知字段，serde default 保证新版读旧文件）。
- 客户端兼容：personal/enterprise 均通过 `zk compose --check`；旧前端不传 revision 仍可保存。

## 6. 测试计划

| 层 | 用例 | 命令 |
| --- | --- | --- |
| T0 组合 | 新 slot 生成、edition 差异、`--check` | `zk validate && zk compose --edition personal --profile default --channel official --check` |
| T1 单包 | revision 冲突/兼容；pane 排序；表单校验 | `cargo test -p zk-contract-ipc -p zk-desktop`；`pnpm --dir packages/ui/ui-settings test:unit` |
| T2/T3 | e2e 设置分页/保存/刷新/窄窗/axe | `pnpm e2e`（m1-a11y + 设置用例） |
| T4/T5 | enterprise 组合构建 + 截图 | `zk compose --edition enterprise ... && pnpm -r build` |

## 7. 文档更新

- [ ] SSOT 文档：`docs/16`（slot 表）、`docs/28`（设置分页）、`docs/19`（设置状态）
- [ ] ADR/D：若 pane slot 成为通用机制，补 ADR；否则记录在 `docs/16`
- [ ] `docs/README.md` 主题登记：否（不新增主题）
- [ ] 迁移/发布说明：CHANGELOG

## 8. 风险

| 风险 | 概率 | 影响 | 缓解 | 验证 |
| --- | --- | --- | --- | --- |
| 生成器改动引入 slot 回归 | 中 | 高 | 保留旧单 slot 行为；新增测试 | `zk compose --check` + 全量 e2e |
| pane props 类型漂移 | 中 | 中 | ui-kit 单一类型 + ts 编译门禁 | `pnpm -r typecheck` |
| 设置字段迁移遗漏 | 中 | 高 | 字段 checklist（40 字段）+ 单测 | 对照 §2.1 清单 |
| revision 兼容旧客户端 | 低 | 中 | `Option<u64>` + 单测 | 契约测试 |
