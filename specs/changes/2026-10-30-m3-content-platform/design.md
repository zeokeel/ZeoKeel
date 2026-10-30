# 2026-10-30-m3-content-platform · 设计

> 权威数据模型以 docs/07 `contents/content_releases/content_grants/content_deployments` 为准；本文件只记录实现映射与安全细节，不自造 skills 专用表。

## 1. 包与角色

| 包 | role | 说明 | 四件套 |
| --- | --- | --- | --- |
| `packages/contracts/content` | contract | contents/releases/grants/deployments DTO | zk.toml + conformance + README |
| `packages/contracts/skill` | contract | 客户端技能目录/安装回执 DTO | 同上（conformance） |
| `packages/contracts/connector` | contract | connectors/grants/oauth DTO | 同上（conformance） |
| `packages/server/content` | `srv.content` | 内容内核/状态机/审批/发布/配额/哈希 | zk.toml + contract + conformance + README + migrations |
| `packages/server/connectors` | `srv.connectors` | 连接器定义/grants/oauth 元数据/下发 | 同上 |
| `packages/server/skills` | `srv.skills` | skill/connector zip 校验、目录、下载 | 同上 |
| `packages/admin/admin-skills` | `admin.page.skills` | 技能页面 | zk.toml + contract + conformance + README |
| `packages/admin/admin-connectors` | `admin.page.connectors` | 连接器页面 | 同上 |
| `packages/ui/ui-skills`（后续切片） | `ui.slot.skills` | 客户端技能目录/安装 | 同上 |
| `packages/ui/ui-connectors`（后续切片） | `ui.slot.connectors` | 客户端连接器启停 | 同上 |

- `srv.content` 拥有 contents/releases/grants/deployments 与审批内核；`srv.skills` 只做技能格式校验/目录/下载，不重复状态机；`srv.connectors` 只拥有连接器聚合，不复用 content 状态机。
- 契约唯一：`zk-contract-content`/`zk-contract-skill`/`zk-contract-connector`；admin 页面用 `zk-contract-admin-ui`；UI 用 `zk-contract-ui`。

## 2. DB（`content` 域迁移）

- `contents`：id uuid, kind, name, owner_id, status, current_release_id, visibility, force_update, min_client_version, created_at, updated_at, deleted_at
- `content_releases`：id, content_id, version, zip_sha256, tree_sha256, archive_size, archive bytea, changelog, created_by, approved_by, created_at
- `content_grants`：content_id,grantee(all|user_id),can_install
- `content_deployments`：id,content_id,release_id,action,actor_id,created_at
- `content_installs`：user_id,content_id,release_id,tree_sha256,source,installed_at
- 唯一约束：`(content_id, version)`；`contents.name` 在未删除范围内唯一。

## 3. 上传/下载

- multipart 边界先读 `Content-Length`，≤16MB；临时文件 0600；校验后入库 BYTEA。
- zip 校验见 spec §4；树哈希实现单点：`zk-srv-content` 暴露 `tree_sha256(dir)`，`srv.skills`/客户端测试复用同一算法描述（客户端 TS 不能复用 Rust，需 conformance golden 对照）。
- 下载响应头同时给 zip/tree sha256；不缓存 secret。

## 4. 管理台

- `admin.page.skills`：表格（状态/owner/可见范围/版本/force）、上传表单、审批/拒绝、转交、下架/恢复、历史版本、回滚、grants 编辑。
- `admin.page.connectors`：transport/URL/command 声明只读展示；下发/停用；不展示 token。
- 上传文件用 `<input type=file>`，前端只做 16MB/扩展名预检；服务端为准。

## 5. 客户端（后续切片）

- `ui-skills`/`ui-connectors`：技能/连接器列表、安装/更新/卸载、来源、冲突/错误/空/加载/离线六态。
- 安装算法：下载 → zip sha256 → 解压到 `temp/` → tree sha256 → 原子 rename；失败清理。
- 删除事件：bootstrap 差异 → 移除本地内容目录。
- 本地技能：个人版 `~/.zeokeel/skills/local/`，不参与服务端目录。

## 6. 安全

- 管理员/owner 权限矩阵见 spec §3；所有写路由 `require_admin` 或 owner 校验。
- zip 解析用成熟库（`zip` crate）并逐条校验；临时目录随机、0700、finally 清理。
- 审计沿用 kit 哈希链，失败 fail-closed；安装回执仅运营事实。
