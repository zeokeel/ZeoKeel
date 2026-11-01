# 2026-11-01-client-skill-commands · 提案（Batch2-C：/<skill>）

- 状态：`approved`；Owner dsh-ref；日期 2026-11-01
- 关联：原父 `client-command-backends` 已按 Lead D7 拆分并删除；本 spec 为 skill-commands 子 spec；DSH §5 技能源；`tool-skill` 单实现；`docs/19` §2.1b/§11、`docs/27`、`docs/11` §8.6；前置 Batch1 command-surface
- 评审：docs/reviews/2026-11-01-client-command-batch2.md
- 预算：**10 文件**（≤10，见 §8；含 generated facade）

## 1. 背景
Batch1 `/` 菜单「技能」组仅有 Batch2 `/skill` 占位；`tool-skill` 只支持 agent 工具 list/read `.agents/skills`，composer 无技能候选/Instructions 插入；enterprise 商店安装技能无 `/name` 入口。用户要求 DSH 1:1。

## 2. 目标
- `/<skill>` 从占位转真实技能候选源：personal 本地 `.agents/skills` user-invocable；enterprise 已安装商店技能；不可用诚实禁用。
- 复用 `tool-skill` 单实现（不新建扫描）；`/name` 插入可移除 Instructions（≤8 KiB），随消息进入上下文，`chat.send` 不变。
- 错误码 `skill/*`；C20 两文件+interaction；六态/i18n/a11y；C10/C11/C17/C18。

## 3. 非目标
- 不做 compact/plan（另子 spec）；不新建 skill 目录扫描/商店后端；不自动执行技能/不绕过商店安装/权限；不做未安装内容读取。

## 4. 用户/场景
| 用户 | 场景 | 期望 |
| --- | --- | --- |
| personal 技能用户 | `/` 看技能 | 本地 user-invocable 技能名/描述；`/name` 插入 Instructions |
| enterprise 员工 | `/` 看技能 | 只列已安装商店技能；未安装/不可用禁用并说明 |
| 键盘/读屏 | 选择/移除 Instructions | 可达/可读/焦点回 composer |

## 5. 成功指标
- 技能组来源正确、无第二套扫描；personal/enterprise 差异正确；不可用 0 次假插入。
- `/name` 插入 ≤8 KiB、可移除、随消息进入上下文；未知/超限错误可读。
- P0 测试+证据；axe 0 serious；错误码登记。

## 6. 风险
| 风险 | 缓解 |
| --- | --- |
| 第二套扫描/权限越界 | 只经 tool-skill + `.agents/skills` + safe_path；测试 |
| enterprise 未安装内容泄露 | 只列已安装目录；不可用禁用；不读未安装 |
| Instructions 膨胀/不可移除 | ≤8 KiB；可移除 chip；发送前校验 |
| 10 文件预算 | 本 spec 只做 skill；共享文件后序 rebase |

## 7. 已定决策（Lead 2026-11-01，D5 推荐 A + D6）
- [x] **D5 skill 来源/权限/插入**（已定 A）：personal 仅 workspace `.agents/skills` 中 `user-invocable != false`；enterprise 仅 `$ZEOKEEL_HOME/skills/<name>/<version>` 已安装技能（按 edition 读；不读未安装）；`tool-skill` 单实现解析 frontmatter；`/name` 插入可移除 Instructions（≤8 KiB），不立即发送；发送时作为用户消息 instructions 块进入上下文，不改聊天契约；不可用诚实禁用。
- [x] **D6**：不做自动执行/自动发送；不做 compact/plan。
- [x] **拆分/串行**：本 spec 为 skill 子 spec；compact→plan-mode→**本 spec** 串行；共享文件由先写者写，本 spec rebase 追加。

## 8. 预算（10）
contract ipc1；tool-skill lib1；host ipc1；`tools/zk/src/main.rs`（generated facade）1；ui-chat AssistantComposer1；ui-kit context1；App1；controller1；i18n1；e2e interaction1 =10（≤10）。共享 docs/19、docs/26、KNOWN 由 compact 首写，本 spec 追加；protocol 并入 App；`tool-skill list` 兼容旧换行（默认 text，host 用 `format:"json"`）。
- 不新增 capability/DB/服务端；生成物由生成器写。

## 9. 复用
- Batch1 菜单/catalog/runCommand；`packages/client/tool-skill` list/read（扩展 frontmatter，不新建扫描）；`safe_path`；现有 Markdown/输入行/chip。
- 新增：UI 技能候选/Instructions；host `skill.list/read` IPC（wrapper）；`zk-composed` generated facade（C4）。
- 不能复用：composer 无候选/Instructions；tool-skill 仅 agent 工具；enterprise 安装目录读取无 IPC。

## 10. 文档
回写 docs/19（技能组/Instructions）、docs/26（IPC/错误码/来源）、KNOWN（enterprise 未安装禁用/不自动执行）。

- 预算例外：Lead 2026-11-01 追认实际约 38 个路径（规划 10；契约/tool-skill 单实现/host/UI/e2e/docs），单目标、无第二套实现，不拆分提交。
