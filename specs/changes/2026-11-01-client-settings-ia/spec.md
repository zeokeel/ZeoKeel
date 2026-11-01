# 2026-11-01-client-settings-ia · 行为规格

> 只写「系统必须怎样」，不写实现选型；实现见 `design.md`。

## 1. 需求

### R1 · 分区导航

- **WHEN** 用户打开设置（侧栏入口或 ⌘/Ctrl+K 命令）
- **THEN** 弹出设置窗口：左侧为分区导航（每项一个 pane），右侧为当前 pane 内容；默认选中第一个可用 pane。
- **AND** 所有 pane（含「通用/Agent/工具与审批/企业/关于」等内建 pane）统一由 `ui.settings.pane` 贡献表达：
  `ui-settings` 在自身 `zk.toml` 声明内建 pane 并给 `order`，第三方包同样声明；不存在第二套内建列表。
- **AND** **贡献可声明 `editions`**（`[[ui.contributions]] editions = ["personal"]` 等，缺省=全部 edition）：
  `zk compose` 按当前 edition 过滤贡献；personal 专属 pane（provider/embedding/media）与 enterprise 专属 pane（server_url/登出/网关模型）
  由此在**构建期**不同，pane 内不做运行期 edition 分支（C2）。
- **AND** 窄窗（< 720px）时导航退化为顶部横向 tab，内容区不横向滚动。

### R2 · pane 贡献与替换

- **WHEN** 某个 UI 包在 `zk.toml` 声明 `[[ui.contributions]] slot="ui.settings.pane"` 且被当前 edition 选中
- **THEN** `zk compose` 生成的 registry 暴露有序 `SettingsPanes`（按 `order` 升序；同 order 按包 id 字典序稳定排序），设置外壳按下标渲染。
- **AND** 固化 order 表：通用 10 / 模型与 Provider 20 / Provider 21 / Agent 30 / 工具与审批 40 / 网络与检索 50 /
  嵌入 60 / 媒体 65 / 企业 80 / 关于与更新 90（新增 pane 取未占用 order）。
- **AND** 未选中该包或 edition 不匹配时该 pane 不出现，外壳不报错；**无任何 pane 时 `SettingsPanes = []`**，外壳显示空态而非白屏。
- **AND** 生成器输出 personal/enterprise 两套 registry 快照测试（贡献顺序、edition 过滤、空列表、缺失 slot 无重复导出）。

### R3 · 字段迁移与校验

- **WHEN** 用户编辑任一 pane 字段并保存
- **THEN** 校验规则、错误 i18n、错误定位与改造前一致（Rust `AppSettings::validate` 与前端 schema 校验均执行）。
- **AND** `provider`/`embedding`/`media` pane 只在 personal 出现；`server_url`/登出/网关模型 pane 只在 enterprise 出现（由 R1 的 edition 贡献过滤实现）。
- **AND** **单一底部保存**：外壳把全部 pane 草稿合并为完整 `AppSettings` 后一次提交；不提供逐 pane 保存，避免「局部保存 + 旧 revision」的语义歧义。
- **AND** 保存结果（`settings + revision`）回写外壳与所有 pane 草稿基线。

### R4 · 保存并发（revision 栅栏）

- **WHEN** 前端读取设置
- **THEN** `settings.get` 返回 `SettingsGetResult { settings, revision }`；`revision` 每次成功保存递增。
- **WHEN** 前端保存成功
- **THEN** `settings.set` 返回 `SettingsSetResult { settings, revision }`（与 get 同构，避免二次读取）。
- **WHEN** 前端保存时携带的 `expected_revision` 与当前不一致
- **THEN** 服务端拒绝保存（不写盘），返回可读冲突错误；前端保留草稿并提示「设置已在别处更新，请重新加载」。
- **AND** 未携带 `expected_revision`（旧客户端）时保持旧行为（不校验），不得破坏兼容。
- **AND** revision 存储于 `store-settings-json` 的**旁挂元数据**（独立 `settings.meta.json`，与 `settings.json` 同目录，含 `settings_hash`），
  不写进 `AppSettings`，避免旧客户端回写把 revision 归零。
- **AND** 双文件写入的失败语义冻结：保存事务 = 备份旧 `settings.json` → 写新 settings（tmp+rename）→ 写 meta（tmp+rename）；
  若 meta 写失败则恢复旧 settings 并返回错误（**失败不落盘**）；若恢复也失败，则删除 meta 并返回错误，下次加载按 revision=0 重建并提示；
  meta 含 `settings_hash`，加载时 hash 不匹配 ⇒ revision=0 并重建；以故障注入测试覆盖「settings 成功 / meta 失败」「回滚失败」「meta 损坏」「并发写」四类场景。

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

- **新增契约 DTO**：`SettingsGetResult { settings: AppSettings, revision: u64 }`、
  `SettingsSetResult { settings: AppSettings, revision: u64 }`（`zk-contract-ipc`）；
  `SettingsSetPayload` 增加 `expected_revision: Option<u64>`（`#[serde(default)]`）。
- revision 旁挂存储：`store-settings-json` 新增 `settings.meta.json`（`{schema:1, revision:u64, settings_hash:string}`，原子写）；
  不存在/hash 不匹配时 revision=0；`AppSettings` schema 不变。
- 生成 schema/TS 同步（C10/C11）；旧 `AppSettings` 文档无需迁移。
- 写操作幂等：相同 revision 重试安全；冲突返回错误不写；meta 写失败回滚 settings。

## 5. 验收断言（可执行）

- [ ] `zk compose --edition personal --profile default --channel official --check` 绿，生成 `SettingsPanes`（personal/enterprise 两套快照；edition 过滤生效）。
- [ ] 单测：多贡献排序（order/包 id）稳定；edition 不匹配贡献被过滤；空贡献 `[]`；revision 冲突不写盘；revision 递增；
      meta 写失败回滚 settings；meta 损坏/hash 不匹配降级 revision=0。
- [ ] `cargo test --workspace export_schema && git diff --exit-code` 与 `bash scripts/schema-check.sh` 绿（新增 DTO 的 schema/TS）。
- [ ] e2e：设置打开→遍历 pane→改字段→保存→刷新保持；键盘/axe 断言绿。
- [ ] personal pane 数 ≥8（general/models/provider/agent/tools/search/embedding/media/about）；enterprise pane 数 ≥7（无 provider/embedding/media，含企业）；
      企业 `server_url` 锁定只读/改地址需重登不回归（C14）。
- [ ] `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build` 绿；`zk capability --check` 绿（本 spec 不新增 capability 条目）。

## 6. 未决问题

- 已定（2026-11-01）：pane 粒度取「每域一个 pane」；内建 pane 统一走贡献；贡献支持 `editions` 过滤（本 spec 扩展生成器，docs/16+docs/24 先行）；
  `ui.settings.pane` 是 slot 机制、不是 `@capability`，不登记 INDEX；provider 迁到独立 order 21；embedding/media 以 `editions=["personal"]` 贡献在共享包内；
  企业 pane 以 `editions=["enterprise"]` 贡献；设置搜索/快捷跳转非本 spec。
- 待定：无。
