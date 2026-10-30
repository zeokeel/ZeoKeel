# 2026-10-30-m3-content-platform · 提案

- 状态：`approved`
- Owner：lead
- 日期：2026-10-30
- 关联：`docs/12-roadmap.md` M3；`docs/07` §5–§6；`docs/04` §2.4–§2.5；`docs/13` ADR-060/061；`docs/21`
- 评审：docs/reviews/2026-10-30-m3-plan.md
- 评审结论：R6 APPROVED；R7 CHANGES-REQUESTED（已 reopen）
- 变更预算：M3 全量（服务端 content/skills/connectors + 管理台 + 客户端安装 + 证据）；按 tasks 分切片提交

## 1. 背景 / 问题

M1/M2 已能登录、网关对话、扣费、bootstrap 模型目录。M3 要交付“技能与连接器可管理、可分发、可强制更新”，否则企业无法下发内容，个人版也没有技能入口。

## 2. 目标

- 服务端技能中心：zip + SKILL.md 校验（≤16MB）、sha256、上传、审批、归属/转交、可见范围、版本、强制更新、删除。
- 服务端连接器中心：管理员下发 MCP 配置（stdio/http），员工启停；OAuth 令牌只在客户端本地。
- 管理台：技能列表/上传/审批/版本/转交/下架；连接器列表/下发/停用。
- 客户端：bootstrap 下发技能/连接器目录；安装/卸载/更新/冲突展示；个人版本地技能目录。
- 审计：上传/审批/下发/安装/删除链路可查。

## 3. 非目标

- 不做技能扫描/沙箱；技能在客户端执行，安全责任在终端（ADR-030）。
- 不做服务端代理 MCP 流量；连接器 OAuth 回调在客户端。
- WASM 应用中心（M4）。
- 商店计费/付费内容（D 已定不做）。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 员工 | 打开商店/技能页 | 看到全员或指定给自己的技能，安装/更新 |
| 技能作者 | 上传 zip | 校验通过后进入待审 |
| 管理员 | 审批/转交/下架 | 状态和可见范围立即生效 |
| 员工 | 连接器页启停 MCP | 客户端本地保存启停；OAuth 令牌不出客户端 |

## 5. 成功指标

- `scripts/srv-store-test.sh` 覆盖上传→审批→可见→安装→新版本→删除链路。
- `scripts/admin-e2e.sh` 增加技能页面真人操作 + axe 0 serious/critical。
- `pnpm --dir apps/desktop/web test` 覆盖技能/连接器状态与六态。
- 审计表能查到上传/审批/下发动作。

## 6. 风险与未知

| 风险/未知 | 影响 | 缓解/验证 |
| --- | --- | --- |
| zip 校验与压缩炸弹 | 高 | 限制 16MB、条目数、路径穿越、SKILL.md 必填；单测 |
| 技能版本强制更新冲突 | 中 | sha256 整树 + 客户端版本比较；e2e |
| 连接器 secret 泄漏 | 高 | 客户端本地存储；日志脱敏；bootstrap 不含 secret |
| 存储位置 | 中 | 归档存 PG BYTEA（docs/06），沿用 PG 备份/恢复 |

## 7. 需要用户拍板

- [x] 技能包校验：按 docs/12 M3 已定（zip + SKILL.md、16MB、暂不扫描）。
- [x] 连接器：公共客户端 + PKCE，令牌客户端本地（ADR-030）。
- [x] 可见范围：全员/指定用户（ADR-032）。
- [x] 不做付费内容/计费。

## 8. 变更预算与拆分

本 change 覆盖 M3 全量，tasks 内分 3 个可独立验证切片（skills server/admin、connectors、client install），不另建绕过 M3 验收的子 spec。

## 9. 复用与已有实现（C17）

- 已搜索：`zk capability --check`、`docs/generated/CAPABILITY-INDEX.md`、`rg "skills|connectors" packages`。
- 可复用：`zk-srv-http` 迁移执行、`zk-kit-server` auth/audit/admin guard、`admin-ui-kit` API/表单、`ui-kit` 六态组件。
- 本次新增可复用能力：`zk-kit-content` 持有归档元数据/哈希规范化；`srv.content`/`srv.skills`/客户端 golden 复用，不复制。
- 为什么不能复用：现有无 store 域；服务端 admin 页面与客户端 bootstrap 已有 seam，可复用。
