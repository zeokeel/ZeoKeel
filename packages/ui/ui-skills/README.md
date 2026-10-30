# @zeokeel/ui-skills (`ui.slot.skills`)

**功能说明（M3）**：企业客户端技能面板。

- 目录来自 `/api/client/v1/skills`（approved/可见/can_install）
- 安装：调 app IPC `skills.install`（下载/校验/解压由 app/Rust 完成）
- 显示版本/归档 sha/强制更新/已安装/冲突/错误
- 不执行技能；安装路径由 app 管理
