# side-panels（Phase 2b）· 认知走查（intuition walkthrough）

> 方法：`docs/27` §2.8（3 persona × 3 top task × 每步 4 问）；本走查是**专家走查**，不等于用户测试。
> 摩擦项引用 `intuition.md` 的 INT ID；P0 未解决不得进入实现/发布。
> 状态：`planned`（实现前先走查；实现后由 fresh-context 评审复核，见 §4）。

## 1. Personas

| ID | Persona | 特征 | 关注点 |
| --- | --- | --- | --- |
| P1 | 首次使用者 | 不懂代码、第一次用 ZeoKeel；习惯鼠标 | 右栏在哪里打开；文件树能否一眼看懂；点文件会发生什么；错误能否退出 |
| P2 | 高频键盘用户 | 每天使用、键盘流、熟悉 ⌘B/⌘K | 右栏/tab/树的键盘路径；会话菜单是否好找；终端命令是否明确由自己触发；搜索是否稳定 |
| P3 | 出错/辅助技术用户 | 键盘 + 读屏；离线/权限失败/大文件 | 错误可读可恢复；焦点/aria 正确；路径安全/跨 workspace 隔离不出错；六态不白屏 |

## 2. Top tasks（频率 × 失败代价）

| ID | Task | 频率 | 失败代价 | 对应 surface |
| --- | --- | --- | --- | --- |
| T1 | 打开右栏 → 浏览目录树 → 预览一个文本/图片文件 | 极高 | 高 | files 树/预览 |
| T2 | 打开终端执行一条用户命令；对会话做 rename/fork/copy id/pin/archive | 中 | 高（会话数据/C18） | terminal + sessions 菜单 |
| T3 | 用键盘/搜索完成「找到某个会话内容并恢复一次失败操作」 | 高 | 中 | 搜索/键盘/六态 |

### 2.1 3×3 覆盖矩阵（防漏格）

| Persona \ Task | T1 文件树/预览 | T2 终端/会话操作 | T3 搜索/键盘/恢复 |
| --- | --- | --- | --- |
| P1 首次使用者 | W1/W2/W3 | W10（N/A + 替代验证） | W11（N/A + 替代验证） |
| P2 高频键盘用户 | W12 | W6/W7 | W8 |
| P3 出错/辅助技术用户 | W4/W9 | W13 | W9 |

> 每个格子必须至少一个 W 行（或 N/A + 理由 + 替代验证）；N/A 不得留空。

## 3. Walkthrough 与摩擦表

每步回答：① 知道目标吗？② 看得到可操作控件吗？③ 能预判反馈吗？④ 出错能恢复吗？

| # | Persona/Task | 步骤 | ①目标 | ②控件 | ③反馈 | ④恢复 | 卡点 | INT | 严重度 | 最小修法 | 状态 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| W1 | P1/T1 | 打开右栏，找 files tab | 知道要看文件 | 右栏开关可见；tab 名「文件/变更/终端」 | 打开后显示树/空态 | 可关右栏 | 若右栏入口只在菜单里，首次用户找不到 | INT-01 | P1 | 保留 header 可见开关 + ⌘B 提示；tab 常驻 | closed（design §2.3/现有 header 行为） |
| W2 | P1/T1 | 点一个目录，再点一个文件 | 知道目录可展开/文件可预览 | 目录行有展开 affordance；文件行可点 | 行内 loading；预览区出现 | 预览可关闭返回树 | 目录展开无反馈或一次性展开全树会迷失 | INT-03/07 | P1 | 懒加载 + 行内 loading；一次只展开所需层级 | closed（design §2.3） |
| W3 | P1/T1 | 点 PDF/二进制/超大文件 | 知道这是文件、想预览 | 文件行可点 | 看到「不支持在线预览/大小」降级说明 | 可返回树继续 | 若空白或卡死，用户以为坏了 | INT-08/10 | P1 | 降级块说明类型/大小/支持范围；不阻塞 | closed（spec R2/design §2.3） |
| W4 | P3/T1 | 打开 `../`/workspace 外/符号链接文件 | 想看文件 | 树中不展示 symlink/越界项 | 显示可读拒绝错误 | 关闭预览回树 | 越界读取 = 安全/数据边界 | INT-05/10 | **P0** | `safe_join_under` 逐组件 no-follow+canonicalize+starts_with；UI 错误态；host 单测 | closed（design §2.3、tasks #6/12） |
| W5 | P2/T2 | 打开 terminal，执行 `printf hi` | 想用 shell 确认状态 | 「终端」tab、新建/输入框可见 | 运行态→输出出现；清空/关闭 | 命令失败/超时可读，会话仍在 | 若打开 terminal 自动跑命令或混淆为 agent 工具 | INT-01/03/05 | P1 | 用户显式 open/send；不生成工具行/审批；运行态/超时提示 | closed（spec R3/design §2.3） |
| W6 | P2/T2 | 给会话 rename/fork/pin/archive | 想整理会话 | 会话行 `⋯` hover/focus 可见 | 菜单项/成功提示/列表更新 | 空标题拒绝；失败列表不变；archive 可恢复 | `⋯` hover-only 首次用户找不到；archive 被误解为删除 | INT-01/04/05/12 | P1（发现性）/ **P0**（archive 语义/跨 workspace fork） | `⋯` focus 也可见+aria；archive=隐藏可筛选/恢复；fork 仅当前 workspace；流式中禁用 | closed（design §2.4、spec R5、tasks #9/#10/#11/#12） |
| W7 | P2/T3 | 切「内容」搜索输入，快速改名再查 | 想找历史内容 | 模式切换明确；结果按会话 | loading→结果/无命中；旧请求取消 | 失败可重试；空查询不请求 | 旧查询结果乱入、跨 workspace 泄露 | INT-03/07/10 | **P0**（跨 workspace）；P1（旧结果） | host `session.search` 仅当前 workspace；debounce≥250ms + 取消；错误可重试 | closed（design §2.3/§2.4、tasks #5/#8/#12） |
| W8 | P2/T2 | 拖拽调宽/折叠后刷新，切回会话 | 想保持布局 | 分隔条可拖/键盘；折叠按钮可见 | 宽度/折叠即时；刷新后恢复 | 脏数据回默认；窄窗自动收起 | 刷新丢状态/窄窗挤坏中栏 | INT-06/09/11 | P1 | 会话级 localStorage + 240–720px/≤50% 视口 + <768px 自动收起 | closed（spec R4、design §2.3） |
| W9 | P3/T3 | 读屏/键盘遍历右栏/会话菜单/搜索 | 完成 T1/T3 | 所有控件有 role/name；焦点可见 | aria 状态可读 | Esc/关闭焦点回入口；错误 role=alert | 焦点丢失/axe 违规即不可用 | INT-09/10/12 | P1 | ui-kit 原语/现有菜单模式；axe 0 serious；键盘 e2e | closed（interaction `INS2-A11Y-01`、design §2.3） |

