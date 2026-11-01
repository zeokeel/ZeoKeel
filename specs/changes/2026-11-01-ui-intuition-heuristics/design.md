# 2026-11-01-ui-intuition-heuristics · 技术设计

> 规范细节链接 `docs/`，不在这里复制第二份规范。组合/manifest 以 `24` 为准，API 以 `04` 为准。

## 1. 影响面

| 类型 | 变更 |
| --- | --- |
| 包 | 仅 `zk-agent-rig` 提示词常量（无新包/依赖） |
| 契约 | 无 |
| 路由/迁移/生成物 | 无 |
| edition/profile/channel | 不适用（开发/评审规范） |
| 文档 | `docs/27` §2.7/§2.8、`docs/19` §11.1、`docs/11` §8.6、`docs/README`、`AGENTS.md` C20/§8/§13、`KNOWN-LIMITATIONS`、`docs/39` §3.1、评审记录 |
| 测试基础设施 | `scripts/ui-interaction-gate.py` 增加直觉文件存在性检查；两个模板；`new-change.sh` |

## 1.5 复用检查（C17）

| 点 | 现有能力 | 结论 | 归属 |
| --- | --- | --- | --- |
| 交互契约/证据 | `interaction.md`、gate、`docs/11` §8.6 | 扩展（同目录增两文件） | docs/spec |
| 唯一交互模式 | `docs/27` §2.5 | 复用（INT 只加原则/方法） | docs |
| 模板生成 | `scripts/new-change.sh`、`specs/templates/` | 扩展 | 脚本 |
| 评审流程 | C19、`docs/reviews/`、`review-check.py` | 复用 | docs |
| 提示词 | `agent-rig::default_preamble`、`docs/39` | 扩展 | agent-rig |

- 搜索证据：`rg -n "intuition|INT-|cognitive|walkthrough|usability|heuristic" docs specs AGENTS.md scripts`（0 命中）。
- 新增可复用能力：无 `@capability`（docs/测试基础设施）。

## 2. 方案

### 2.1 文件布局

```text
specs/templates/intuition.md               # INT 判定表模板
specs/templates/intuition-walkthrough.md   # 3 persona × 3 task × 4 问模板
specs/changes/<id>/intuition.md            # 每个 UI 变更（active 必填）
specs/changes/<id>/intuition-walkthrough.md
scripts/ui-interaction-gate.py             # 增加存在性检查（不评质量）
docs/27 §2.7/§2.8                          # 原则 + 走查/用户测试 SSOT
docs/19 §11.1                              # 契约必填文件/ID/证据
docs/11 §8.6                               # 证据/门禁口径
AGENTS.md C20 + §8 + §13                   # 每次任务可见的硬约束
agent-rig::default_preamble + docs/39 §3.1 # 提示词层
.agents/skills/zeokeel-ui-interaction       # 步骤/命令/判据
```

### 2.2 门禁算法（增量）

1. 现有 `interaction.md` 解析/active/planned 判定不变。
2. 对每个 enforced 契约：
   - 读同目录 `intuition.md`；按行首 `| INT-NN |` 统计；<3 → error `intuition-too-few`；文件缺失 → `missing-intuition`。
   - 读同目录 `intuition-walkthrough.md`；缺失或 <64 bytes → `missing-walkthrough`。
3. 输出 `intuition` 计数；不解析 INT 行的 P0/P1（不由脚本评质量）。
4. `--self-test` 增加：缺 intuition.md、INT 行不足、缺 walkthrough 三类。
5. `--strict` 与 proposal-status 模式共用上述检查；planned 契约跳过。

### 2.3 模板/脚本

- `intuition.md` 列：`ID | 原则 | surface | 场景/任务 | 用户直觉期望（可观察） | 反例 | 验证方式 | 证据`。
- `intuition-walkthrough.md` 结构：persona 表、top task 表、每步 4 问、摩擦表（INT/严重度/修法）、用户测试计划与结果（未跑写 UNVERIFIED）。
- `new-change.sh`：复制 `intuition.md`、`intuition-walkthrough.md`（存在模板时），替换 `<change-id>`；仅在新建 change 目录时生成（目录已存在则脚本退出）。

### 2.4 提示词/可见性

