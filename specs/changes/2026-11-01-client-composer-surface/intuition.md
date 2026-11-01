# composer（Phase 2a）· 直觉判定（INT）

> 本文件与 `interaction.md` 同目录（第七件，`docs/19` §11.1、`docs/27` §2.7/§2.8、`docs/11` §8.6）。
> 只写本 surface 的直觉判据与映射；原则表/走查方法不在本文件复制。
> 状态：`active`（实现阶段）；`INT` 不由 `@interaction` 门禁评分，质量由 reviewer 走查 + 用户测试判定。
> 证据根：`target/evidence/2026-11-01-client-composer-surface/intuition/`；走查见 `intuition-walkthrough.md`。

| ID | 原则 | 场景 / top task | 用户直觉期望（可观察） | 反例 | 验证方式 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| INT-01 | 可发现性 | T1 首次发送/引用文件（P1） | 发送、`/`、`@`、附件入口在不 hover 时可见；空态/placeholder 提示 Enter/Shift+Enter | `/`、`@` 只能靠记忆；附件入口 hover-only | e2e proxy + review | `CMP2-CMD-01`、`CMP2-AT-01`、走查 |
| INT-02 | 预期一致 | T1/T2 键盘（P2） | Enter 发送、Shift+Enter 换行、Esc 关最上层、↑↓ 只移高亮；与 ⌘K 相同命令行为 | Shift+Enter 发送、Esc 丢失草稿、方向键移动光标 | e2e proxy | `CMP2-KEY-01`、`CMP2-CMD-02` |
| INT-03 | 状态可见 | T1/T2 发送/上传（P1/P3） | 发送后 100ms 内出现流式/停止；附件上传中 chip 可见；失败 chip 可读 | 点击无反馈、重复提交、附件无可视状态 | e2e proxy + review | `CMP2-ATT-01`、`CMP2-ATT-02` |
| INT-04 | 就近反馈 | T2 附件错误（P3） | 类型/大小/数量错误就地提示并可移除/重试，草稿与其它 chip 保留 | 只弹全局 toast、错误后清空草稿 | e2e proxy + review | `CMP2-ATT-02` |
| INT-05 | 默认安全 | T2 发送带引用/附件（P1） | 未完成上传/失败附件不允许发送；发送内容可预期（引用块只在尾部） | 半成品附件被发送、引用块出现在正文中间 | e2e proxy + review | `CMP2-FILE-01`、`CMP2-ATT-02`、走查 |
| INT-06 | 可恢复 | T2 失败/关闭菜单（P2/P3） | 菜单 Esc 关闭保留草稿；发送失败沿用既有 toast/重试；引用块刷新后仍可识别 | Esc 清空输入、刷新后引用消失 | e2e proxy | `CMP2-CMD-02`、`CMP2-FILE-01` |
| INT-07 | 认知负担 | T1 输入面（P1） | `/` 菜单按 group 分组、过滤后才要求选择；`@` 只列当前 workspace；模型/用量只读不增负 | 一屏堆 20 条命令、`@` 混入 `.git`/附件目录 | review（Hick/Miller） | 走查 |
| INT-08 | 文案即引导 | T1/T2 空态/错误（P1/P3） | 空列表/错误/离线给下一步；无内部错误码直出 | “attachment/invalid”直出给用户 | review + i18n | 走查 + `i18n-check` |
| INT-09 | 键盘/焦点 | T2 菜单/附件（P2/P3） | 菜单打开时焦点留在输入框；关闭后回输入框；chip 删除按钮键盘可达 | 菜单抢焦点、Esc 后焦点丢失 | e2e proxy | `CMP2-KEY-01`、`CMP2-CMD-02` |
| INT-10 | 空态/错误态 | T1/T2 无匹配/离线（P3） | `/`、`@` 无匹配有可读空态；离线/加载/错误不阻塞继续输入 | 白屏或静默无候选 | e2e proxy | `CMP2-AT-03`、`CMP2-A11Y-01` |
| INT-11 | 感知性能 | T1 输入/菜单（P2） | 输入反馈 <16ms；菜单打开 <100ms；`@` 首次加载有 loading 状态 | 每次按键卡顿、菜单延迟无提示 | e2e/perf + review | 走查（`fileMenuLoading`） |
| INT-12 | 单一模式 | T1/T2 命令/菜单（P1/P2） | ⌘K 与 `/` 同一命令目录与执行入口；附件/引用同一 chip 模型；无第二套菜单/焦点 | 两处命令不一致、两套菜单 Esc 行为不同 | review + C17 | 单测 `buildCommandCatalog` + 走查 |

## 覆盖说明

- P0 交互 ID → INT 映射：
  - `CMP2-CMD-01` → INT-01/INT-12；`CMP2-CMD-02` → INT-02/INT-06/INT-09；
  - `CMP2-AT-01` → INT-01/INT-07；`CMP2-AT-02` → INT-07/INT-09；
  - `CMP2-ATT-01` → INT-03/INT-11；`CMP2-ATT-02` → INT-03/INT-04/INT-05；
  - `CMP2-KEY-01` → INT-02/INT-09；`CMP2-IME-01` → INT-02；
  - `CMP2-FILE-01` → INT-05/INT-06。
- 用户测试（≥5 人 think-aloud）：**UNVERIFIED**（隐藏式实现阶段未执行；发布前按 `intuition-walkthrough.md` §4 执行；未执行前不得声称"符合直觉"）。
- 未解决 P0 摩擦：**无 P0 摩擦**（见 `intuition-walkthrough.md` §3；F1/F2 已关闭）。
