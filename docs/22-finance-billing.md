# 22 · 钱包、计费与对账（补充规范）

> 状态：Plan v3.2 补充规范。补齐 `07` §3 中"能扣费、不能对账/出账/举证"的缺口。
> **已拍板（D60）**：余额 = **企业配额**（非预付资金，不可提现/转让/退款）；不提供收款/开票/税务/坏账功能。
> 本文的“充值/调账”均指管理员分配内部配额，不涉及金钱收付。

## 1. 余额性质（已定：企业配额）

- 钱包是**企业内部用量配额**，不是客户预付资金；不可提现、转让、退款。
- 不提供收款、开票、税务、坏账、资金存管；管理员操作只是“分配/调整配额”。
- 仍然必须：账本只增不改、余额 = Σ账本、并发原子扣减、可对账。
- “充值”一词在代码/接口中可保留，但语义=配额入账，禁止对外宣称可退款/可提现。

## 2. 钱包状态机

```text
active ──余额≤0且非流式──► exhausted（准入拒绝；流式可继续到上限）
active/exhausted ──管理员充值/发放──► active
active ──欠费且超过宽限/停服策略──► suspended（禁止新请求；可登录/充值）
suspended ──结清/管理员恢复──► active
active/suspended ──销户──► closed（余额按规则清零/转移，账本留痕）
```

- 状态存 `wallet_accounts.status`；`currency` 固定（默认 `CNY`）；`version` 乐观锁。
- `suspended` 策略：由管理员配置（默认欠费>月发放额×1 或 30 天）；停服不影响审计/导出。
- 销户：余额清零/转移必须生成 `adjust` 账本 + 审批 + 审计；欠费作为账本事实保留，不生成应收。

## 3. 账本（唯一真源）

| 列 | 说明 |
| --- | --- |
| `id` | 单调 id |
| `user_id` | 归属 |
| `delta_micro` | 正=入账，负=出账 |
| `kind` | grant/credit/debit/adjust/clear/expire |
| `balance_after` | 写入后余额（链式校验） |
| `idempotency_key` | **唯一索引**；重放回放首次结果 |
| `ref_type/ref_id` | usage/充值单/发放批次/审批单 |
| `actor` | 系统/管理员/用户 |
| `request_id` | 链路追踪 |
| `created_at` | UTC |

规则：

- 余额只能经账本变更；**禁止任何包直接 UPDATE 余额列**（ADR-023/046）。
- `overwrite` 模式：不得 `balance = amount`；必须写一条 `adjust`（delta = amount − 旧余额），并记录发放规则。
- 负余额（欠费）保留为账本事实，不得被后续发放静默抹平；冲销需 `adjust` + 审批 + 审计。
- 不变量（CI/对账 job）：`balance = Σ delta`；`balance_after` 逐条可复算；`overwrite` 后差额有对应 adjust。

## 4. 预授权 / Hold（硬阻断的前提）

表 `wallet_holds`：`id, user_id, request_id, amount_micro, status(held/released/settled/expired), created_at, expires_at`。

流程：

1. 准入：`amount = max_output_tokens × 输出价 × 峰谷倍率 + 输入估算`；在**单条 SQL 事务**中插入 `wallet_holds(held)`，并校验 `balance - active_holds >= amount`（行锁/条件插入）；**hold 阶段不改余额列**，余额始终 = Σ账本。
2. 流式：每 N 秒/每段更新 hold 估算；超过单请求上限 → 中止并结算已生成部分。
3. 结束：按实际 usage 结算；写 `debit` 账本 + `hold.settled`（余额由账本派生），释放多余 hold。
4. 超时/崩溃：hold 过期由 job 释放；pending usage 对账。
5. 并发：`SELECT ... FOR UPDATE` 或条件 UPDATE；每用户并发流上限（默认 2），全局上限；超限 429。

负余额下限：默认 `月发放额 × 1`（可配）；流式达到下限立即中止；非流式不允许负余额。

## 5. 计量与结算

### 5.1 usage 字段（必须完整）

`request_id, user_id, model, provider, input_tokens, cached_tokens, output_tokens, reasoning_tokens, tool_tokens?, amount_micro, price_version, period_id, multiplier, status(success/aborted/upstream_failed/estimated), source(upstream/local), at`。

### 5.2 结算状态机

`pending → settled | voided | estimated`：

