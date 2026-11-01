# side-panels（Phase 2b）· 直觉判定（INT）

> 本文件与 `interaction.md` 同目录（第七件，`docs/19` §11.1、`docs/27` §2.7/§2.8、`docs/11` §8.6）。
> 只写本 surface 的直觉判据与映射；原则表/走查方法不在本文件复制。
> 状态：`planned`（实现开始时**与 `interaction.md` 同时**改 `active`；planned 只 WARN，不参与门禁强制）。
> 证据根：`target/evidence/2026-11-01-client-side-panels/intuition/`；走查见 `intuition-walkthrough.md`。
> 说明：`e2e proxy` 只证明可观察行为，不证明用户觉得直观；最终需 fresh-context 评审 + ≥5 人用户测试（当前 **UNVERIFIED**）。

| ID | 原则 | 场景 / top task | 用户直觉期望（可观察） | 反例 | 验证方式 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| INT-01 | 可发现性 | T1 打开右栏/文件树（P1） | 右栏 tab（文件/变更/终端）与刷新/折叠入口常驻可见；会话行 `⋯` 在 hover 与 focus 都出现且名字可读；终端 tab 名称直白 | tab 只在 hover 出现；`⋯` 只有鼠标悬停才可见；入口靠培训 | e2e proxy + review | INS2-TREE-01（target/evidence/2026-11-01-client-side-panels/interaction/INS2-TREE-01.json；e2e/interaction/inspector-files.spec.ts）、INS2-TERMINAL-01（target/evidence/2026-11-01-client-side-panels/interaction/INS2-TERMINAL-01.json；e2e/interaction/inspector-terminal.spec.ts）、INS2-SES2-OPS-01（target/evidence/2026-11-01-client-side-panels/interaction/INS2-SES2-OPS-01.json；e2e/interaction/sessions-ops.spec.ts）、review: walkthrough W1/W5/W6 |
| INT-02 | 预期一致 | T2 键盘操作（P2） | tab 切换按 ←/→；树展开用 Enter/Space；菜单 Esc 关最上层；终端输入 Enter 执行；与既有设置/命令面板一致 | Esc 关闭整栏；方向键切 tab 与树冲突；Enter 在树中打开文件而非展开 | e2e proxy | INS2-PERSIST-02（target/evidence/2026-11-01-client-side-panels/interaction/INS2-PERSIST-02.json；e2e/interaction/inspector-layout.spec.ts）、INS2-TREE-01（target/evidence/2026-11-01-client-side-panels/interaction/INS2-TREE-01.json；e2e/interaction/inspector-files.spec.ts）、review: walkthrough W1/W12 |
| INT-03 | 状态可见 | T1/T2 加载/终端/搜索（P1/P3） | 目录展开 ≤300ms 显示行内 loading；终端执行显示运行态；内容搜索显示 loading/命中数；动作 ≤100ms 有 busy | 点开无反馈；终端命令无运行态；搜索无结果/无 loading | e2e proxy + review | INS2-TREE-02（target/evidence/2026-11-01-client-side-panels/interaction/INS2-TREE-02.json；e2e/interaction/inspector-files.spec.ts）、INS2-TERMINAL-02（target/evidence/2026-11-01-client-side-panels/interaction/INS2-TERMINAL-02.json；e2e/interaction/inspector-terminal.spec.ts）、INS2-SEARCH-01（target/evidence/2026-11-01-client-side-panels/interaction/INS2-SEARCH-01.json；e2e/interaction/sessions-search.spec.ts）、review: walkthrough W2/W5/W9 |
| INT-04 | 就近反馈 | T2 会话操作（P1/P3） | rename 空/超长、copy id 成功、fork/archive 失败都在行/菜单附近提示；不弹无关全局 toast | 只弹「操作失败」且列表无变化；错误不指出哪个操作 | e2e proxy + review | INS2-SES2-OPS-01（target/evidence/2026-11-01-client-side-panels/interaction/INS2-SES2-OPS-01.json；e2e/interaction/sessions-ops.spec.ts）、INS2-SES2-OPS-02（target/evidence/2026-11-01-client-side-panels/interaction/INS2-SES2-OPS-02.json；e2e/interaction/sessions-ops.spec.ts） |
| INT-05 | 默认安全 | T2 归档/fork/终端（P1/P2） | archive 仅隐藏、可恢复且有筛选；fork 仅当前 workspace；终端命令由用户显式执行，不在后台自动运行 | archive 直接删除；fork 跨 workspace；打开 terminal 自动执行命令 | e2e proxy + review | INS2-SES2-OPS-01（target/evidence/2026-11-01-client-side-panels/interaction/INS2-SES2-OPS-01.json；e2e/interaction/sessions-ops.spec.ts）、INS2-SES2-OPS-02（target/evidence/2026-11-01-client-side-panels/interaction/INS2-SES2-OPS-02.json；e2e/interaction/sessions-ops.spec.ts）、INS2-TERMINAL-01（target/evidence/2026-11-01-client-side-panels/interaction/INS2-TERMINAL-01.json；e2e/interaction/inspector-terminal.spec.ts）、review: walkthrough W5/W6/W7 |
| INT-06 | 可恢复 | T2 失败/刷新（P2/P3） | 目录加载失败行内重试；预览失败可关闭返回树；搜索失败可重试；刷新后 tab/宽度/折叠与已展开目录恢复 | 一次失败整栏白屏；刷新丢展开状态；预览失败卡死 | e2e proxy | INS2-TREE-02（target/evidence/2026-11-01-client-side-panels/interaction/INS2-TREE-02.json；e2e/interaction/inspector-files.spec.ts）、INS2-PREVIEW-03（target/evidence/2026-11-01-client-side-panels/interaction/INS2-PREVIEW-03.json；e2e/interaction/inspector-files.spec.ts）、INS2-PERSIST-01（target/evidence/2026-11-01-client-side-panels/interaction/INS2-PERSIST-01.json；e2e/interaction/inspector-layout.spec.ts）、INS2-SEARCH-02（target/evidence/2026-11-01-client-side-panels/interaction/INS2-SEARCH-02.json；e2e/interaction/sessions-search.spec.ts） |
| INT-07 | 认知负担 | T1/T2 树/搜索（P1/P2） | 树按层级/kind 展示，不一次展示全树；搜索「标题/内容」模式切换明确；命中只显示片段，不整段倾倒 | 平铺 300 条路径；两种搜索模式无区分；命中整段长文 | review（Hick/Miller） | INS2-TREE-01（target/evidence/2026-11-01-client-side-panels/interaction/INS2-TREE-01.json；e2e/interaction/inspector-files.spec.ts）、INS2-SEARCH-01（target/evidence/2026-11-01-client-side-panels/interaction/INS2-SEARCH-01.json；e2e/interaction/sessions-search.spec.ts）、review: walkthrough W2/W8 |
| INT-08 | 文案即引导 | T1/T2 空/错误/降级（P1/P3） | 空目录/无命中/PDF/二进制/超限/终端不可用都给「下一步或支持类型」；不直出内部错误码 | “file/unsupported”直出；空目录只有空白 | review + i18n | review: walkthrough W3/W4、i18n-check |
| INT-09 | 键盘/焦点 | T2 键盘/读屏（P2/P3） | tab/树/菜单/终端/搜索键盘可达；分隔条有 aria；关闭预览/菜单焦点回入口；搜索框不吞快捷键 | 只能鼠标操作；Esc 后焦点丢失；分隔条不可键盘调宽 | e2e proxy | INS2-PERSIST-02（target/evidence/2026-11-01-client-side-panels/interaction/INS2-PERSIST-02.json；e2e/interaction/inspector-layout.spec.ts）、INS2-A11Y-01（target/evidence/2026-11-01-client-side-panels/interaction/INS2-A11Y-01.json；e2e/interaction/inspector-a11y.spec.ts）、INS2-TREE-01（target/evidence/2026-11-01-client-side-panels/interaction/INS2-TREE-01.json；e2e/interaction/inspector-files.spec.ts） |
| INT-10 | 空态/错误态 | T1/T2 六态（P3） | 空工作区/无匹配/离线/流式中/已取消都有分区级提示，可返回/重试，不白屏 | 全栏白屏；错误静默；流式中状态丢失 | e2e proxy | INS2-ERR-01（target/evidence/2026-11-01-client-side-panels/interaction/INS2-ERR-01.json；e2e/interaction/inspector-a11y.spec.ts）、INS2-PREVIEW-02（target/evidence/2026-11-01-client-side-panels/interaction/INS2-PREVIEW-02.json；e2e/interaction/inspector-files.spec.ts）、INS2-SEARCH-02（target/evidence/2026-11-01-client-side-panels/interaction/INS2-SEARCH-02.json；e2e/interaction/sessions-search.spec.ts） |
| INT-11 | 感知性能 | T1 打开右栏/展开（P2） | 打开右栏 ≤100ms 显示骨架；目录懒加载不阻塞其它目录；搜索防抖 ≥250ms、旧请求取消 | 打开右栏卡 1s；展开大目录整栏卡住；搜索每键发请求 | e2e/perf + review | INS2-TREE-01（target/evidence/2026-11-01-client-side-panels/interaction/INS2-TREE-01.json；e2e/interaction/inspector-files.spec.ts）、INS2-SEARCH-01（target/evidence/2026-11-01-client-side-panels/interaction/INS2-SEARCH-01.json；e2e/interaction/sessions-search.spec.ts）、review: walkthrough W12 |
| INT-12 | 单一模式 | T1/T2 菜单/预览/终端（P1/P2） | 会话菜单复用现有 Modal/菜单焦点模式；右栏 tab/预览只有一套；终端错误/危险确认与其它 surface 一致 | 两套会话菜单 Esc 行为不同；预览与文件树两套焦点；确认文案不一致 | review + C17 | review: walkthrough W6/W7 |

## 覆盖说明

- P0 交互 ID → INT 映射（每个 P0 至少 1 条）：
  - `INS2-TREE-01` → INT-01/INT-02/INT-07/INT-09/INT-11；
  - `INS2-TREE-02` → INT-03/INT-06/INT-10；
  - `INS2-PREVIEW-01` → INT-03/INT-07/INT-10；
  - `INS2-TERMINAL-01` → INT-01/INT-03/INT-05/INT-09；
  - `INS2-PERSIST-01` → INT-06/INT-11；
  - `INS2-SES2-OPS-01` → INT-01/INT-04/INT-05/INT-12；
  - `INS2-SEARCH-01` → INT-03/INT-07/INT-11。
- 用户测试（≥5 人 think-aloud）：**UNVERIFIED**（未执行；发布前按 `intuition-walkthrough.md` §4 执行；未执行前**任何 acceptance/发布文案不得声称“符合直觉/直观”**）。
- 未解决 P0 摩擦：**无 P0 摩擦**（见 `intuition-walkthrough.md` §3；F1/F3 已按 design/tasks 最小修法关闭）。
- 状态：`planned`；实现开始时**与 `interaction.md` 同时**改 `active` 并补对应 e2e 证据/走查复核。