| W10 | P1/T2 | 首次用户看到会话行 `⋯`/终端 tab，尝试整理/执行 | N/A：首任务不涉及会话整理和终端命令；频率低但危险操作需确认 | N/A：入口仍可见（W1/W6） | N/A | N/A | 不适用组合，不得漏格 | INT-01/07 | N/A | 替代验证：W1/W2 覆盖入口发现性，W6/W7 覆盖会话操作与安全语义 | N/A（替代验证见左） |
| W11 | P1/T3 | 首次用户用键盘/搜索找历史会话 | N/A：首任务不要求键盘流/跨会话搜索 | N/A | N/A | N/A | 不适用组合，不得漏格 | INT-09/10 | N/A | 替代验证：W8 覆盖键盘/搜索可恢复，W9 覆盖读屏 | N/A（替代验证见左） |
| W12 | P2/T1 | 键盘打开右栏→展开目录→预览文本，Esc 关闭预览 | 想不碰鼠标看文件 | tab/树/文件行可 Tab/方向键 | loading→预览出现；Esc 关闭 | Esc 后焦点回文件行/树 | 焦点丢失/方向键冲突 | INT-02/09/11 | P1 | tab/树 roving tabindex；Esc 关闭预览回原焦点；e2e 键盘序列 | closed（interaction `INS2-A11Y-01`、tasks #12） |
| W13 | P3/T2 | 读屏/键盘操作会话菜单 rename/archive，终端失败 | 想完成操作并听到结果 | 菜单/输入/确认按钮有 role/name | 成功/失败 aria-live；错误可读 | 取消/失败后焦点回 `⋯`；archive 可恢复 | 只能鼠标/焦点丢失/错误静默 | INT-04/09/10 | P1 | 复用现有菜单/Modal 焦点模式；错误 role=alert；axe 0 serious | closed（interaction `INS2-A11Y-01`、tasks #9/#12） |

**P0 摩擦结论**
- W4（预览路径越界）、W6（archive 语义/跨 workspace fork）、W7（内容搜索跨 workspace 泄露）三项 P0 卡点均已用最小修法关闭，并在 spec/design/tasks 落位；**无 OPEN P0 摩擦**；3×3 矩阵已满格（含 N/A + 替代验证），可进入实现（仍待评审复评）。

## 4. 用户测试计划（未执行 → UNVERIFIED）

- 对象：≥5 名真实用户（非实现者，至少 2 名不熟悉本产品）
- 任务：T1/T2/T3；think-aloud；不提示操作路径
- 指标：任务成功率 / 首次点击正确率 / 完成时间 / 错误数 / SEQ-SUS
- 证据：`target/evidence/2026-11-01-client-side-panels/usability/{script.md,notes/*.md,scores.json}`
- 结论：**UNVERIFIED**（未执行；执行前不得声称“符合直觉/直观”——`docs/11` §8.6、`docs/27` §2.8）

## 5. 评审复核（fresh context，待执行）

- 评审者（非实现者）按本走查复跑 P1–P3 × T1–T3，核对 `intuition.md` 的 P0 映射是否覆盖 interaction.md 全部 P0；
- 结论记录到 `docs/reviews/` 并回填 `acceptance.md`；P0 摩擦未关闭 → 不通过。
