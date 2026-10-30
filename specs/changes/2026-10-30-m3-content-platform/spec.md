# 2026-10-30-m3-content-platform · 规格（M3 总切片）

- 状态：`approved`
- 依据：`proposal.md`、`design.md`；`docs/12` M3；`docs/07` 内容模型；`docs/06` 存储；`docs/21` 安全/审计
- 本 change 覆盖 M3 全部退出条件；实现按 tasks.md 分 3 个可独立提交/验证的切片（技能服务端、连接器、客户端安装），不另建绕过 M3 验收的子 spec。

## 1. 数据模型（权威：docs/07 §content）

- `contents(id, kind, name, owner_id, status, current_release_id, visibility, created_at, updated_at, deleted_at)`
  - kind ∈ `skill` | `app`（M3 只用 skill；app 留 M4；连接器是独立聚合，见 §7）
  - status ∈ `pending` | `approved` | `rejected` | `delisted`
  - visibility ∈ `all` | `selected`
- `content_releases(id, content_id, version, sha256, archive_size, archive bytea, changelog, created_by, approved_by, created_at)`
  - `sha256` = zip 文件哈希；另存 `tree_sha256`（整树规范化哈希，见 §4）
- `content_grants(content_id, grantee, can_install)`：`grantee` = `all` 或 user_id；`all` 表示全员可见；`can_install` 区分“可见/可安装”（docs/07）。`visibility=all` 时必须存在 `grantee=all` grant。
- `content_deployments(id, content_id, release_id, action, actor_id, created_at)`：publish/rollback/delist/delete/force_update
- `content_installs(user_id, content_id, release_id, tree_sha256, installed_at, source)`：客户端回执，仅运营事实，不作合规审计
- 归档 **存 PG BYTEA**（docs/06），不引入 skills 专用文件目录；容量/备份由 PG 备份覆盖。

## 2. 服务端 API（管理 + 客户端）

管理（skill，content 聚合）：
- `GET /api/admin/contents?kind=skill`
- `POST /api/admin/contents`（JSON 元数据；四件套校验）
- `POST /api/admin/contents/{id}/releases`（multipart zip，≤16MB；服务端算 zip/tree sha256）
- `POST /api/admin/contents/{id}/approve` / `reject` / `delist` / `rollback` / `transfer`
- `PUT /api/admin/contents/{id}/grants`
- `POST /api/admin/contents/{id}/force-update`（min_client_version + 目标 release）
- `GET /api/admin/contents/{id}/releases`

管理（connector，独立聚合）：
- `GET/POST /api/admin/connectors`
- `POST /api/admin/connectors/{id}/grants`
- `POST /api/admin/connectors/{id}/oauth`（client_id/scopes/redirect_uri，公共客户端）
- `POST /api/admin/connectors/{id}/delist|rollback|force-update`
- 权限点：`connectors:admin`（管理）与 `connectors:use`（员工启停）

客户端（skill）：
- `GET /api/client/v1/contents?kind=skill`：仅 approved 且可见；返回 id/name/version/sha256/tree_sha256/force/min_client_version/can_install；不返回 archive
- `GET /api/client/v1/contents/{id}/releases/{version}/archive`：zip 下载，响应头 `X-ZeoKeel-Content-Sha256`（zip）+ `X-ZeoKeel-Content-Tree-Sha256`（整树）
- `POST /api/client/v1/contents/{id}/install`：回执 `{release_id, tree_sha256, source}`；服务端校验 `can_install` 后记录；回执不反向授权

客户端（connector）：
- `GET /api/client/v1/connectors`：可见连接器定义 + OAuth 公共客户端元数据
- `POST /api/client/v1/connectors/{id}/toggle`：员工启停记录（不回传 token/secret）

