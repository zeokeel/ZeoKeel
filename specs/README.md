# `specs/` — 变更规格（AI 与人的施工合同）

> 目的：**先写清"改什么、为什么、怎么验收"，再让 AI 写代码。**
> `docs/` 是长期规范（SSOT）；`specs/` 是**一次变更**的范围、设计与验收。变更完成后，耐久结论回写 `docs/`/ADR/D，
> 变更目录归档到 `specs/archive/`。

## 1. 硬规则

1. **没有 spec 不开工**：代码/契约/数据/文档行为变更，必须先有 `specs/changes/<id>/`。
2. **一个变更一个目录**：不把两个无关目标塞进同一 spec。
3. **spec 先评审**：`proposal.md` 的目标/非目标/范围经用户确认后，才能进入实现。
4. **任务必须可验收**：`tasks.md` 每一行必须有 `verify`（命令或检查）。
5. **完成 = 证据**：`acceptance.md` 记录命令、结果路径、文档更新与回滚；模型自述不算。
6. **耐久结论回写**：新决策写 `docs/15`（D）或 `docs/13`（ADR）；新规范先改 `docs/README.md` 指定的 SSOT 文档。
7. **不改历史**：`specs/archive/` 只读；已归档 spec 不追改，新变更另建目录。

## 2. 生命周期

```text
propose → specify → design → tasks → implement → verify → archive
  草案      需求      技术方案    任务      小步提交     证据       归档
```

| 状态 | 含义 | 谁改 |
| --- | --- | --- |
| `draft` | 正在写，未评审 | 提案人/AI |
| `approved` | 用户确认目标/范围，可开工 | 用户 |
| `in-progress` | 实现中 | 实现者 |
| `done` | 验收证据齐全，已合并 | 实现者 |
| `abandoned` | 明确不做，记录原因 | 提案人 |

状态写在 `proposal.md` 顶部；`approved` 之前**不得改产品代码**。

## 3. 目录与命名

```text
specs/
├─ README.md
├─ templates/                 # 五件套模板（只读）
├─ changes/
│  └─ 2026-10-30-add-package-foo/
│     ├─ proposal.md
│     ├─ spec.md
│     ├─ design.md
│     ├─ tasks.md
│     └─ acceptance.md
└─ archive/                   # done/abandoned 后移动到这里
```

- `<id>` = `YYYY-MM-DD-<kebab-slug>`，全仓唯一。
- 新建：`bash scripts/new-change.sh add-package-foo`（自动生成 id 与五件套）。
- 小变更可只填必要小节，但**不能删除文件**；大变更五件套必须完整。

## 4. 每个文件写什么

| 文件 | 必须回答 | 不写什么 |
| --- | --- | --- |
| `proposal.md` | 为什么做、目标/非目标、用户场景、成功指标、风险、需拍板项、变更预算 | 具体表结构/API 字段 |
| `spec.md` | 行为需求（WHEN/THEN）、边界/错误、权限、数据变更、兼容性、验收断言 | 实现细节、选型理由 |
| `design.md` | 影响的包/契约/路由/迁移/生成物、方案与备选、安全、发布/回滚、测试计划、文档更新 | 重复 `docs/` 已有规范 |
| `tasks.md` | 可独立提交的任务、写作用域、依赖、验证命令 | 大而全的“完成功能” |
| `acceptance.md` | 验收命令、证据路径、手工检查、文档/ADR、回滚、签字 | 主观“看起来没问题” |

## 5. 与 `docs/` 的边界

- `docs/` 定义**长期规范**；`specs/` 定义**本次变更**。
- 规范冲突时以 `docs/README.md` 的 SSOT 表为准；spec 不得另造一套规范。
- 变更引入新规范/新决策：先改 `docs/`（或先写 ADR），再在 spec 的 `design.md` 链接，不复制正文。
- 变更完成后：`docs-check` 必须 0/0；`specs/` 目录归档；PR 描述链接 spec。

## 6. 门禁（当前如实口径）

- `python3 scripts/spec-check.py`：**只做结构检查**——模板齐全、`changes/<id>/` 五件套存在、`proposal.md` 状态合法、`tasks.md` 有任务表头与验证列。**`specs/changes/` 为空时不报错**；内容质量由评审负责，不由脚本保证。
- `python3 scripts/docs-check.py`：文档结构/链接/SSOT 登记/ADR-D 编号/废弃口径。**不检查"多文档重复短语"**；去重靠 SSOT 评审 + `rg` 手工检索。
- **C16（无 spec 不开工）当前由评审执行**：没有自动门禁拦截"未建 spec 就改代码"；scaffold 后接入 CI（`AGENTS.md` §6）。
- CI 顺序（scaffold 后）：`spec-check` → `docs-check` → 代码/契约/测试门禁（见 `AGENTS.md` §6）。

## 7. 最小示例（只看结构）

```text
specs/changes/2026-10-30-add-tool-fs/
├─ proposal.md   # 为什么需要 tool.fs，非目标：不做 shell
├─ spec.md       # R1 读文件；R2 写文件；错误码；权限
├─ design.md     # packages/client/tool-fs/、角色 tool.fs、契约 zk-contract-tool
├─ tasks.md      # 1. 建包；2. 契约；3. conformance；4. 注册 edition；5. 文档
└─ acceptance.md # cargo nextest、zk validate、docs-check、证据路径
```
