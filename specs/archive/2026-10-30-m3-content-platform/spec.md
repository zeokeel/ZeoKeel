# 2026-10-30-m3-content-platform · 规格（M3 总切片）

- 状态：`approved`
- 依据：`proposal.md`、`design.md`；`docs/12` M3；`docs/07` 内容模型；`docs/06` 存储；`docs/21` 安全/审计；ADR-061
- 本 change 覆盖 M3 全部退出条件；实现按 tasks.md 分 3 个可独立提交/验证的切片（技能服务端、连接器、客户端安装），不另建绕过 M3 验收的子 spec。

## 1. 数据模型（权威：docs/07 §content）

- `contents(id, kind, name, owner_id, status, current_release_id, visibility, force_update, min_client_version, created_at, updated_at, deleted_at)`
  - kind ∈ `skill` | `app`（M3 只用 skill；app 留 M4；连接器是独立聚合，见 §7）
  - status ∈ `pending` | `approved` | `rejected` | `delisted`
  - visibility ∈ `all` | `selected`
- `content_releases(id, content_id, version, sha256, archive_size, archive bytea, changelog, created_by, approved_by, created_at)`
  - `sha256` = **zip 归档哈希**（ADR-061；服务端不存/不计算树哈希）
- `content_grants(content_id, grantee, can_install)`：`grantee` = `all` 或 user_id；`all` 表示全员可见；`can_install` 区分“可见/可安装”（docs/07）。`visibility=all` 时必须存在 `grantee=all` grant。
- `content_deployments(id, content_id, release_id, action, actor_id, reason, created_at)`：publish/rollback/delist/delete/force_update
- `content_installs(user_id, content_id, release_id, tree_sha256, installed_at, source)`：客户端回执，仅运营事实；唯一键 `(user_id, content_id, release_id)`。
- 归档 **存 PG BYTEA**（docs/06），不引入 skills 专用文件目录。

## 2. 服务端 API（docs/04 §2.4/§2.5；管理台前缀 `/api/admin`）

管理（skill）：
- `GET /api/admin/skills`
- `POST /api/admin/skills`（JSON 元数据；manifest 校验）
- `POST /api/admin/skills/{id}/versions`（multipart zip，≤16MB；服务端只审归档元数据并算归档 sha256）
- `POST /api/admin/skills/{id}/approve|reject|delist|rollback|transfer|force-update`
- `PUT /api/admin/skills/{id}/grants`
- `DELETE /api/admin/skills/{id}`（软删；写 deployment `delete`，客户端 tombstone 自动移除）

管理（connector，独立聚合）：
- `GET/POST /api/admin/connectors`
- `POST /api/admin/connectors/{id}/grants`
- `POST /api/admin/connectors/{id}/oauth`（client_id/scopes/redirect_uri，公共客户端）
- `POST /api/admin/connectors/{id}/delist|rollback|force-update`

客户端（skill，docs/04）：
- `GET /api/client/v1/skills`：approved ∧ 上架 ∧ 授权；items `{name,version,title,category,sha256,min_client_version,enforced,can_install}`；不返回 archive
- `GET /api/client/v1/skills/{name}/{version}/archive`：zip 下载；`X-ZeoKeel-Skill-Sha256` = **归档哈希**（ADR-061）
- `POST /api/client/v1/skills/{name}/{version}/install`：回执 `{release_id, tree_sha256, source}`；服务端校验 `can_install` 后记录
- `POST /api/client/v1/skills/{id}/delete-ack`：客户端确认已移除 tombstone（可选，运营）

客户端（connector，docs/04）：
- `GET /api/client/v1/connectors`：定义 + OAuth 公共客户端元数据（不含 secret/token）
- `POST /api/client/v1/connectors/{id}/{enable,disable}`：员工启停（记偏好）

错误码（封闭）：`SKILL_NOT_FOUND`、`SKILL_FORBIDDEN`、`SKILL_NOT_APPROVED`、`SKILL_ARCHIVE_TOO_LARGE`、`SKILL_ZIP_INVALID`、`SKILL_ENTRY_FORBIDDEN`、`SKILL_MD_MISSING`、`CONFLICT`、`VALIDATION`、`PENDING_LIMIT`、`QUOTA_EXCEEDED`、`CONNECTOR_UNAVAILABLE`。