- `AGENTS.md`：C20 一行 + §8 一条 + §13 工作流“先走查”步骤；这是每轮 workspace instructions 必带的。
- skill description：加入 “cognitive walkthrough / INT / intuition-walkthrough / user test”，使 skill catalog 每次可见。
- `agent-rig::default_preamble`：在既有 UI 交互段后追加“new UI feature: cognitive walkthrough (3 personas × top tasks) + intuition.md/INT + intuition-walkthrough.md before code; e2e cannot prove intuition; no 'intuitive' claim without walkthrough/review/user test”。`docs/39` §3.1 逐字记录，单测锁定 `cognitive walkthrough`/`intuition.md`/`user test` 关键词。
- 不在 `tool_guidance` 重复（避免第二份提示词）。

### 2.5 用户测试

- 脚本模板：5 用户 × 3 top task，think-aloud；记录成功率/首次点击/时间/错误/SEQ/SUS；匿名化。
- 证据：`target/evidence/<change>/usability/{script.md,notes/*.md,scores.json}`（target gitignore）；发布验收引用路径；未执行写 UNVERIFIED。
- 不把用户测试接入 `pnpm e2e:ui`/CI（需要真人，且不可复现为单测）。

## 3. 备选与取舍

| 方案 | 优点 | 缺点 | 结论 |
| --- | --- | --- | --- |
| A：SSOT 判定表 + 走查 + 用户测试 + 存在性 gate | 可执行、可评审、诚实 | 质量仍靠人 | **采用** |
| B：把 INT 也塞进 `@interaction` 标签门禁 | 看似全自动 | 把主观判断伪装成断言，易假绿 | 放弃 |
| C：只写原则文档，无契约/模板/门禁 | 成本低 | AI 不会执行，回到“靠自觉” | 放弃 |
| D：引 SaaS 可用性平台 | 现成 | 隐私/网络/依赖，非本地门禁 | 放弃 |

## 4. 安全与隐私

- 用户测试数据匿名化；不记录密钥/客户信息；证据目录按仓库策略（target gitignore）。
- 不改变弱管控/客户端权限语义；不新增网络/依赖。

## 5. 发布与回滚

- 不涉及发布/灰度/迁移。
- 回滚：删除新增 docs/模板/基线文件，还原 AGENTS/docs/skill/prompt/gate/new-change.sh；无数据影响。

## 6. 测试计划

| 层 | 用例 | 命令 |
| --- | --- | --- |
| 文档 | 结构/链接/SSOT | `python3 scripts/docs-check.py`、`docs-command-check.py` |
| 规范 | 模板/spec 结构 | `python3 scripts/spec-check.py`、`review-check.py` |
| 门禁 | 存在性 + self-test | `python3 scripts/ui-interaction-gate.py --self-test && python3 scripts/ui-interaction-gate.py --strict` |
| 提示词 | preamble 单测 | `cargo test -p zk-agent-rig` |
| 行为回归 | 交互套件不回归 | `pnpm e2e:ui`（19 passed） |
| 脚本 | 新变更生成三个模板 | `bash scripts/new-change.sh <tmp>`（临时删除） |

## 7. 文档更新

- [x] `docs/27` §2.7/§2.8
- [x] `docs/19` §11.1
- [x] `docs/11` §8.6/§9.2
- [x] `AGENTS.md` C20/§8/§13
- [x] `docs/README` 阅读路径
- [x] `KNOWN-LIMITATIONS` 直觉验证边界
- [x] `docs/39` §3.1
- [x] 评审记录 `docs/reviews/2026-11-01-ui-intuition-heuristics.md`

## 8. 风险

| 风险 | 概率 | 影响 | 缓解 | 验证 |
| --- | --- | --- | --- | --- |
| INT 表流于形式 | 中 | 中 | 反例 + 验证方式 + 评审走查 + 用户测试 | 评审记录/用户测试 |
| 门禁只查存在性被误当质量门禁 | 中 | 中 | docs/11 明确“存在性 ≠ 质量”；reviewer 判定 | 评审 |
| 用户测试未执行 | 高 | 中 | 模板/脚本；acceptance UNVERIFIED；发布前强制 | acceptance |
| C20 增加与既有体系冲突 | 低 | 低 | 追加编号；不改旧 C1–C19 | docs-check/评审 |
| 在途 spec 未补文件 | 中 | 中 | planned 只 WARN；实现时改 active | gate |
