# 2026-11-01-client-composer-surface · 验收与证据

> 本 spec 已通过 C19 独立评审（复评 APPROVED）：docs/reviews/2026-11-01-client-phase2-specs.md §15。P2-R1…R4 为批准前编辑性修复，见评审记录。

- 验收人：待独立评审 / 实现者回填
- 日期：2026-11-01（预填；实现后回填实际日期）
- PR/commit：待填
- 证据根：`target/evidence/2026-11-01-client-composer-surface/`（日志 `logs/`、截图 `screenshots/`、`axe.json`、`golden/`）

> 状态：`draft`（未开工）。实现完成后按任务表逐项填证据路径；本文件不预写「已通过」。

## 1. 验收命令与结果

| 命令 | 结果 | 证据路径 |
| --- | --- | --- |
| `cargo test -p zk-kit-tools`（`safe_join_under` symlink/../绝对/越界） | 待跑 | `.../logs/kit-tests.txt` |
| `cargo test -p zk-contract-ipc -p zk-desktop` | 待跑 | `.../logs/contract-desktop-tests.txt` |
| `zk compose --edition personal --profile default --channel official --check`（生成 facade） | 待跑 | `.../logs/compose-check.txt` |
| `cargo test --workspace export_schema && git diff --exit-code` | 待跑 | `.../logs/export-schema.txt` |
| `bash scripts/schema-check.sh` | 待跑 | `.../logs/schema-check.txt` |
| `zk compose --edition personal --profile default --channel official --check` | 待跑 | `.../logs/compose-check.txt` |
| `zk capability --check` | 待跑 | `.../logs/capability-check.txt` |
| `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build` | 待跑 | `.../logs/pnpm-gates.txt` |
| `bash scripts/i18n-check.sh` | 待跑 | `.../logs/i18n-check.txt` |
| `pnpm e2e`（m1-serve 用例含 `/`、`@`、附件、刷新恢复） | 待跑 | `.../logs/pnpm-e2e.txt`、`.../screenshots/`、`.../axe.json` |
| `python3 scripts/spec-check.py && python3 scripts/docs-check.py` | 待跑 | `.../logs/spec-gates.txt` |
| `cargo check --workspace --all-targets` | 待跑 | `.../logs/workspace-check.txt` |
| **待工具落地**：`pnpm e2e:ui --grep composer`（本 spec `interaction.md` 改 `active` 后；依赖 `2026-11-01-ui-interaction-verification`，当前 UNVERIFIED） | 待跑/UNVERIFIED | `.../interaction/<ID>.json` |
| **待工具落地**：`python3 scripts/ui-interaction-gate.py`（同上；当前脚本不存在） | 待跑/UNVERIFIED | `.../logs/interaction-gate.txt` |

## 交互验收（ID → 测试 → 证据）

> 契约见本目录 `interaction.md`（`planned`；实现开始时改 `active`）。P0/P1 必须有 `@interaction:<ID>` 测试与证据
> `target/evidence/2026-11-01-client-composer-surface/interaction/<ID>.json`；口径见 `docs/11` §8.6、`docs/19` §11。

