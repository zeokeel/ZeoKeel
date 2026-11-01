# 2026-11-01-client-settings-ia · 行为规格

> 只写「系统必须怎样」，不写实现选型；实现见 `design.md`。

## 1. 需求

### R1 · 分区导航

- **WHEN** 用户打开设置（侧栏入口或 ⌘/Ctrl+K 命令）
- **THEN** 弹出设置窗口：左侧为分区导航（每项一个 pane），右侧为当前 pane 内容；默认选中第一个可用 pane。
- **AND** pane 列表 = 构建期贡献（`ui.settings.pane`）+ 内建外壳项；personal 与 enterprise 的 pane 集合按 edition 不同。
- **AND** 窄窗（< 720px）时导航退化为顶部横向 tab，内容区不横向滚动。

### R2 · pane 贡献与替换

- **WHEN** 某个 UI 包在 `zk.toml` 声明 `[[ui.contributions]] slot="ui.settings.pane"` 且被当前 edition 选中
- **THEN** `zk compose` 生成的 registry 暴露有序 `SettingsPanes`（按 `order` 升序、稳定排序），设置外壳按下标渲染。
- **AND** 同一 slot 允许多个贡献；不同包贡献相同 `order` 时按包 id 字典序稳定排序。
- **AND** 未选中该包时该 pane 不出现，外壳不报错（personal/enterprise 差异由此实现）。

### R3 · 字段迁移与校验

- **WHEN** 用户编辑任一 pane 字段并保存
- **THEN** 校验规则、错误 i18n、错误定位与改造前一致（Rust `AppSettings::validate` 与前端 schema 校验均执行）。
- **AND** `provider`/`embedding`/`media` 仍只在 personal 出现；`server_url`/登出/网关模型仍只在 enterprise 出现。
- **AND** 每个 pane 单独保存时只提交该 pane 的改动（外壳负责合并最新文档），保存结果回写表单。

### R4 · 保存并发（revision 栅栏）

- **WHEN** 前端读取设置
- **THEN** `settings.get` 返回 `{ settings, revision }`；`revision` 每次成功保存递增。
- **WHEN** 前端保存时携带的 `expected_revision` 与当前不一致
- **THEN** 服务端拒绝保存（不写盘），返回可读冲突错误；前端保留草稿并提示「设置已在别处更新，请重新加载」。
- **AND** 未携带 `expected_revision`（旧客户端）时保持旧行为（不校验），不得破坏兼容。

### R5 · 无障碍与六态

- **WHEN** 用户使用键盘/读屏/窄窗
- **THEN** Tab 顺序 = 导航 → 当前 pane 字段 → 底部保存/取消；Esc 关闭；关闭后焦点回到打开入口。
- **AND** pane 切换不丢失未保存草稿（草稿属于设置窗口会话）。
- **AND** 保存中/失败/成功三态可见，axe 0 serious/critical。

## 2. 边界与错误

| 场景 | 期望行为 | 错误码/提示 |
| --- | --- | --- |
| 保存校验失败 | 不提交，字段内联错误 + 焦点到首个错误 | 现有 i18n 校验键 |
| revision 冲突 | 不写盘，提示重新加载并保留草稿 | `settings/revision-conflict`（新增 i18n） |
| pane 组件渲染异常 | `SlotErrorBoundary` 局部降级，不白屏，可切到其它 pane | 现有 boundary 文案 |
| 设置文档过旧（缺新字段） | 反序列化用默认值；保存时补齐 | `#[serde(default)]` |
| 无任何 pane 贡献 | 显示空态与「恢复默认」入口，不白屏 | 新增 i18n 键 |

## 3. 权限与可见性

- 角色：personal 用户仅本地设置；enterprise 员工与超管看到相同 pane 集合（设置不是管理台能力）。
- 服务端不下发设置策略（弱管控，C6）；企业仅 `server_url`/登出/网关模型由登录态决定。
- 敏感字段（api key）保持 write-only/掩码，不回显。

## 4. 数据与兼容

- `SettingsGetResult` 增加 `revision: u64`（`#[serde(default)]`）；`SettingsSetPayload` 增加
  `expected_revision: Option<u64>`（`#[serde(default)]`）。
- 生成 schema/TS 同步（C10/C11）；旧 `AppSettings` 文档无需迁移。
- 写操作幂等：相同 revision 重试安全；冲突返回错误不写。

## 5. 验收断言（可执行）

- [ ] `zk compose --edition personal --profile default --channel official --check` 绿，生成 `SettingsPanes`。
- [ ] 单测：多贡献排序（order/包 id）稳定；revision 冲突不写盘。
- [ ] e2e：设置打开→遍历 pane→改字段→保存→刷新保持；键盘/axe 断言绿。
- [ ] personal 构建不含 enterprise pane；enterprise 构建不含 personal-only pane。
- [ ] `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build` 绿。

## 6. 未决问题

- [ ] pane 粒度过细（每个 fieldset 一个包）还是适中（每域一个包）？建议适中，本轮只在 `ui-settings` 内分文件。
- [ ] 是否把 provider/embedding/media 的字段组件迁到各自现有包（`ui-settings-provider` 已存在）？建议 provider 先迁，其余后续。