- 请求开始写 `usage(pending)` + hold；成功结束用上游 usage 结算；缺失用本地计数标 `estimated`。
- 断线/取消/上游 5xx：按已生成 token 结算；无法确定时 `estimated` + 对账复核。
- 重试：同一 `request_id` 只结算一次；首字节后禁止重试/换 provider。
- 崩溃：启动扫 pending，超时后按最后记录结算或 void，差异进对账。

### 5.3 价格版本（不可变）

| 表 | 说明 |
| --- | --- |
| `price_versions` | `version, model, input_per_1k, output_per_1k, cached_per_1k, peak_multiplier, offpeak_multiplier, effective_from, status(draft/active/retired), created_by` |
| `price_periods` | `id, name, weekday_mask, start_min, end_min, tz, scope(model/global)` |
| `usage.price_version` | 结算时快照版本；价格行不可改，改价=新版本 |
| `price_snapshot` | JSON：version、model、period、multiplier、输入/输出/缓存价、舍入规则 |

规则：历史请求只能按快照复算；改价需公告期（默认 7 天）+ 审计；展示价缓存失效；禁止删除旧版本。

### 5.4 舍入

金额单位 micro（1e-6 元）；计算用整数：`tokens × price_per_1k / 1000`，四舍五入到 micro；倍率乘后同样舍入；算法固定并 proptest 覆盖。账单与账本必须逐条一致。

## 6. 对账

| 对账 | 频率 | 差异处置 |
| --- | --- | --- |
| 账本 ↔ usage | 每日 + 实时抽样 | 差异报告；人工确认；补账/冲销需审计 |
| 账本 ↔ 日报 | 每日 | 日报仅派生，可重建 |
| 客户上游账单（可选） | 每月 | 客户自行核对；我方不收款/不持有上游账单 |
| 余额 ↔ Σ账本 | 每小时 | 不一致立即告警，冻结相关账户 |
| hold ↔ pending usage | 每小时 | 泄漏 hold 释放 |

差异报告字段：request_id、用户、金额、原因、处置人、处置结果；保留 ≥12 个月。

## 7. 用量报表（不做账单/发票）

- 月度用量快照（不可变）：按用户 + 汇总；字段：期初、发放、配额调整、消费、期末、明细。
- 导出：CSV（可选 PDF），脱敏；导出动作审计。
- 不做发票/收款/账单；客户内部如需报销，使用导出报表。
- 欠费报表、配额调整流水、价格变更记录、异常用量报表。
- 争议流程：客户申诉 → 对账 → 调整（审计）→ 回复；默认不承诺 SLA，争议流程内部处理（支持分级见 `23` §6，内部目标不构成合同 SLA）。

## 8. 钱审计矩阵（缺一 CI 红）

| 动作 | 审计字段 |
| --- | --- |
| 充值 | actor、金额、原因、幂等键、before/after、request_id |
| 月发放 | 批次、规则版本、人数、总额、模式、失败明细 |
| 调账/清空 | actor、审批单、原因码、证据、before/after |
| 改价/峰谷时段 | 旧值/新值、生效时间、公告、影响面 |
| 销户 | 余额处置、欠费处置、内容转交 |
| 只读查询 | 审计员查询本身留痕（谁查了谁） |

哈希链必选 + 每日 WORM 锚定；审计表 append-only；保留 ≥180 天（配额相关建议 ≥5 年，按内部策略）。

## 9. 风控

- 单用户日消费上限、单请求输出上限、并发流上限；超限 429。
- 异常告警：短时用量激增、负余额、频繁调账、同一 IP 多账号。
- 大额配额调整/改价阈值告警（>月发放额×10）；如需双人复核，须先补审批模型/API（当前不做）。
- 盗号：设备吊销、强制重登、余额冻结；审计追溯。

## 10. 验收（M2/M6）

- [ ] 并发 100 请求不超发；余额始终 = Σ账本；hold 无泄漏。
- [ ] 断线/取消/上游 5xx/崩溃/重试均只结算一次且可对账。
- [ ] 任意历史请求可按 price_snapshot 复算到 micro。
- [ ] 每日对账差异=0 或有人工处置记录；月度账单可导出。
- [ ] 钱审计矩阵全绿；哈希链/WORM 验证通过。
- [ ] 余额=企业配额口径已在 UI/接口/导出中一致（D60）。
