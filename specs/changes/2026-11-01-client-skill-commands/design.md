# 2026-11-01-client-skill-commands · 设计

> 状态 draft；D5/D6 已定。共享设计：三子 spec 串行 compact→plan-mode→**skill-commands**；共享文件由先写者写，本 spec rebase 追加；错误码 `skill/*` 登记 docs/26 §5；菜单占位由 Batch1 catalog 改真实。

## 0. 决策
D5：tool-skill 单实现；personal 本地 user-invocable；enterprise 已安装；Instructions 可移除、不立即发送；D6：不自动执行/发送。

## 1. 影响
- 契约 `skill.list`/`skill.read` + DTO/错误码；`tool-skill` 扩展 frontmatter/limit（单实现，list 默认 text 兼容旧换行，`format:"json"` 新格式）；**`zk-composed` generated facade（`tools/zk/src/main.rs`）暴露技能读取**；host 经 facade（不 direct import tool_skill，C4）；ui-chat 技能候选/Instructions；App/controller/i18n；e2e。
- 无 DB/文件迁移；不新增 capability/服务端；generate schema。

## 2. 方案
### 2.1 tool-skill 单实现
- `list`: 遍历 `.agents/skills`（现有）；读每个 `SKILL.md` 前 4 KiB 解析 YAML frontmatter `name/description/user-invocable`（缺省 name=dir，description="", user_invocable=true；坏 frontmatter 降级不 panic）；返回行 `name\tdescription\t1|0` 或 JSON? 为 C10 稳定用 **JSON array** `{"skills":[{name,description,user_invocable}]}`；截断既有。
- `read`: name 经 `safe_path`，内容 ≤8 KiB（超出 ToolError；host map `skill/too-large`）；保持 `SkillTool` spec/approval 不变；**`list` 默认仍返回旧换行 name（agent 工具兼容），新增可选 `format:"text"|"json"`，json 返回 `{skills:[{name,description,user_invocable}]}`（host 用 json）**。
- personal 只 `.agents/skills`；**enterprise 只 `$ZEOKEEL_HOME/skills/<name>/<version>`**（`skill_root`/`installed_skills`；选已安装版本，frontmatter/user-invocable；safe path/name/version 校验；不读未安装）；不扫描其它目录；无新后端。

### 2.2 host IPC
- `skill.list {workspace_id, conversation_id?}` → host 经 **`zk-composed` generated facade** 调技能 list（`format:"json"`；facade 内部调用 tool-skill 单实现），解析 DTO；不 direct read_dir/不 direct import tool_skill（C4）。
- `skill.read {workspace_id, conversation_id, name, version?}` → 经 facade 读；personal workspace `.agents/skills`；enterprise `$ZEOKEEL_HOME/skills/<name>/<version>` 已安装（未指定版本选已安装最新/排序规则明确）；name/version/路径校验；返回 `{name,version?,content,source}`；>8 KiB/未知/越界→`skill/too-large`/`skill/not-found`/`skill/unavailable`。
- enterprise 未安装/`.agents/skills` 不存在→空列表/`skill/unavailable`（UI 禁用原因）；不读商店未安装内容；workspace 归属校验；无日志。

### 2.3 UI
- Batch1 catalog：`slash-skill` 占位移除；技能组由 `skill.list` 动态候选（与菜单同一 `CommandMenu` listbox；`/<query>` 过滤）；`/` 打开时异步加载 loading/空/错误。
- 选择 `/name`：调 `skill.read`；在 composer 插入 **Instructions chip**（可移除；标技能名/来源；≤8 KiB），不发送；正文/引用/附件保留；`/skill` 命令本身可显示技能组/帮助。
- 发送：序列化 Instructions 到用户消息尾部/前部 block `--- skill instructions ---`（与 references 块共存；总 cap 8 KiB；路径/内容控制字符处理）；`chat.send` 不变；移除 chip 后不再包含。
- 六态/i18n/axe/焦点回 composer；键盘 ↑↓/Enter/Tab/Esc 同 Batch1；流式中可打开/插入但不发送。

## 3. 备选
新扫描器（C17 拒绝）；立即发送（不可编辑，拒绝）；只列不插入（不满足 1:1）；读未安装商店（权限拒绝）。

## 4. 安全
- 只 `.agents/skills` + safe_path；name/path 校验；≤8 KiB；不跨 workspace；不泄露未安装内容；Instructions 用户数据不写日志；无自动执行/网络。

## 5. 测试
| 层 | 用例 | 命令 |
| --- | --- | --- |
| tool | list 旧 text 兼容/json/坏文件/路径；read 8 KiB；enterprise path/version | cargo test -p zk-tool-skill |
| host | facade 调用/list/read 来源/归属/未知/超限；enterprise skill_root 版本/越界；personal；无 direct import | cargo test -p zk-desktop；`zk compose --check` |
| UI | 技能组 loading/空/错/过滤/插入/移除/发送块/键盘 | pnpm ui-chat/app test:unit |
| e2e | personal 技能插入/发送/移除；enterprise 未安装禁用；axe | pnpm e2e；e2e/interaction/command-skill.spec.ts |
| 门禁 | schema/i18n/gate | tasks §4 |

## 6. 文档
docs/19 技能组/Instructions；docs/26 IPC/错误码/来源；KNOWN enterprise 未安装禁用/不自动执行。