| ID | 计划测试落点（`e2e/interaction/composer-phase2a.spec.ts`） | 证据路径（待实现） |
| --- | --- | --- |
| CMP2-CMD-01 | `/` 打开/过滤/↑↓/Enter 执行并回输入框 | `target/evidence/2026-11-01-client-composer-surface/interaction/CMP2-CMD-01.json` |
| CMP2-CMD-02 | Esc 保留草稿、disabled 只读、`/cancel`/`/retry` 门控 | `.../CMP2-CMD-02.json` |
| CMP2-AT-01 | `@` 文件菜单/过滤/kind 标注 | `.../CMP2-AT-01.json` |
| CMP2-AT-02 | 引用 chip 原子添加/去重/≤5 | `.../CMP2-AT-02.json` |
| CMP2-AT-03 | 文件列表空/加载/错误/离线不阻塞输入 | `.../CMP2-AT-03.json` |
| CMP2-ATT-01 | 拖放/选择上传、chip、缩略图、≤5 | `.../CMP2-ATT-01.json` |
| CMP2-ATT-02 | 类型/大小/数量/空文件/失败错误与重试 | `.../CMP2-ATT-02.json` |
| CMP2-ATT-03 | 附件落 `.zeokeel/attachments/<conv>/` 且刷新恢复 | `.../CMP2-ATT-03.json` |
| CMP2-INFO-01 | 模型标签三种来源 + 点击设置入口 | `.../CMP2-INFO-01.json` |
| CMP2-INFO-02 | usage 本轮/累计、无 usage 文案、刷新一致 | `.../CMP2-INFO-02.json` |
| CMP2-KEY-01 | Enter/Shift+Enter/Esc 与焦点 | `.../CMP2-KEY-01.json` |
| CMP2-IME-01 | composing 不发送/不开菜单、词首触发 | `.../CMP2-IME-01.json` |
| CMP2-FILE-01 | 引用块 compose/发送/chip 渲染/刷新恢复 | `.../CMP2-FILE-01.json` |
| CMP2-A11Y-01 | axe 0 serious + 菜单 aria + 键盘遍历 | `.../CMP2-A11Y-01.json` |
| CMP2-VIS-01 | composer + `/` 菜单视觉基线（人工确认） | `.../CMP2-VIS-01.json` |

## 2. 手工检查

- [ ] `/`：打开/过滤/↑↓/Enter/Esc/disabled/执行后清空触发文本；⌘K 与 `/` 命令集一致
- [ ] `@`：过滤/加 chip/去重/上限/空态/错误态；引用块发送后 chip 渲染、刷新后恢复
- [ ] 附件：拖放+文件选择；类型/大小/数量上限；失败保留草稿与重试；图片缩略图为 **FileReader `data:` URL**（非 blob，Tauri CSP 可显示）；文件落在 `.zeokeel/attachments/<conv>/` 且父目录 symlink 逃逸被拒（P0-1/P1-1）
- [ ] 模型标签：personal provider.model / enterprise gateway model / 未配置态；点击打开设置
- [ ] 用量：流式中更新、完成后一致、无 usage 显示「用量未报告」、不显示伪造百分比
- [ ] 键盘/IME：Enter/Shift+Enter/Esc/↑↓；composing 期间不触发；焦点回输入框
- [ ] 六态：空/加载/错误/离线/流式中/已取消不白屏；离线发送禁用
- [ ] axe 0 serious/critical；无硬编码中英文（i18n 门禁）
- [ ] 兼容：无引用消息行为不变；删除附件文件后 chip 降级提示、不白屏

## 3. 文档与决策

- [ ] `docs/19` §2：输入行新增状态与附件口径
- [ ] `docs/26` §5：`workspace.attachment_save` 命令 + `attachment/*` 错误码登记（SSOT，P1-2）
- [ ] `KNOWN-LIMITATIONS.md`：附件 ≤1 MiB/数量/位置/不直传多模态/孤儿文件无 GC/chip 非内联原子 token/serve body limit 口径
- [ ] proposal §7 D1–D6 拍板记录（Lead/用户结论）已回填
- [ ] 无新增 ADR/D；无新增 capability 条目；`docs/README.md` 不适用

## 4. 回滚

- 回滚步骤：回退 commit；删除 `<workspace>/.zeokeel/attachments/`（可选，孤儿文件不影响正确性）
- 数据影响：无数据库迁移；已发送消息的引用块是普通文本，回滚后按文本显示（可读）
- 已验证回滚：待实现后填（字段/命令均为 additive，旧客户端不调用新命令）

## 5. 剩余风险 / 后续

| 项 | 影响 | 后续 |
| --- | --- | --- |
| 附件上限 1 MiB（受 serve `/ipc` 2 MiB JSON body 限制） | 中 | 后续 spec 提升 body limit 到 8 MiB + 5 MiB 附件（design §2.2 说明） |
| chip 显示在输入框上方、非内联原子 token | 低 | Phase 3 评估 Lexical/结构化 composer（design §2.7 备选 B） |
| 图片不直传模型多模态（靠 `read_image` 工具） | 中 | provider/llm 契约支持后另立 spec |
| 附件无 GC/去重 | 低 | Phase 3 会话清理/附件管理 |
| 原生 textarea 自管 IME/键盘状态机 | 中 | 单测（composition/Enter/Esc/↑↓）+ e2e 键盘路径；不实现富文本/撤销栈 |
| `@` 候选受 `workspace.files` 有界列表（depth≤2/≤300）限制 | 中 | 深目录引用需扩展 listing（side-panels spec 的懒加载树可部分缓解） |

