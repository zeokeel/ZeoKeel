# zk-srv-skills (`srv.skills`)

**功能说明（M3）**：技能归档格式/目录/下载。

- `POST /api/admin/skills/{id}/versions`：multipart zip ≤16MB；只读 central directory，不解压（ADR-061）。
- 校验：条目名/类型/总量/条目数/SKILL.md；拒绝 symlink/hardlink/device/FIFO/路径穿越。
- 入库 `content_releases`（BYTEA、归档 sha256）；同内容同版本永久占位。
- `GET /api/client/v1/skills/{name}/{version}/archive`：授权后返回 zip；`X-ZeoKeel-Skill-Sha256` 为归档哈希。
- 不重复 content 状态机/审批；不执行技能。