## 3. 权限矩阵 / 权限点

权限点：`skills:read`、`skills:publish`、`skills:admin`、`connectors:use`、`connectors:admin`。

| 操作 | owner | 其他员工 | 管理员 | 审计员 |
| --- | --- | --- | --- | --- |
| 上传新 skill | ✅ `skills:publish` | ✅ `skills:publish` | ✅ | ❌ |
| 新版本 | ✅ | ❌ | ✅ | ❌ |
| 审批/拒绝/下架/转交/grants/delete | ❌ | ❌ | ✅ `skills:admin` | ❌ |
| 客户端可见/下载 | 按 `can_install` + `skills:read` | 同左 | ✅ | 只读列表 |
| 安装回执 | `can_install` | `can_install` | ✅ | 只读 |
| 连接器启停 | 按 grants + `connectors:use` | 同左 | ✅ | 只读 |
| 连接器管理/grants/oauth | ❌ | ❌ | ✅ `connectors:admin` | ❌ |
| 审计查询 | 本人 | 本人 | ✅ | ✅ |

管理台页面必须声明 `permission`（`skills:admin` / `connectors:admin`）；服务端 API 强制，前端只做显示。转交后 owner 变更；非 owner 不能建新版本；审计失败 fail-closed。

## 4. 上传校验（服务端只审归档元数据）与客户端整树哈希

服务端（不写盘、不解压）：
- multipart 原始 zip ≤16MB；条目数 ≤256；条目名不得绝对路径、`..`、NUL、Windows 盘符/反斜杠穿越、重复规范化路径、非 UTF-8；拒绝 symlink/hardlink/device/FIFO 类型；拒绝声明解压总量 >64MB 或条目数超限；必须存在 `SKILL.md`（根或唯一顶层目录内）。
- 计算并入库 zip 归档 sha256；不计算 tree sha256。
- 解压/路径/链接/总量/临时目录/整树哈希全部由客户端安装器执行（ADR-061）。

客户端（安装器）：
- 下载 zip → 校验归档 sha256（响应头/目录）→ 解压到随机 0700 临时目录。
- 拒绝绝对路径/`..`/NUL/Windows 路径/重复名/symlink/hardlink/device/FIFO/非 UTF-8 名；解压总量 ≤64MB；文件数 ≤256；可执行位只允许白名单扩展名。
- 计算整树哈希 `tree_sha256`：
  - 遍历解压目录，排除 `.DS_Store`/`__MACOSX`；
  - 相对路径统一 `/`，按 UTF-8 字节升序；
  - 每个文件：`path NUL size NUL content` 依次喂入 SHA-256；目录只参与路径序列；权限/可执行位不入哈希；
  - 失败/非法路径 → 拒绝并清理临时目录。
- 安装后重算 tree_sha256 写入 `installed.json`/回执；服务端不据此授权。

## 5. 生命周期 / 强制更新 / 配额

- 状态机：`pending → approved`；`pending → rejected`；`approved → delisted`；`delisted → approved`（重新上架需新 release 审批）；`* → deleted`（软删 + deployment `delete`）。
- 删除：`DELETE /api/admin/skills/{id}` → `deleted_at`；bootstrap 目录返回 tombstone；客户端移除本地内容目录并回执。
- 强制更新：`force_update=true` + `min_client_version`；客户端版本 < min 禁用旧版并提示；否则按版本/归档 sha 更新；冲突保留旧版并提示。
- 回滚：`rollback` 指向历史 release；current_release 更新并写 deployment。
- 配额：每用户 pending/approved skill ≤50；单用户总归档 ≤5GB（docs/07 默认）；超限 `QUOTA_EXCEEDED`。
- 审批拒绝后可编辑元数据重新提交新 release；rejected release 不复用。

## 6. 审计

写 kit 哈希链：`skill.upload`、`skill.release.create`、`skill.approve`、`skill.reject`、`skill.transfer`、`skill.grants.update`、`skill.delist`、`skill.rollback`、`skill.force_update`、`skill.delete`、`skill.download`、`skill.install`。
每条含 actor、target、request_id、ip、device、before/after、result；安装回执 `source=client` 仅运营事实，合规审计以服务端下载/审批为准。保留 ≥180 天。

