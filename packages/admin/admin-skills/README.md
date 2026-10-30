# @zeokeel/admin-skills (`admin.page.skills`)

**功能说明（M3）**：管理台技能页。

- 列表：状态/名称/版本/force/可见性
- 创建技能、上传 zip（version + archive）、审批/拒绝/下架
- grants（全员/指定、can_install）
- 删除（软删）
- 只调 `/api/admin/skills*`；使用 `@zeokeel/admin-ui-kit` 的 `api`/`apiForm`。
- 权限 `skills:admin`；不执行技能、不解析 zip。
