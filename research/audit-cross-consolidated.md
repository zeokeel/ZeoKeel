# 第二轮：交叉互审 + SSOT 去重汇总

> 时间：2026-10-30 14:26–14:35。4 个只读子代理：
> [`audit-role-ssot.md`](audit-role-ssot.md)（37 主题 SSOT）、
> [`audit-cross-a.md`](audit-cross-a.md)（架构/客户端互审）、
> [`audit-cross-b.md`](audit-cross-b.md)（服务端/财务/OEM 互审）、
> [`audit-role-readability.md`](audit-role-readability.md)（AI 可读性）。
> 本文件只汇总处置结果。

## 1. 结论

- **SSOT**：37 个主题中 8 个 P0 重复/冲突；同一功能平均 3.1 份"规范段"。
- **交叉互审**：架构/客户端 11 个 P0、服务端/财务/OEM 9 个 P0（多为旧文未回写新规范）。
- **可读性**：冲突少了，但"同一事实有 2–4 份都像权威"未清理；AI 会按相似度命中旧文本。
- 本轮已删除/改写重复规范段，使**每个主题只有一个规范出处**，其余改为链接或摘要。

## 2. 已处置的 P0（文档层）

| 主题 | 处置 |
| --- | --- |
| manifest 两套 | 删除 `02` 的 TOML 块；`24` §2 为唯一 schema，并补 `summary/stability/config/permissions/compat/conformance` |
| 钱包/价格/usage 两套 | 删除 `07` §3 表与流程；`22` §2–§5 唯一；`07` 只留配额性质/准入摘要 |
| overwrite 直改余额 | `07` 公式删除；`22` 要求写 `adjust`；hold 阶段不改余额列 |
| 路由真源三说 | `04/06/03` 统一为 manifest `[[routes]]` 静态真源 + `Register()` 校验（`24` §4/ADR-047） |
| 会话存储 JSONL vs SQLite | `03` 改 `store-session-sqlite`；`24` §7/ADR-053 唯一 |
| exclude 用角色名 | `03` 改包 id；`24` §1 唯一；personal-only 清单唯一 |
| 迁移两套 | `06/10` 改全局序号 + 复合键，`24` §5 唯一 |
| metadata 可降级 | `03` 改 fail-closed；`24` §8 唯一 |
| 基线 profile 写 edition | 删除 profile 的 `edition=`；`24` §1.2 基线元组唯一 |
| 每 app Tauri capability | `19` 改为单一 `app` capability + 窗口 label/origin allowlist（构建期可生成） |
| 关闭窗口=托盘 vs 退出 | `19` 区分"关闭按钮=托盘"与"退出应用=quit" |
| serve 安全四处定义 | `21` §6 唯一；`19` 只留用户可见错误 |
| 登录页可改地址 | `19` 改为"设置中修改，需重登"；`09` §3/ADR-044 唯一 |
| 备份含 master.key | `06/09` 改为"单独离线保管、不入备份包"；`20` §3 唯一 |
| 回滚 vs 客户端 1:1 | 清单加 `previous?`；`04/09/20` 统一"受控降级" |
| 负余额告警 | `20` 改为"低于下限/宽限期才告警"；`22` §4 唯一 |
| RACI "处理者" | `23` 改为"软件提供方，不处理数据"（D63） |
| OEM 换域名数据根 | `23` 明确同渠道迁移 vs 新渠道导出/导入 |
| OEM Windows 签名例外 | 删除；全渠道不签名（ADR-033） |
| 16 预构建包例外 | 删除；OEM 源码入库（D61） |
| 22 应收/上游账单/退款 | 删除；余额=配额，不生成应收/账单 |

## 3. 新增的单一入口

- [`docs/README.md`](../docs/README.md)：唯一索引/SSOT 表/术语表/去重规则/冲突裁决。
- 根 `README.md` 文档地图压缩为 10 秒入口，不再复制 20+ 行目录。
- 17 标为历史存档；15 改为决策记录（D1–D65）；13 保留 ADR 理由。
- `01` 功能清单改为范围 + 链接，不再复制 07/05/09。

## 4. 仍存（P1/P2，下一轮）

1. `09` 仍一事多职（渠道/构建/镜像/更新/合规），建议拆 3 份；当前先靠 SSOT 表约束。
2. `13` 56 条 ADR 无顶部索引；风险表建议外移 `25-risk-register.md`；部分 ADR 仍在复述规范正文。
3. `19` 的 a11y/六态/验收与 `11/18` 仍有摘要级重复；应按 `11` 门禁、`18` lint 配置收敛。
4. `01/07/09` 还有可删的功能清单/摘要重复；`07 §9` 包清单应由 `zk plan` 生成。
5. 生成索引缺失：`docs/generated/{PACKAGE-INDEX,CONTRACT-INDEX,PROFILE-MATRIX,schema}.md`。
6. ~~`docs-check` 未实现~~ → 已实现 `scripts/docs-check.py`，当前 0 error / 0 warning。
7. `Q` 编号仍有 15/04/07/09-Q 四套；04/07 §11 已标"实现细节"，但跨文档引用需带前缀。

## 5. 下一步

- 先落 `docs-check` + 生成索引，再考虑拆 09/13；否则拆完仍会漂移。
- 之后进入 M0 骨架；M0 的退出条件必须引用 `11` 的 T0–T5 与 `24` 的组合/生成物门禁，不再复制。


## 6. 最终门禁（2026-10-30）

- 五轮独立门禁审计：P0/P1 = 18 → 7 → 4 → 1 → **CLEAN（0/0）**。
- `python3 scripts/docs-check.py`：**0 errors / 0 warnings / 29 files**。
- 终审报告：[`audit-final-gate-r5.md`](audit-final-gate-r5.md)；此前：[`r4`](audit-final-gate-r4.md)、[`r3`](audit-final-gate-r3.md)、[`r2`](audit-final-gate-r2.md)、[`r1`](audit-final-gate.md)。
- 说明：CLEAN 是"文档规范层无 P0/P1"，不等于实现/测试完成；代码与证据仍按 `11`/`24` 执行。
