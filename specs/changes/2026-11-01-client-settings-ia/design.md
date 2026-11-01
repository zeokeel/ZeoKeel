# 2026-11-01-client-settings-ia · 技术设计

> 规范细节链接 `docs/`，不在这里复制第二份规范。组合/manifest 以 `24` 为准，API 以 `04` 为准。

## 1. 影响面

| 类型 | 变更 |
| --- | --- |
| 包 | 修改 `zeokeel/ui-settings`（外壳+共享 pane 贡献）、`zeokeel/ui-settings-provider`（改挂 `ui.settings.pane`）、`zeokeel/ui-kit`（共享表单类型/pane props）；不新增 Rust 包 |
| 契约 | `zk-contract-ipc`：新增 `SettingsGetResult`/`SettingsSetResult`（含 revision）、`SettingsSetPayload.expected_revision`；`zk-contract-config` 的 `SettingsStore` trait 增加 `revision()`/`save_with_revision()`（默认实现保持旧行为） |
| 路由 | 无（本地 IPC） |
| 迁移 | 无数据库迁移；revision 存 `store-settings-json` 旁挂 `settings.meta.json`（`{schema:1,revision:u64,settings_hash}`，原子写 + 故障回滚）；`settings.json` schema 不变 |
| 生成物 | `apps/desktop/web/src/generated/registry.ts`（新增 `SettingsPanes`）、`shared/contracts/ui-contributions.json`、schema/TS（生成器写） |
| edition/profile/channel | `[[ui.contributions]]` 新增可选 `editions` 字段（缺省=全部）；`zk compose` 按 edition 过滤贡献；personal-only 包继续走 `editions/personal-only.toml` 清单 |
| 文档 | `docs/16`（slot 表 + 多贡献规则）、`docs/24`（组合/registry 生成规则）、`docs/19`（设置状态）；本机制**不新增 ADR**（T6：属 `docs/16`/`docs/24` 既有机制扩展） |

## 1.5 复用检查（C17，必填）

| 要实现的点 | 现有能力（capability id / 路径） | 结论 | 归属 |
| --- | --- | --- | --- |
| 设置弹窗外壳/焦点/Esc | `@zeokeel/ui-kit` `useDialogFocus`、`SlotErrorBoundary`；`ui-settings` 现有 Radix Dialog | 复用 | ui-kit |
| 表单字段/校验 | `packages/ui/ui-settings/web/src/settings-validation.ts`（表单↔`AppSettings` 映射） | 扩展（类型提升到 ui-kit） | ui-kit + ui-settings |
| pane 扩展机制 | `[[ui.contributions]]` + `zk compose` registry（`tools/zk/src/main.rs` `slot_binding`） | 扩展（多贡献 list slot + `editions` 过滤） | 生成器 |
| provider 表单 | `zeokeel/ui-settings-provider`（已存在 `ui.settings.provider` 贡献） | 复用（改挂新 slot） | ui-settings-provider |
| 设置持久化 | `packages/client/store-settings-json`（原子写 JSON） | 扩展（旁挂 revision 元数据） | store-settings-json |

- 搜索证据：`rg -n "settings.provider|slot_binding|revision" tools/zk packages/ui packages/contracts packages/client/store-settings-json`。
- 为什么不能复用/扩展：现有 slot 生成器「一个 slot 一个组件导出」，无法表达多 pane 有序贡献。
- **不新增 `@capability`**：slot 机制不是 Rust 实现能力（`zk capability` 只扫 Rust `src/`），不登记 INDEX；本 spec 验收 `zk capability --check` 保持绿即可。
- 为什么不能复用/扩展（字段）：现有 `ui.settings` 单组件 + 单 `renderProvider` 注入无法表达多 pane 的有序贡献；在 `ui-settings` 内硬编码会继续违反「UI 也是包」的可替换目标。

## 2. 方案

