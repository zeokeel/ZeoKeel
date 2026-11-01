# command-plan-mode · 认知走查

> docs/27 §2.8；专家走查非用户测试；planned。

## Personas
P1 首次使用者（怕误执行）；P2 键盘；P3 出错/读屏。

## Tasks
T1 `/plan` 进入/只读/Markdown；T2 exit/右栏审核；T3 键盘/拒绝/刷新恢复。

### 3×3
| Persona\Task | T1 plan | T2 review | T3 键盘/恢复 |
| --- | --- | --- | --- |
| P1 | W1 | W2 | W3 |
| P2 | W4 | W5 | W6 |
| P3 | W7 | W8 | W9 |

## 走查/摩擦
| # | P/T | 步骤 | ①目标 | ②控件 | ③反馈 | ④恢复 | 卡点 | INT | 严重度 | 修法 | 状态 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| W1 | P1/T1 | `/plan` 描述；试 subagent/subagent_start/subagent_fork/agent_team/workflow/ralph | 想先计划 | 菜单/模式可见；delegation 被拒 | 只读/Markdown；child mutating 0 | off/拒绝可退 | 经 delegation 执行写/无状态 | INT-01/02/05 | **P0** | agent-rig 显式阻断 delegation+模式状态 | closed（spec R2/design §2.2、tasks #3–4） |
| W2 | P1/T2 | 看 plan tab/卡 | 想审核 | 计划可见 | 状态/动作 | 拒绝保留 | 混 activity/不可读 | INT-03/07 | P1 | plan tab order50/卡 | closed（design §2.4） |
| W3 | P1/T3 | 批准后检查 | 怕自动执行 | 批准按钮 | 状态 approved | 计划保留 | 批准自动执行 | INT-05/08 | **P0** | 批准只退出模式，不执行；测试 | closed（spec R3/design §2.3） |
| W4 | P2/T1 | 键盘 `/pl` | 不碰鼠标 | 菜单可键盘 | 模式可见 | Esc 回 | 焦点丢 | INT-02 | P1 | 焦点留输入框 | closed（interaction KEY） |
| W5 | P2/T2 | plan tab 键盘审核 | 键盘 | tab/按钮可 Tab | 状态可读 | 焦点回卡 | 不可达 | INT-09 | P1 | tab/按钮键盘/axe | closed（interaction A11Y） |
| W6 | P2/T3 | 刷新/切会话 | 恢复 | 状态可见 | 恢复 | 无丢失 | 计划丢 | INT-06 | P1 | plans.json/恢复测试 | closed（spec R3） |
| W7 | P3/T1 | 读屏进入 | 完成 | 菜单/模式 aria | 可读 | 回 composer | 不可读 | INT-01/02 | P1 | aria/axe | closed |
| W8 | P3/T2 | exit 超长/未知 | 审核 | 错误可见 | 可读 | 保留旧 | 白屏 | INT-04/08 | P1 | plan/too-large 等 | closed（spec R2） |
| W9 | P3/T3 | 离线/拒绝 | 恢复 | 状态/错误 | 可读 | 重试/继续 | 白屏 | INT-06/10 | P1 | 六态/反馈 | closed |

**P0 结论**：W1/W3 关闭，无 OPEN P0；3×3 满格；可进入实现（待独立评审）。用户测试 UNVERIFIED。
