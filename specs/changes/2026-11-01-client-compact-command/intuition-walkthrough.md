# command-compact · 认知走查

> 方法 docs/27 §2.8；专家走查非用户测试；状态 planned。

## 1. Personas
P1 首次使用者（怕丢聊天）；P2 高频键盘；P3 出错/辅助技术。

## 2. Top tasks
T1 长会话 `/compact` 后继续；T2 键盘执行+查看分隔；T3 provider 失败/离线恢复。

### 3×3
| Persona\Task | T1 compact | T2 键盘 | T3 失败恢复 |
| --- | --- | --- | --- |
| P1 | W1 | W2 | W3 |
| P2 | W4 | W5 | W6 |
| P3 | W7 | W8 | W9 |

## 3. 走查/摩擦
| # | P/T | 步骤 | ①目标 | ②控件 | ③反馈 | ④恢复 | 卡点 | INT | 严重度 | 修法 | 状态 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| W1 | P1/T1 | `/compact`→看分隔→继续问/搜索旧消息 | 想压缩 | 菜单/分隔可见 | 成功+摘要/条数；旧消息仍可见/可搜 | 失败可重试 | 失败丢历史/旧消息搜索不到 | INT-01/02/05 | **P0** | 原始 messages_in + compacted view 仅分隔/replay；失败不写 | closed（spec R2/design §2.2–§2.3、tasks #3–5） |
| W2 | P1/T2 | 展开摘要 | 想理解 | 摘要可展开 | 摘要/范围可读 | 折叠回 | 摘要倾倒 | INT-04 | P1 | 折叠/范围文案 | closed（design §2.4） |
| W3 | P1/T3 | provider 失败 | 想知道原因 | 错误可见 | 历史不变、可重试 | 重试成功 | 丢历史/假成功 | INT-03/05 | **P0** | 失败不 append；错误码；测试 | closed（spec R2/design §2.2） |
| W4 | P2/T1 | 键盘 `/co`+Enter | 不碰鼠标 | 菜单可键盘 | busy/成功 | Esc 回 composer | 焦点丢 | INT-02 | P1 | 焦点留输入框 | closed（interaction CMD3C-KEY-01） |
| W5 | P2/T2 | 键盘展开摘要 | 键盘 | 分隔可聚焦 | 展开/折叠 | Esc 折回 | 键盘不可达 | INT-04 | P1 | 分隔 button/tabindex | closed（interaction） |
| W6 | P2/T3 | 流式中尝试 | 想压缩 | 命令禁用 | 提示停止后 | 停止后可用 | 流式仍压缩 | INT-03 | P1 | streaming guard | closed（spec R1/R2） |
| W7 | P3/T1 | 读屏执行 | 完成 | 菜单/分隔 aria | 摘要可读 | 回输入框 | 读屏不可读 | INT-01/02 | P1 | aria/axe | closed（interaction CMD3C-A11Y 由 ERR 测试覆盖） |
| W8 | P3/T2 | 键盘摘要/取消 | 完成 | 按钮可 Tab | 状态可读 | 焦点回 | 焦点丢 | INT-04 | P1 | 键盘/焦点 | closed |
| W9 | P3/T3 | 离线/失败后重试 | 恢复 | 错误可见 | 历史不变 | 重试 | 白屏 | INT-03/06 | P1 | 六态/重试 | closed（spec R2） |

**P0 摩擦结论**：W1/W3 已关闭，无 OPEN P0；3×3 满格；可进入实现（待独立评审）。

## 4. 用户测试
≥5 人 think-aloud；T1–T3；证据 usability/；结论 **UNVERIFIED**；未执行前不得声称符合直觉。
