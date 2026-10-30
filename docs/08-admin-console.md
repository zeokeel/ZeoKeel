# 08 · 管理后台：也是一个包集合

> 已拍板：管理后台必须和客户端一样"一切皆包"，可任意替换、可 OEM 换皮。
> 本文件说明它如何落进同一套包模型、组合机制与交付链路。

## 1. 形态

```text
浏览器  ──HTTPS──►  服务端 /admin/*（静态资源，内嵌二进制）
                        │
                        ├─ @zeokeel/admin-shell        （布局/导航/登录态）
                        ├─ @zeokeel/admin-users        （用户/角色/禁用）
                        ├─ @zeokeel/admin-wallet       （余额/充值/流水）
                        ├─ @zeokeel/admin-pricing      （模型价格/峰谷）
                        ├─ @zeokeel/admin-gateway      （provider/价格/路由）
                        ├─ @zeokeel/admin-usage        （用量/账单/报表）
                        ├─ @zeokeel/admin-skills       （技能审核/归属）
                        ├─ @zeokeel/admin-connectors   （连接器下发）
                        ├─ @zeokeel/admin-apps         （应用上下架/运行诊断）
                        ├─ @zeokeel/admin-audit        （审计）
                        ├─ @zeokeel/admin-settings     （系统配置/品牌/更新）
                        └─ @zeokeel/admin-ui-kit       （契约包：路由/权限/表格/表单）
```

- 管理台是一个独立的 React SPA 构建（`apps/admin`），但**由 UI 包组装**，不是一个大前端。
- 服务端只提供静态资源与 `/api/admin/*`，不感知前端包结构；包选择在构建期由 edition/profile 决定。
- 管理台可以单独部署（把 `dist` 放到任意静态服务器），也可以内嵌服务端二进制（默认）。

## 2. 包模型

管理台 UI 包与客户端 UI 包使用**同一套前端包规范**（见 `16-ui-packages.md`），差别在：

| 项 | 客户端 UI 包 | 管理台 UI 包 |
| --- | --- | --- |
| 清单 | `zk.toml` + `web/package.json` 的 `zeokeel.client` | 同上，`zeokeel.admin` |
| 角色族 | `ui.*` | `admin.slot.*` / `admin.page.*` |
| 契约 | `zk-contract-ui` | `zk-contract-admin-ui`（路由、权限、页面注册） |
| 权限 | 无（本地） | 每个页面/操作声明权限点；由服务端 API 强制，前端只做显示 |
| 主题 | 客户端主题包 | 管理台主题包（可 OEM） |

页面注册是静态声明（ADR-017）：

```toml
# packages/server/admin-users/zk.toml
[facets]
rust = { crate = "zk-srv-users" }           # 提供 /api/admin/users* 路由（域包）
ui   = { package = "@zeokeel/admin-users" }

[[ui.contributions]]
slot      = "admin.nav"
component = "admin.users"
order     = 20
permission = "users.read"
```

## 3. 服务端 API 包与管理台 UI 包的配对

一个域通常由两个包组成：

```
packages/server/users/            # Rust：路由 + 仓储 + 权限点（zk-srv-users）
packages/server/admin-users/      # UI：页面（@zeokeel/admin-users）+ 声明 crate（zk-admin-users）
```

- 二者在 profile 里分别选择；可以只换 UI 不换 API（OEM 换皮），也可以整体替换（自研管理端）。
- `zk validate` 校验：管理台页面的 `permission` 必须存在于服务端 API 包声明的权限点集合。
- 管理台 API 的权限矩阵由生成器产出（`shared/contracts/admin-permissions.json`），CI 校验覆盖率。

## 4. OEM 与可替换性

- 品牌：`brand` 包提供 logo/名称/主题色；管理台主题包提供 token 覆盖。
- 文案：i18n 包，OEM 客户仓可覆盖词条（见 `09-delivery-and-oem.md`）。
- 页面：OEM 可禁用某页（从 profile 里移除）、替换某页（提供同 component id 的包）、新增页面（新包）。
- 深度 OEM 可替换整个域的实现（例如客户已有用户中心，替换 `srv-users` 与 `admin-users`），
  但必须满足 `zk-contract-admin-api` 契约。

## 5. 构建与交付

```bash
zk compose --edition enterprise            # 生成 admin registry + 路由表
pnpm --filter @zeokeel/admin-shell build    # 产出 dist/admin
cargo build -p zeokeel-server               # include_dir! 把 dist/admin 编进二进制
```

- 生产镜像里管理台静态资源在二进制内，不需要单独容器（参考实现同样内嵌）。
- 管理台与客户端 UI **可以共享组件包**（表格、表单、图标、i18n），但共享包必须是契约包或纯 UI 包，
  不允许直接 import 对方业务组件。
- 管理台体积预算：gzip 后 ≤ 1.5 MB（首屏），超出要拆包或降级。

## 5.5 管理台规则（v3.1：管理员安全，不是员工管控）

- **角色**：超管（全部写权限）、**审计员（只读）**、员工（无管理台权限）。审计员仅可访问审计、用量、
  登录事件、内容变更记录页；后端强制只读，前端隐藏写按钮不作为安全边界。
- **无自由入口**：不提供任意 SQL、脚本、命令、文件浏览器、日志 grep 等"万能入口"。
- **每个写操作**：权限点 + 二次确认（不可逆操作）+ before/after 审计 + 影响预览（如"将禁用 37 个用户"）。
- **批量与导入**：有上限、有模板、有校验、有预览；失败逐条报告，不静默跳过。
- **危险操作**：删除用户/清空余额/下架应用/改 provider，需要输入名称确认 + 审计；保留期/回收站。
- **配置项**：每项标注 hot/restart 与影响范围；保存后显示"已生效/需重启"，不允许模糊。
- **只读优先**：能用只读视图表达的信息不给编辑入口；导出也审计。

## 5.6 门户边界

- 服务端提供**员工客户端下载门户**（`/portal/*`，随渠道品牌），不是 OEM 自助门户；
  管理台不提供客户自助改品牌/版本的功能。
- 门户只读：列出三平台安装包 + 版本 + SHA256 + 安装说明；无登录也可访问（可选配置登录后可见）。

## 6. 安全

| 项 | 要求 |
| --- | --- |
| 会话 | Cookie（HttpOnly/Secure/SameSite）+ CSRF token；空闲超时；超管/审计员可登录（审计员只读） |
| 权限 | 服务端强制；前端隐藏不作为安全边界 |
| 审计 | 所有写操作记录 actor/action/target/before/after（脱敏） |
| 危险操作 | 删除用户/清空余额/下架应用/改 provider 需要二次确认 + 审计 |
| XSS | 无 `dangerouslySetInnerHTML`（除白名单）；CSP；依赖审计 |
| 内嵌资源 | 使用 `include_dir!` 时禁止目录穿越；开发模式另起 vite |

## 7. 待确认

- 管理台是否复用客户端主题包（一份 token 两处用）还是独立主题包？
- ~~审计员可见页面范围~~ → 已定：审计/用量/登录/内容变更，只读（D53）。
- OEM 深度定制管理台时，客户是否需要自己构建前端（还是我们代构建）？
