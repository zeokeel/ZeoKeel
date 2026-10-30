# 25 · 复用优先与去重门禁

> 目标：**一个能力只实现一次**。写新代码前先找现有实现；找不到才扩展到共享库；重复/复制粘贴在 CI 被硬拦。
> 约束：C17（复用优先/单一实现）；配套 `18`（静态检查）、`11`（T0 门禁）、`24` §3（生成物 writer）、`10`（布局）。
> 本文是"复用、能力索引、去重门禁"的唯一权威；其它文档只引用。

## 1. 问题：AI 最容易犯的四类错

| 错误 | 典型表现 | 后果 |
| --- | --- | --- |
| 不知道已有 | 没搜就新写一个 `formatDuration` / `retry()` / `useDebounce` | 两份行为漂移，一处修一处漏 |
| 图省事放本地 | 在业务包新建 `utils/`、`helpers/`、`common.ts` | 共享能力被私有化，其它包只能再抄一份 |
| 复制近似代码 | 复制旧文件改两行，形成"平行实现" | 同一 bug 要修两处；换包/替换失效 |
| 造并行抽象 | 同一职责两套 hook/两个 trait/两套 DTO | 契约失效、认知负担、测试翻倍 |

**判定**：只要两处代码对同一输入应产生同一行为，就必须只有一处实现；另一处必须调用它。

## 2. 规则（硬约束）

1. **复用顺序（不可跳步）**：现有能力 → 扩展现有（加参数/加实现，保持兼容）→ 提升为共享能力（kit/契约）→ 新建包。跳步必须在 spec 的"复用检查"里写明理由。
2. **能力唯一 owner**：每个可复用能力有唯一 `capability id`（如 `kit.time.format_duration`）与唯一实现位置；禁止第二份实现。
3. **可复用 Rust 代码只放** `crates/zk-kit-*`：无 `zk.toml`、无角色、无状态、无业务逻辑；只依赖契约与其它 kit。业务包可以依赖 kit，kit 不得依赖业务包。
4. **可复用 TS/UI 代码只放** `packages/ui/ui-kit`（角色 `ui.kit`，singleton）：Radix 包装、通用组件、六态容器、格式化；**不放业务组件**。
5. **契约**只放 trait/DTO/错误/常量/符合性套件（`zk-contract-*`），不放实现。
6. **业务包内部 helper 必须私有**（`pub(crate)`/不导出）；业务包之间禁止互相 import（C4）；只允许依赖契约、kit、`@zeokeel/ui-kit` 与 React/Radix 等基线依赖。
7. **禁止新建垃圾桶目录/文件**：`utils/`、`helpers/`、`common/`、`misc/`、`shared.ts`、`lib.ts`（kit 内除外）。需要共享就进 kit，不需要就保持私有。
8. **可复用公共项必须登记**：`crates/zk-kit-*` 与 `zk-contract-*` 的每个 `pub` 项都要有 `@capability <id>` 文档注释与一句用途；未登记 = CI 红。
9. **功能归属唯一**：每个用户可见功能归属一个包；`[[routes]]`/`[[commands]]`/`tool.<name>`/`componentId`/`slotId`/权限点全局唯一，重复由 `zk validate`/`zk compose` 拦。
10. **复制粘贴零容忍**：新增克隆块（≥阈值）即红；历史存量用 ratchet（只降不升）。
11. **例外必须批准**：确需第二份实现的，spec + ADR/D 写明原因，并在能力索引登记为独立 capability（含"为何不能复用"）；到期复核。

## 3. 可发现性（AI 与人都要能搜到）

### 3.1 能力索引（生成物，唯一 writer）

| 产物 | writer | `--check` |
| --- | --- | --- |
| `shared/contracts/capability-index.json` | `zk capability` | 只校验 |
| `docs/generated/CAPABILITY-INDEX.md` | `zk capability` | 只校验 |

条目字段：`capability id`、签名、路径、所属包/crate、`since`、摘要、消费方（反查）。索引从源码注释生成，禁止手改。

### 3.2 搜索命令

```bash
zk find <关键词>          # 按 id/名称/摘要搜能力索引，输出候选 + 路径 + 签名
zk find --used-by <id>    # 谁在用它（反查，避免重复造轮子/误删）
zk capability --check     # 索引与公共 API 一致；kit/contract 公共项未登记 = 红
```

### 3.3 目录约定

