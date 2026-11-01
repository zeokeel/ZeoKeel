# tool-skill (`tool.skill`)

**功能说明**：技能目录的唯一实现。`list`/`read` 同时服务 agent 工具与宿主 `skill.list`/`skill.read`
（经 `zk-composed` 生成 facade，`2026-11-01-client-skill-commands`）：

- 工作区技能：`<workspace>/.agents/skills/<name>/SKILL.md`；`list` 默认仍输出旧换行目录名（agent 兼容），
  `format:"json"` 输出 `{skills:[{name,description,user_invocable}]}`（frontmatter 前 4 KiB，缺省 name=目录名、
  description 空、user-invocable=true；坏 frontmatter 降级不 panic）。
- 已安装商店技能（enterprise）：`$ZEOKEEL_HOME/skills/<name>/<version>/SKILL.md`；按确定性版本序选最新，
  未安装版本返回 `skill/unavailable`，不读取未安装内容。
- `read`/Instructions 内容 ≤8 KiB（`skill/too-large`）；名称/版本逐组件校验（禁 `..`/绝对/控制字符/符号链接逃逸）。
- 只读：不写日志、不写缓存、不跨工作区。
