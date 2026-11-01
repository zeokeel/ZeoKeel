# command-compact · 直觉判定（INT）

> 状态 planned；证据根 `target/evidence/2026-11-01-client-compact-command/intuition/`；用户测试 **UNVERIFIED**。

| ID | 原则 | 场景 | 期望 | 反例 | 验证 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| INT-01 | 可发现性 | `/` 菜单 | compact 在「指令」可见/可执行，无「未实现」 | 命令藏/仍禁用 | e2e+review | CMD3C-COMPACT-01（interaction/CMD3C-COMPACT-01.json）；W1 |
| INT-02 | 状态可见 | 执行后 | busy→「已有较早消息压缩」+摘要/条数/usage | 无反馈/假成功 | e2e | CMD3C-COMPACT-01 |
| INT-03 | 可恢复 | 失败/超限/流式 | 历史不变、错误可读、可重试 | 失败丢历史 | e2e | CMD3C-ERR-01 |
| INT-04 | 认知负担 | 查看分隔 | 摘要可折叠，不倾倒全文；明确压缩范围 | 摘要占满屏 | review | W2 |
| INT-05 | 默认安全 | 再次执行/边界 | 无新消息 no-op；不重复写；原始消息仍可见/可搜索/fork | 重复压缩/丢原消息/搜索不到旧消息 | e2e | CMD3C-COMPACT-01；W3 |
| INT-06 | 文案引导 | 空/失败 | 说明压缩了什么/下一步；不直出内部码 | 空白/错误码直出 | review+i18n | CMD3C-ERR-01；W4 |

## 覆盖说明
- `CMD3C-COMPACT-01`→INT-01/02/05；P1 KEY→INT-02/04；ERR→INT-03/06。
- 用户测试 UNVERIFIED；未执行前不得声称符合直觉。
- 未解决 P0 摩擦：无（walkthrough W1/W3 关闭）。
- planned；实现开始时与 interaction 一起 active。
