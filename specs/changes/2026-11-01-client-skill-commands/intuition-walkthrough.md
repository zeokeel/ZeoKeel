# command-skill-commands · 认知走查

> docs/27 §2.8；专家走查非用户测试；planned。

## Personas
P1 首次使用者（不知技能来源）；P2 键盘；P3 出错/辅助技术/企业未安装。

## Tasks
T1 `/` 找技能并插入；T2 移除/发送上下文；T3 企业未安装/键盘/错误恢复。

### 3×3
| Persona\Task | T1 技能 | T2 Instructions | T3 权限/键盘/恢复 |
| --- | --- | --- | --- |
| P1 | W1 | W2 | W3 |
| P2 | W4 | W5 | W6 |
| P3 | W7 | W8 | W9 |

## 走查/摩擦
| # | P/T | 步骤 | ①目标 | ②控件 | ③反馈 | ④恢复 | 卡点 | INT | 严重度 | 修法 | 状态 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| W1 | P1/T1 | `/` 打开技能组 | 想用技能 | 技能组可见/描述 | loading→列表/空 | 关闭回 composer | 只有占位/来源不明 | INT-01/03 | **P0** | 真实技能组+tool-skill 单源 | closed（spec R1/design §2.1–2.3、tasks #3–4） |
| W2 | P1/T2 | 选技能看 chip；发送 | 想插入 | Instructions chip 可移除 | 发送后上下文含技能 | 删除后不含 | 立即发送/不可撤销 | INT-02/07 | P1 | chip/发送块/移除测试 | closed（spec R2/design §2.3） |
| W3 | P1/T3 | enterprise 未安装/版本路径 | 想知道可用 | 禁用原因可读 | 不插入/不假可用 | 安装后重开 | 未安装内容泄露/路径越界 | INT-05/08 | **P0** | personal `.agents/skills`/enterprise `$ZEOKEEL_HOME/skills/<name>/<version>` + safe path/版本；host 经 facade | closed（spec R2/design §2.2、tasks #3b/#4） |
| W4 | P2/T1 | 键盘 `/sk` | 不碰鼠标 | 菜单可键盘 | 过滤/加载 | Esc 回 | 焦点丢 | INT-02/09 | P1 | 焦点留输入框 | closed（interaction KEY） |
| W5 | P2/T2 | 删除 chip 键盘 | 键盘 | chip 删除可聚焦 | 删除/焦点回 | 重新打开 | 不可达 | INT-02/09 | P1 | Delete/Enter；axe | closed（interaction A11Y） |
| W6 | P2/T3 | 错误后重试 | 恢复 | 错误可见 | 可重试 | 保留 chip | 白屏/假插入 | INT-04/06 | P1 | 六态/重试 | closed（spec R2） |
| W7 | P3/T1 | 读屏技能组 | 完成 | listbox/name | 名/描述可读 | 关闭回 | 不可读 | INT-01/03 | P1 | aria/axe | closed（interaction A11Y） |
| W8 | P3/T2 | 读屏 chip/发送 | 完成 | chip 可读/移除 | 上下文含 | 删除 | 不可读 | INT-02/08 | P1 | aria/标签 | closed |
| W9 | P3/T3 | 未安装+离线 | 恢复 | 禁用/错误 | 可读 | 重试/安装 | 假可用 | INT-05/10 | P1 | 禁用原因+六态 | closed（spec R2） |

**P0 结论**：W1/W3 关闭，无 OPEN P0；3×3 满格；可进入实现（待独立评审）。用户测试 UNVERIFIED。