1. **契约**：`SettingsGetResult { settings: AppSettings, revision: u64 }`、
   `SettingsSetResult { settings: AppSettings, revision: u64 }`、`SettingsSetPayload.expected_revision: Option<u64>`；
   `SettingsStore` 增加 `fn revision(&self) -> Result<u64, ConfigError>` 与
   `fn save_with_revision(&self, settings: &AppSettings, expected: Option<u64>) -> Result<u64, ConfigError>`
   （默认实现：无 revision 存储时 `revision()=0`、`save_with_revision` 忽略 expected 并返回 0，保持第三方实现可用）。
2. **存储**：`store-settings-json` 在 `settings.json` 同目录写 `settings.meta.json`（`{schema:1,revision:u64,settings_hash}`，原子写 + fsync）；
   读取失败/缺失/hash 不匹配 → revision=0；保存事务：备份旧 settings → 写 settings（tmp+rename）→ 写 meta（tmp+rename）；
   meta 失败则恢复旧 settings 并返回错误；**回滚失败边界**：若恢复备份也失败，则标记 `settings.meta.json` 为 corrupt（删除 meta）、返回错误，下次加载按 revision=0 重建并提示用户设置可能需重存；测试覆盖损坏/只读/并发写/故障注入。
3. **生成器**：(a) `UiContributionDecl` 增加可选 `editions: Vec<String>`（serde default=空=全部），`zk validate` 校验取值；
   (b) `slot_binding` 增加 `ui.settings.pane` 的 **list** 类型，生成
   `export const SettingsPanes: ReadonlyArray<SettingsPaneContribution> = [{ id, order, Component: cN["component"] }, ...]`，
   仅收集「包被选中 + contribution.editions 含当前 edition」的贡献；按 `order` 升序 + 包 id 字典序稳定排序；空列表生成 `[]`；
   (c) personal/enterprise 两套快照测试（含 edition 过滤）。
4. **内建 pane 也走贡献**：`ui-settings` 在自身 `zk.toml` 声明
   `general(10)/models(20)/agent(30)/tools(40)/search(50)/embedding(60, editions=["personal"])/media(65, editions=["personal"])/enterprise(80, editions=["enterprise"])/about(90)` 贡献；
   provider 由 `ui-settings-provider`（order 21, editions=["personal"]）贡献；外壳只做导航与保存，不硬编码字段。
5. **pane props（不含 edition 分支）**：`SettingsPaneProps { t, draft, update, errors, saving }`；
   personal/enterprise 差异由「贡献声明 editions + 构建期过滤」表达（C2），pane 内不按 `edition` 分支。
6. **保存语义**：单一底部保存；外壳合并全部 pane 草稿为完整 `AppSettings`，携带 `expected_revision` 提交；
   成功后用返回的 `{settings, revision}` 重置所有草稿基线；冲突时保留草稿并提示重新加载；失败时 settings 不变（事务语义见 §2 第 2 条）。
7. **前端 revision**：`controller.saveSettings` 记录 revision；`settings.get` 初始化；冲突映射为 i18n `settings/revision-conflict`。

### 2.1 关键流程

```text
打开设置 → settings.get → SettingsGetResult{settings,revision} → 渲染 SettingsPanes
        → 编辑草稿（窗口内保留）→ 保存：validate → settings.set{settings, expected_revision}
        → ok: SettingsSetResult{settings,revision} 重置基线
        → conflict: 提示重载不写盘；invalid: 内联错误
```

### 2.2 数据结构

```text
SettingsGetResult  { settings: AppSettings, revision: u64 }
SettingsSetResult  { settings: AppSettings, revision: u64 }
SettingsSetPayload { settings: AppSettings, expected_revision: Option<u64> }
settings.meta.json { schema: 1, revision: u64, settings_hash: string }
SettingsPaneContribution { id: string, order: number, Component: ComponentType<SettingsPaneProps> }
// 注：editions 只存在于 zk.toml 的 contribution 声明，用于 compose 过滤；生成数组只含已过滤的条目
```

## 3. 备选与取舍