- `crates/zk-kit-<domain>/`：共享 Rust 实现库（M0 先建 `zk-kit-core`；后续按 `time`/`fs`/`text`/`http` 等拆，**不建万能 kit**）。
- `packages/ui/ui-kit/`：共享 UI 组件/样式/格式化（角色 `ui.kit`）。
- 新 kit 的准入：**至少 2 个消费方**或明确公共语义；禁止把业务逻辑塞进 kit。

## 4. CI 门禁

### 4.1 T0（每提交，秒级；M0 落地）

| 门禁 | 失败条件 | 实现 |
| --- | --- | --- |
| 能力索引 | `zk capability --check` 漂移；kit/contract 公共项缺 `@capability` | `zk capability` |
| 禁止目录 | 新增 `utils/helpers/common/misc/shared.ts/lib.ts`（kit 外） | `zk lint`（路径规则） |
| import 边界 | 业务包 import 其它业务实现；kit 依赖业务包 | `zk lint`（依赖图；**待实现**，当前仅 `zk validate`/contract 依赖约束） |
| 重复代码（TS） | 新增克隆 ≥30 行且相似度 ≥70% | `jscpd --threshold` + baseline ratchet |
| 重复代码（Rust） | 同上（token 规范化后比对） | `zk lint --duplicates`（已实现：30 行滑窗、token 规范化、共享 shingle 候选 + Jaccard ≥70%；`duplication-baseline.json` 计数 ratchet，CI/check.sh 已接入；见 T20） |
| 唯一性 | 路由/命令/tool/component/slot/权限点重复 | `zk validate` / `zk compose --check` |

### 4.2 T1/T2（PR 必跑）

- 契约 conformance 套件对**所有实现**跑同一批断言（换实现不改调用方）。
- 换包矩阵：替换实现后调用方代码零修改（`03` §6.2）。
- 新增/修改契约：`cargo public-api` diff + 两侧反序列化对拍（`04` §10）。

### 4.3 PR 门禁

PR 模板（`.github/pull_request_template.md`）必须回答：

1. 搜过什么（`zk find` / `docs/generated/CAPABILITY-INDEX.md` / 符号 grep）？
2. 复用了哪些现有能力（id + 路径）？
3. 新增了什么能力、为什么不能复用/扩展？
4. 新增能力登记在哪个索引/kit/契约？

未填 = 不评审。CI 在 scaffold 后接线；在此之前由 reviewer 按本清单执行。

## 5. AI 工作流（强制）

写任何代码前，按顺序执行并留证据：

```text
1) zk find <能力>                    # 或读 docs/generated/CAPABILITY-INDEX.md
2) rg -n "<符号/近似名>" packages apps crates
3) 三选一：
   a. 复用：在 spec/PR 写 "复用 <id> @ <path>"
   b. 扩展：说明向后兼容方式 + 更新的 capability 条目
   c. 新建：spec 写 "为什么不能复用" + 归属（kit/契约/新包）+ 登记索引
4) 禁止复制旧文件后改；只允许先提取到 kit 再引用
5) acceptance/PR 贴证据：搜索输出、索引条目、复用清单
```

Reviewer 三问：**这段逻辑是否已存在？为什么不在 kit？调用方是否只依赖契约/kit？**

## 6. 度量（可控性看板）

| 指标 | 目标 |
| --- | --- |
| 新增克隆块（相对 baseline） | 0 |
| kit 外新增垃圾桶文件 | 0 |
| 未登记 capability 的公共项 | 0 |
| "同一 bug 修两处"事故数 | 0 |
| 重复率（jscpd/Rust clone） | 只降不升（ratchet） |

`duplication-baseline.json` 入库；T0 与 baseline 比较。每里程碑复盘（`12`），异常时先修机制再继续。

## 7. 与既有约束的关系

- **C3（一切皆包）**：kit 是"非包共享实现库"，与 `crates/zk-bootstrap`（ADR-009）同类；它不提供角色、不进 edition，不是业务能力包。
- **C4（只依赖契约）**：业务包仍只依赖契约；kit 是**显式、受控**的共享实现依赖，只有 kit/契约可跨包，且 kit 不得依赖业务包。
- **C17**：本文件是其执行细则；违反 = 拒绝合并。
- **生成物**：能力索引属于 `24` §3 的生成物，唯一 writer 是 `zk capability`；禁止手改。
