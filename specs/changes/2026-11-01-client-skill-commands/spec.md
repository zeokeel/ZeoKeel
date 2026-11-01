# 2026-11-01-client-skill-commands · 行为规格

> 状态 draft；D5/D6 已定 Lead 2026-11-01。

## 1. 需求
### R1 · 技能候选
- **WHEN** 用户在 composer 词首输入 `/`（或 `/<query>`）
- **THEN** 「技能」组列出当前可用 **user-invocable** 技能：personal 仅 workspace `.agents/skills`（`user-invocable != false`）；**enterprise 仅 `$ZEOKEEL_HOME/skills/<name>/<version>` 已安装技能（按 edition 读 `skill_root`/`installed_skills`；版本选择 frontmatter，不读未安装内容）**；不可用/未安装诚实禁用并说明。
- **AND** 技能名/描述来自 `tool-skill` 单实现（frontmatter 解析；无描述降级 name；`list` 默认 text 兼容旧换行，host 用 `format:"json"`）；**host 经 `zk-composed` generated facade（C4），不直接 import tool_skill**；不新建目录扫描；按名过滤大小写不敏感；`/skill` 不再 `command/unimplemented`。
- **AND** enterprise 未安装/版本不可用时不显示假技能；personal 空态「暂无可用技能」；流式中允许打开/插入（不发送）；enterprise 路径/版本越界/非 frontmatter 拒绝。

### R2 · 插入 Instructions
- **WHEN** 用户选择 `/name`
- **THEN** host 读 `SKILL.md`（≤8 KiB；超限 `skill/too-large`），在 composer 插入可移除 **Instructions** 块（标技能名/来源），不立即发送；正文/其它 chip 保留。
- **AND** 发送时 Instructions 作为用户消息的 instructions 上下文块（与引用块共存，cap 8 KiB）进入上下文；`chat.send` 契约不变；移除后消息不含技能；未知/不可用/超限可读。
- **AND** personal 只读 workspace `.agents/skills`；enterprise 只读 `$ZEOKEEL_HOME/skills/<name>/<version>` 已安装内容；均经 safe path/名称/版本校验；不跨 workspace；不读未安装；不写日志；Instructions 不持久化扫描缓存。

## 2. 边界/错误
| 场景 | 期望 | 错误码 |
| --- | --- | --- |
| 未知技能 | 可读，不插入 | `skill/not-found` |
| enterprise 未安装/版本不可用 | 禁用/提示，不读 | `skill/unavailable` |
| SKILL.md >8 KiB | 拒绝 | `skill/too-large` |
| 空技能/损坏 frontmatter | 降级 name/无描述，不白屏 | — |
| 跨 workspace/不存在 | 拒绝 | 既有 session/* |
| 离线/流式 | 六态；插入允许但不发送 | 既有文案 |

## 3. 权限与可见性
- personal 本地；enterprise 仅已安装商店技能（不越权/不读未安装）；无企业策略/服务端新端点；只当前 workspace。

## 4. 数据兼容
- 无 DB/文件迁移；Instructions 仅内存/消息文本；`SKILL.md` 只读；无技能时当前行为；旧客户端不调用新 IPC；回滚移除 UI/IPC 即可。

## 5. 验收断言
- [ ] 技能组来源正确（personal/enterprise）；不可用禁用；无第二套扫描。
- [ ] `/name` 插入可移除 Instructions ≤8 KiB；发送进入上下文；未知/超限错误。
- [ ] 六态/i18n/axe/键盘；错误码登记；C20 P0 证据。
- [ ] 无自动执行/自动发送/服务端变更；用户测试 UNVERIFIED。

## 6. 未决
- [x] D5/D6 已定；实现细节 design/tasks。