## 5.5 评审条目关闭（2026-11-01 Phase 2 评审 §7）

| 条目 | 处理 | 证据/验证 |
| --- | --- | --- |
| P0-1 附件父目录 symlink 逃逸 | 新增单一 `zk_kit_tools::safe_join_under`（逐组件 no-follow、canonicalize 父、starts_with(root)、拒绝绝对/`..`）；host 经 `zk_composed` 生成 facade 调用，禁止自行拼路径 | `cargo test -p zk-kit-tools -p zk-desktop`（父目录/attachments/文件名 symlink、`..`、绝对、越界不写盘） |
| P1-1 缩略图 blob CSP | 改用 `FileReader.readAsDataURL`（`data:`，CSP 已允许）；不使用 `URL.createObjectURL`/不加 `blob:` | UI 单测断言 data URL；e2e 缩略图渲染 |
| P1-2 IPC/错误码 SSOT | composer task 0 增 `docs/26` §5 登记 `workspace.attachment_save` + `attachment/*`；预算 16（Lead 例外） | `rg` 对照 contract 命令常量；docs-check |
| P1-5 交互门禁不可执行 | tasks/acceptance 标「依赖 `ui-interaction-verification` 落地（当前 UNVERIFIED）」；不在本 spec 声称已绿 | 实现前确认 `scripts/ui-interaction-gate.py`/`pnpm e2e:ui` 存在 |
| P2-1 命令数量 | 改为「现有 4/7；本 spec 新增 cancel/retry 后 6/9」 | 单测断言 catalog id 集合 |
| P2-2 模型点击语义 | spec/design/interaction 统一为「打开设置并定位模型相关 pane（`provider`/`enterprise`；`openSettings(paneId)` 深链）」 | 代码评审 + e2e 点击后选中 pane 断言 |
| P2-3 引用块路径编码 | 拒绝控制字符/换行；仅尾部块；`@` 开头写 `@@path`；parser 跳过非法行；单测 | UI 单测（composer.test） |
| P2-4 共享文件 owner | tasks §5 增 `e2e/m1-serve.spec.ts` 与 docs/19/26/KNOWN owner（composer 先写，side 后追加） | tasks §5 |
| P2-11 @generated 模板 | 改为修改生成器源 `tools/zk/src/main.rs` 生成 facade；不手改模板/生成物 | `zk compose --check` |
| P2-12 app 直接依赖 tool | 前端 app 只经 host IPC；host 经 `zk-composed` 生成 facade，不新增 app→tool 直接依赖 | 代码评审 + compose |
| P2-13/P2-14/P2-15 | 见 activity/side spec 对应关闭；docs/11 基线/门禁由 `ui-interaction-verification` 落地（P1-5） | 复评 |

## 6. 结论

- [ ] `done`：证据齐全，可合并（且复评通过）
- [ ] `abandoned`：原因：待填

## 7. 复用证据（C17）

- 搜索记录：`rg -n "AssistantComposer|composerPlaceholder|commands =|workspace.files|parseFileEntries|AgentUsage|usage" packages apps docs`
- 能力索引复用：`contract.ipc`（`shared/contracts/capability-index.json:43`）、`kit.tools`（`:131`）；**不新增 capability/INDEX 条目**
- 复用清单：`commands`/`runCommand`（App.tsx:591-607，提升为唯一 catalog）、`workspace.files`（IPC + `parseFileEntries`）、
  `AgentUsage`/`usage` 事件/`readUsage`、`UiActions.send/cancel`、`settings.get/set` revision、`ui-commands` 的 listbox 交互模式
- 新增包内实现（不登记 INDEX）：`workspace.attachment_save` IPC + host helper（单一实现，无第二套文件存储）
