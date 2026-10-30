# `.agents/skills/` — 项目 Agent Skills

本目录存放 ZeoKeel 的**可复用工作流**，按 Agent Skills 约定：每个技能一个目录 + `SKILL.md`
（YAML frontmatter `name`/`description`，正文渐进披露）。

| Skill | 何时用 |
| --- | --- |
| `zeokeel-change` | 任何代码/契约/数据/行为变更的入口 |
| `zeokeel-add-package` | 新增/替换/删除包 |
| `zeokeel-docs` | 改文档/决策/SSOT |

规则：

- Skills 只写**步骤、命令、完成判据**；规范仍以 `docs/README.md` 的 SSOT 为准，不复制正文。
- 新增 Skill 必须：触发条件清晰、只读入口明确、有验证命令、列禁止项。
- Skills 可带脚本时，脚本视为供应链：必须 CODEOWNERS 评审，禁止读取密钥/上传数据。
- 发现 Skill 与 `AGENTS.md`/`docs/` 冲突：以用户当前指令 > `AGENTS.md` > `docs/` 为准，并修 Skill。
- 不要让 Skills 变成第二套文档；长内容拆 `references/`，规范链接 SSOT。
