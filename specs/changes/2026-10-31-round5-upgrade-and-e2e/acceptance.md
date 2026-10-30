# 2026-10-31-round5-upgrade-and-e2e · 验收

- 状态：`done`（C19 独立复评 APPROVED at HEAD 9e2ae62）
- 验收人：lead；C19 独立评审：auditor-rig

## 1. 验收结果

| 命令 | 通过条件 |
| --- | --- |
| cargo fmt/clippy/test --workspace | 0 |
| zk validate/compose personal --check/capability/docs/db lock/lint duplicates | 0 |
| pnpm -r typecheck/lint/test:unit/build | 0 |
| m1/agent/login/admin e2e + m1-serve usage/changes | m1 含 serve 7 passed（usage 重启 + changes 真实 git）；agent 9；login 4；admin 11 |
| docs/spec/review gates | 0 |
| e2e apps/updates/oem | 全 0 |

## 1.1 实现记录

- 服务端 426：`/api/client/v1/*` 校验 `X-ZeoKeel-Client`；login/refresh/bootstrap/稳定面例外；CORS preflight 放行；缺头/非法 fail-closed；单测+真实 PG smoke。
- 客户端：生成 `BRAND.clientVersion`，所有 server 请求带 `X-ZeoKeel-Client: <semver>;<channel>;desktop;<device>`；426 统一升级门禁（不可关闭、门户链接、不重试）。
- e2e：m1-e2e 拆分 mock profile 与 personal serve（serve 只用 personal baseline 构建）；真实 SQLite usage reload、真实 git changes/空态；运行期 settings.json 已加入 serve workspace gitignore。
- OEM pin：git 索引读 submodule gitlink（mode 160000），fixture/非 git 回退目录哈希；docs/09 明确两种路径。

## 2. UNVERIFIED

- 真实签名密钥/证书、Docker/多平台、`zk api` 路由真源、cargo-deny 本地、真实超大规模仓库 changes。
- **Tauri 真机企业登录/强制升级 UNVERIFIED**：round-5 客户端证据为浏览器/serve 载体；Tauri 壳 CSP（`connect-src`）与外部打开门户能力需在真实桌面壳验证。
- P2（review §4）：store.ts 直连 426 未统一映射门禁、portal_url 相对路径、错误 envelope、平台/X-Request-Id、docs/19 门禁动作、per-channel min version、无真实 server 426 e2e、OEM dir-hash fallback 限制、changes 行级计数。