| 方案 | 优点 | 缺点 | 结论 |
| --- | --- | --- | --- |
| A. 多贡献 slot + ui-settings 外壳 | 可替换、构建期组合、与 DSH 结构对位 | 生成器需改动 | 采用 |
| B. 在 ui-settings 内硬编码分页 | 改动小 | 无法换包/贡献，继续结构债 | 放弃 |
| C. 设置独立窗口/路由 | 空间大 | 改 Tauri 窗口与 a11y/焦点路径，超出本轮 | 放弃（后续可评估） |
| D. revision 写进 AppSettings | 改动小 | 旧客户端回写会把 revision 归零 | 放弃 |
| E. revision 旁挂元数据 | 兼容旧文档 | 需改 store-settings-json | 采用 |
| F. contribution 增加 editions 过滤 | 一个共享包即可表达两版差异，构建期组合 | 生成器/manifest 机制扩展，需 docs/16+24 先行 | 采用（P1-R1） |
| G. edition 专属 pane 拆包 | 不动生成器 | 新增 2–3 个四件套包，预算更大 | 放弃 |

## 4. 安全与隐私

- 设置含 api key（write-only）；revision 冲突不写盘，避免静默覆盖密钥配置。
- pane 贡献在构建期确定，无运行期加载（C2）；新 pane 包必须走 `zk validate`/conformance。
- 旁挂元数据只存单调计数，无敏感信息；原子写失败不改变 `settings.json`。
- 无新增网络调用；无审计要求（本地）。

## 5. 发布与回滚

- 无灰度；随客户端版本发布。
- 回滚：代码回退即可；`settings.json` 不变，`settings.meta.json` 可删除（回退为 revision=0）。
- 客户端兼容：旧前端不传 revision 仍可保存；旧版本忽略 meta 文件。

## 6. 测试计划

| 层 | 用例 | 命令 |
| --- | --- | --- |
| T0 组合 | 新 slot 生成、edition 过滤、personal/enterprise 快照、空列表、`--check` | `zk validate && zk compose --edition personal --profile default --channel official --check` |
| T1 单包 | revision 递增/冲突/兼容；pane 排序/edition 过滤；表单校验；meta 损坏/hash 不匹配；故障注入（settings 成功+meta 失败回滚） | `cargo test -p zk -p zk-contract-ipc -p zk-contract-config -p zk-store-settings-json -p zk-desktop`；`pnpm --dir packages/ui/ui-settings test:unit` |
| T1 生成物 | schema/TS 导出 | `cargo test --workspace export_schema && git diff --exit-code`；`bash scripts/schema-check.sh` |
| T2/T3 | e2e 设置分页/保存/刷新/窄窗/axe/C14 | `pnpm e2e`（m1-a11y + 设置用例；真实 serve profile） |
| T4/T5 | enterprise 组合构建 + 截图 | `zk compose --edition enterprise ... && pnpm -r build` |

## 7. 文档更新

- 任务 0（先于代码）：`docs/16` §3 增「多贡献 slot（`ui.settings.pane`）排序/空列表/`editions` 过滤/生成物」；`docs/24` §1/§3 增 registry 生成规则；
  `docs/19` 设置状态更新（分页/冲突态）。本机制不新增 ADR（T6）。
- 任务末：`docs/16`/`docs/28`/`docs/19` 回写实现状态；parity 报告 §7 更新。
- `docs/README.md` 主题登记：否。

## 8. 风险

| 风险 | 概率 | 影响 | 缓解 | 验证 |
| --- | --- | --- | --- | --- |
| 生成器改动引入 slot 回归 | 中 | 高 | 保留旧单 slot 行为；新增 list 类型与快照 | `zk compose --check` + 全量 e2e |
| pane props 类型漂移 | 中 | 中 | ui-kit 单一类型 + ts 编译门禁 | `pnpm -r typecheck` |
| 设置字段迁移遗漏 | 中 | 高 | 字段 checklist（40 字段）+ 单测 | 对照 §2.1 清单 |
| revision 元数据并发写 | 中 | 中 | 原子写 + 单写者（设置保存串行化） | store 单测 |
| 旧客户端回写丢 revision | 低 | 中 | meta 独立文件，旧客户端不写 | 兼容测试 |
| contribution editions 过滤与旧 manifest 兼容 | 低 | 低 | `#[serde(default)]` = 全部 edition | 兼容单测 |
| 双文件保存中途失败 | 低 | 中 | 备份+回滚+hash 校验 | 故障注入测试 |
