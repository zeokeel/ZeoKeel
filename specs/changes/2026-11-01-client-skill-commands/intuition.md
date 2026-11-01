# command-skill-commands · 直觉判定（INT）

> active；证据根 target/evidence/2026-11-01-client-skill-commands/intuition/；用户测试 **UNVERIFIED**。

| ID | 原则 | 场景 | 期望 | 反例 | 验证 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| INT-01 | 可发现性 | `/` 技能组 | 技能组可见、列名/描述；`/name` 可发现 | 技能隐藏/只剩占位 | e2e+review | CMD3S-SKILL-01（interaction/CMD3S-SKILL-01.json）；W1 |
| INT-02 | 预期一致 | 选择/删除 | `/name` 插入可移除 chip，不立即发送；删除后不再包含 | 立即发送/不可撤销 | e2e | CMD3S-SKILL-01 |
| INT-03 | 状态可见 | 加载/插入/发送 | loading/空/不可用可见；插入/移除状态清楚 | 无反馈/假插入 | e2e | CMD3S-SKILL-01/ERR |
| INT-04 | 就近反馈 | 未知/超限/不可用 | 技能组/行内可读错误；不直出码 | 全局 toast/错误码 | e2e | CMD3S-ERR-01 |
| INT-05 | 默认安全 | enterprise 未安装/路径 | personal `.agents/skills`；enterprise `$ZEOKEEL_HOME/skills/<name>/<version>`；未安装禁用；不越权 | 读未安装/路径越界/第二套扫描 | e2e+review | CMD3S-SKILL-01；W3 |
| INT-06 | 可恢复 | 移除/错误/刷新 | 移除后消息不含技能；错误可重试；组可重开 | 删除不可/缓存脏 | e2e | CMD3S-SKILL-01/ERR |
| INT-07 | 认知负担 | 技能组 | 名+描述、过滤；不一次堆全量 | 无描述/难筛 | review | W2 |
| INT-08 | 文案引导 | 空/不可用 | 空态「暂无可用技能」；企业未安装说明下一步 | 空白/“unavailable”直出 | review+i18n | W4 |
| INT-09 | 键盘/焦点 | 组/chip | 键盘可达/焦点回 composer | 只能鼠标 | e2e | CMD3S-KEY-01 |
| INT-10 | 空/错误 | 无技能/离线 | 可读空/错误不白屏 | 空白 | e2e | CMD3S-ERR-01 |
| INT-11 | 感知性能 | 打开 `/` | 技能组 loading ≤100ms，不阻塞输入 | 菜单卡 | e2e/perf | W1 |
| INT-12 | 单一模式 | 全流程 | 只经 tool-skill/`zk-composed` facade/单一菜单，无第二套扫描/chip | 两套技能源/host direct import | review+C17 | W4 |

## 覆盖
- CMD3S-SKILL-01→INT-01/02/03/05/06/11；P1 KEY→02/09；ERR→04/08/10；A11Y→09/10/12。
- 用户测试 UNVERIFIED；无 OPEN P0（walkthrough W1/W3 关闭）；active。
