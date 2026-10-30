# 2026-10-31-round6-hardening · 验收

- 状态：`done`（C19 独立终评 APPROVED at HEAD 7cd0362；首评 4 P1 + P1-R1 已修）
- 验收人：lead；C19 独立评审：auditor-build（非实现者）

## 1. 验收结果

| 命令 | 通过条件 |
| --- | --- |
| cargo fmt/clippy/test --workspace | 0 |
| zk validate/compose personal --check/capability/docs/db lock/lint duplicates | 0 |
| pnpm -r typecheck/lint/test:unit/build | 0 |
| m1/agent/login/admin e2e + client-upgrade e2e | 全绿 |
| docs/spec/review gates | 0 |
| 426 真实 e2e | `scripts/client-upgrade-e2e.sh` 0（gate 显示 0.1.0/9.9.9、portal、无重试） |
| 426 envelope/渠道/request-id、OEM pin、Tauri CSP/open_url | 单测+真实 PG smoke/oem e2e 0 |
| C19 评审 | APPROVED（auditor-build 独立；P0/P1=0） | docs/reviews/2026-10-31-round6-verification.md |

## 1.1 实现记录

- web：`serverFetch` 统一所有 server 直连（client header + X-Request-Id + 426→UpgradeRequiredError），相对 portal 解析、版本说明、platform 归一、changes 文件级文案、Tauri 外部打开分支；真实 426 升级 e2e。
- server：426 统一信封（含 request_id/retryable/x-request-id 回显）、渠道级 min_client_version（服务端构建渠道的 `<CHANNEL>` override → global；客户端 header channel 不参与）、CORS request-id；native Tauri app 请求带客户端标识。
- OEM：git 索引强制 gitlink；仅非 git fixture 回退目录哈希；4 个 channel_validate 测试 + oem-e2e。
- Tauri：CSP connect-src 允许企业 https + 本地回环；`system.open_url` 仅 http/https、固定 argv、无等待；能力不新增 ACL（自定义命令）。
- changes：UI 标注文件级 +add/−del（无行级 diff），docs/27/21/19/09/04 同步。

## 1.2 复评记录

- 首评 CHANGES-REQUESTED：P1-1 native app_request/assets 缺客户端头、P1-2 渠道头伪造绕过、P1-3 brand min 不可达、P1-4 Windows cmd 注入。
- 修复：native_client.rs + CHANNEL 生成；权威 server channel min（spoof 实测 official/beta 均 426 min 9.9.9）；删除死分支并更新 docs；rundll32 固定 argv；portal 同源/http(s) 锁；P1-R1 生成物漏提交已补（7cd0362）。
- 终评核准：compose --check 0、review-check 0（18 records）、gates/e2e 复核 0。
- 非阻塞残留（review §8.4）：bootstrap/updates 426 flat envelope、audit request_id 未落库、gateway /v1 未覆盖 426、open_url 子进程 reap/失败 UX、request-id 字符集、OEM nested-repo fallback、native app 路径无 e2e、**P2-N1 zk JSON key 顺序依赖构建图**（round-7 修）。

## 2. UNVERIFIED

- 真实 OEM 私有索引/submodule（fixture fallback 路径覆盖）；Tauri 真机（CSP/外链代码测试，不含显示环境）；cargo-deny 本地；Docker/多平台。