错误码（封闭）：`CONTENT_NOT_FOUND`、`CONTENT_FORBIDDEN`、`CONTENT_NOT_APPROVED`、`CONTENT_ARCHIVE_TOO_LARGE`、`CONTENT_ZIP_INVALID`、`CONTENT_ENTRY_FORBIDDEN`、`CONTENT_SKILL_MD_MISSING`、`CONTENT_VERSION_EXISTS`、`CONTENT_QUOTA_EXCEEDED`、`CONTENT_CONFLICT`、`CONNECTOR_NOT_FOUND`、`CONNECTOR_FORBIDDEN`、`CONNECTOR_OAUTH_INVALID`。

## 3. 权限矩阵

| 操作 | owner | 其他员工 | 管理员 | 审计员 |
| --- | --- | --- | --- | --- |
| 上传新内容 | ✅ | ✅ | ✅ | ❌ |
| 新版本 | ✅ | ❌ | ✅ | ❌ |
| 审批/拒绝/下架/转交/grants | ❌ | ❌ | ✅ | ❌ |
| 客户端可见/下载 | 按 `can_install` | 按 `can_install` | ✅ | 只读列表 |
| 安装回执 | `can_install` | `can_install` | ✅ | 只读 |
| 连接器启停 | 按 grants | 按 grants | ✅ | 只读 |
| 连接器管理/grants/oauth | ❌ | ❌ | ✅（`connectors:admin`） | ❌ |
| 审计查询 | 本人 | 本人 | ✅ | ✅ |

转交后 owner 变更；非 owner 不能建新版本。管理员操作写审计，审计失败 fail-closed。

## 4. 上传校验与整树哈希

- 大小：multipart 原始 zip ≤16MB；解压总量 ≤64MB；文件数 ≤256；拒绝 symlink/hardlink/device/FIFO、绝对路径、`..`、NUL、Windows 盘符/反斜杠穿越、重复规范化路径、非 UTF-8 名称。
- 必须包含 `SKILL.md`（skill）或 `CONNECTOR.md`（connector），位置为根或唯一顶层目录内。
- 解压到临时目录（随机名，仅 0700），校验后删除；任何失败清理临时目录。
- 整树哈希 `tree_sha256`：
  - 遍历解压目录，排除 `.DS_Store`/`__MACOSX`；
  - 相对路径统一 `/`，按 UTF-8 字节升序；
  - 对每个文件：`path NUL size NUL content` 依次喂入 SHA-256；目录只参与路径序列；权限/可执行位不入哈希；
  - 文件内容读取失败/路径非法 → 拒绝。
- zip 文件 sha256 与 tree sha256 都入库；客户端安装后按同算法重算，必须一致。

## 5. 生命周期 / 强制更新 / 配额

- 状态机：`pending → approved`（管理员）；`pending → rejected`；`approved → delisted`；`delisted → approved`（重新上架需新 release 审批）；`* → deleted`（软删，客户端目录下发删除事件）。
- 删除：服务端置 deleted + deployment `delete`；客户端下次 bootstrap 收到后自动移除本地目录（保留用户数据？只移除技能目录）。
- 强制更新：管理员设置 `force_update=true` + `min_client_version`；客户端版本低于 min → 禁用旧版并提示更新；高于 → 正常更新，冲突保留旧版并提示。
- 回滚：`rollback` 指向历史 release；current_release 更新并写 deployment。
- 配额：每用户 pending/approved skill 总量 ≤100；单用户总归档 ≤512MB；超限 `CONTENT_QUOTA_EXCEEDED`。
- 审批拒绝后可编辑元数据重新提交新 release，不复用 rejected release。

## 6. 审计

写 `audit_log`（沿用 kit 哈希链）：`content.upload`、`content.release.create`、`content.approve`、`content.reject`、`content.transfer`、`content.grants.update`、`content.delist`、`content.rollback`、`content.force_update`、`content.delete`、`content.download`、`content.install`。
- 每条含 actor、target、request_id、ip、device、before/after、result；安装回执只作 `source=client` 的运营事实，合规审计以服务端下载/审批为准。
- 保留 ≥180 天（M2 retention job 已覆盖）。

## 7. 连接器（独立聚合，docs/07 §6）