## 7. 连接器（独立聚合，docs/07 §6）

- 表：`connectors(id,name,transport(stdio|http|sse),config jsonb,enabled,updated_at)`；M3 只接受无静态 secret 配置（ADR-060），敏感字段拒绝；`connector_grants(connector_id,grantee(user|all),can_enable)`；`connector_oauth(connector_id,client_id,scopes,redirect_uri)`。
- 契约 `zk-contract-connector`；服务端 `zk-srv-connectors`（`srv.connectors`）。
- 管理员下发；客户端目录 `{id,name,transport,auth_mode,scopes,enabled}`，不含 secret/token。
- OAuth 授权码/PKCE/刷新全部客户端；服务端不代理 MCP；静态 secret 延后 M3.1（ADR-060）。

## 8. 客户端安装（M3 切片 3）

- bootstrap/目录 → 下载 zip → 归档 sha256 → 解压安全校验/整树哈希（§4）→ 原子 rename 到 `~/.zeokeel/skills/{name}/{version}/`；失败清理。
- 冲突：已装版本 ≠ 目录版本时，force → 覆盖并提示；否则保留旧版提示确认。
- 删除 tombstone → 移除内容目录，保留 installed.json 回执；来源展示（server/local）。
- 个人版本地技能：`~/.zeokeel/skills/local/`，不上传服务端；全局技能不自动继承 workspace（C18）。
- OAuth 刷新/过期/失败：客户端 token 过期前刷新；失败标记 connector 不可用并提示重新授权；测试覆盖。

## 9. C10 运行时校验

- 上传元数据：Rust DTO + JSON Schema 生成 `shared/contracts/schema/skill-*.json`；管理台 Ajv 校验。
- zip 是二进制外部输入：服务端按 §4 做归档元数据校验；客户端解压时按 §4 校验并重算 tree sha256。
- 目录 API 响应：TS 侧生成类型 + runtime guard；禁止 `as` 断言代替校验。

## 10. C17 复用

- 契约：`zk-contract-content`、`zk-contract-skill`、`zk-contract-connector`。
- 服务端：`zk-srv-content`（内容内核）、`zk-srv-skills`（技能归档/目录/下载）、`zk-srv-connectors`；共享 `zk-kit-server` auth/audit/guard。
- 哈希算法单点：`crates/zk-kit-content`（Rust：server 归档校验/客户端测试 golden；TS 客户端实现同算法并用 golden 校验），不复制到各包。
- UI：`@zeokeel/admin-ui-kit` + `@zeokeel/ui-kit`；新增 `admin.page.skills`/`admin.page.connectors`；客户端 `ui.slot.skills`/`ui.slot.connectors`（docs/16 target，接 registry 前先改 docs/16 + tools/zk slot_binding）。

## 11. 迁移 / 回滚

- 迁移 `0010_content.sql`（`zk db lock` 分配全局编号；content 域）；不改历史迁移。
- 回滚：应用层 rollback release；DB 只前向修复；备份/恢复用 M2 脚本。
- 删除软删保留归档供审计/回滚；物理清理由 retention job 另批。

## 12. 验收标准（对应 docs/12 M3）

1. 员工上传 zip（含 SKILL.md）→ 管理员审批 → 指定用户可见/可安装；curl 断言。
2. 新版本 + force_update → 客户端目录更新；归档 sha256 不匹配拒绝安装。
3. 删除 → bootstrap tombstone → 客户端自动移除。
4. 非 owner 不能建新版本；转交后新 owner 可以。
5. 服务端恶意归档元数据拒绝；客户端恶意 zip（路径穿越/symlink/超量/重复名）拒绝并清理。
6. 连接器目录不含 secret/token；员工启停成功；OAuth 刷新/过期测试通过。
7. 管理台技能页真人操作 + axe 0 serious/critical；客户端安装/冲突六态测试。
8. 审计可查上传/审批/安装/删除链路。
9. 配额超限返回 `QUOTA_EXCEEDED`。
10. `review-check`/`docs-check`/`spec-check` 绿。