- 表：
  - `connectors(id, name, transport(stdio|http|sse), config jsonb, enabled, updated_at)`；M3 **只接受无静态 secret 的配置**（command/args/env 名/url），`config` 中的敏感字段一律拒绝；静态 secret 加密存储延后到 M3.1/ADR，需先评审受控交付/轮换/掩码。
  - `connector_grants(connector_id, grantee(user|all), can_enable)`
  - `connector_oauth(connector_id, client_id, scopes, redirect_uri)`；公共客户端 + PKCE，**不下发 client_secret**；OAuth 授权码/令牌只在客户端本地。
- `zk-contract-connector` 承载 connectors/grants/oauth DTO；服务端 `zk-srv-connectors`（`srv.connectors`）。
- 管理员下发；bootstrap 下发 `{id,name,transport,force,oauth:{client_id,scopes,redirect_uri}?}`，不含 token/secret。
- 客户端启停/令牌管理完全本地；服务端不代理 MCP。需要静态 secret 的连接器 M3 不支持，界面明确提示。

## 8. 客户端安装（M3 切片 3）

- bootstrap/contents 目录 → 客户端选择安装；下载 zip → zip sha256 → 解压 → 整树 sha256 → 原子替换目录（先临时目录+rename）。
- 冲突：已装版本 ≠ 目录版本时，若 force → 覆盖并提示；否则保留旧版、提示用户确认。
- 卸载/删除事件：移除内容目录，保留 `installed.json` 回执；来源展示（server/本地）。
- 个人版本地技能：`~/.zeokeel/skills/local/`，不上传服务端；M3 客户端 UI 只读展示。

## 9. C10 运行时校验

- 上传元数据：Rust DTO + JSON Schema 生成到 `shared/contracts/schema/content-*.json`；管理台用 Ajv 校验。
- zip 是二进制外部输入：服务端按 §4 做结构校验；客户端解压后重算 tree sha256 并校验。
- 目录 API 响应：TS 侧用生成类型 + 运行时 guard；禁止 `as` 断言代替校验。

## 10. C17 复用

- 契约：`zk-contract-content`（contents/releases/grants/deployments）、`zk-contract-skill`（客户端技能包/安装回执）、`zk-contract-connector`（connectors/grants/oauth）。
- 服务端：`zk-srv-content`（内容内核）、`zk-srv-skills`（技能校验/目录/下载）、`zk-srv-connectors`（连接器聚合）；共享 `zk-kit-server` auth/audit/guard。
- UI：复用 `@zeokeel/admin-ui-kit` + `@zeokeel/ui-kit`；新增 `admin.page.skills` / `admin.page.connectors` 包；客户端 `ui.slot.skills`/`ui.slot.connectors`（docs/16 target 名单）。
- zip/tree 哈希只实现一次（`zk-srv-content` 或 `crates/zk-kit-content`）；禁止复制到 connector/client。

## 11. 迁移 / 回滚

- 新迁移 `0001_content.sql`（content 域）；不改历史迁移。
- 回滚：应用层 rollback release；DB 回滚只前向修复；备份/恢复用 M2 脚本。
- 删除内容软删，保留归档供审计/回滚；物理清理由 retention job 另批。

## 12. 验收标准（对应 docs/12 M3）

1. 员工上传 zip（含 SKILL.md）→ 管理员审批 → 指定用户可见 → 客户端安装；curl 脚本断言。
2. 新版本 + force_update → 客户端目录更新；tree_sha256 不匹配拒绝安装。
3. 删除 → bootstrap 不再下发 → 客户端自动移除。
4. 非 owner 不能建新版本；转交后新 owner 可以。
5. 恶意 zip（路径穿越/symlink/超量/重复名）全部拒绝，临时目录清理。
6. 连接器目录不含 secret/token；员工启停成功。
7. 管理台技能页真人操作 + axe 0 serious/critical；客户端安装/冲突六态测试。
8. 审计可查上传/审批/安装/删除链路。
9. 配额超限返回 `CONTENT_QUOTA_EXCEEDED`。
