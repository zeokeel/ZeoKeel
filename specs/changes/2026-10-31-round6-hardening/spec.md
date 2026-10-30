# 2026-10-31-round6-hardening · 规格

- 状态：`approved`

## 1. 426 与客户端身份

- 所有 web→server fetch 通过统一 `readJsonOrUpgrade`/`upgradeFromResponse`：426 → `UpgradeRequiredError`（含 body 解析），不再落 contentError。
- `portal_url` 相对路径以 `activeServerUrl` 解析为绝对；缺失时回退 server_url+/portal。
- 强制升级页提供"查看版本说明"（显示 current/min/server version，来源 426 body/`meta.get`）与门户链接；不重试业务请求；i18n。
- `X-ZeoKeel-Client` platform 由 `navigator.userAgentData?.platform || navigator.platform` 归一（linux-x64/windows-x64/macos）；所有 server 请求带 `X-Request-Id`（uuid，回显服务端 header）。

## 2. 426 envelope / 渠道 min version

- 426 body：`{error:{code:"CLIENT_TOO_OLD",message,retryable:false,request_id,current?,minimum,portal_url}}`；其它业务错误逐步统一 envelope 由 docs/04 注明 426 特例，客户端兼容 flat/nested。
- 服务端中间件按**服务端构建渠道**读取 `ZEOKEEL_MIN_CLIENT_VERSION_<CHANNEL>`（客户端头 channel 不参与，防伪造），env 全局兜底；如实现 brand 编译值则一并使用，否则删除该不可达分支并同步 docs。
- `X-Request-Id` 请求头若缺失生成 uuid，响应回显，client 会发送。

## 3. OEM pin
- git 索引（`.git` 存在）必须存在 gitlink；缺失/gitlink 与 pin 不符 → 直接失败（不再回退目录哈希）。
- 非 git 目录（fixture/export）才回退目录哈希，错误/警告明确 `fixture fallback`。
- channel_validate 测试：gitlink 匹配/不匹配、git 仓库无 gitlink 报错、非 git 回退通过。

## 4. Tauri
- `apps/desktop/tauri.conf.json` CSP：`connect-src ipc: http://ipc.localhost https: http://127.0.0.1:* http://localhost:*`（企业 server 动态 HTTPS；本地 serve/开发）；`default-src` 等保持严格。
- 新增 IPC `system.open_url`：仅 http/https，长度限制，固定平台 argv (`xdg-open`/`open`/`cmd /C start "" <url>`)，错误回传；测试 URL 校验（不实际打开）。
- UpgradeGate：若 `window.__TAURI_INTERNALS__` 存在调用 `system.open_url`，否则 `window.open(..., "_blank", "noopener")`；单测两种路径。

## 5. changes 文案
- inspector 变更项显示"文件变更 +add/-del"（文件级），docs/27 说明无行级 diff；e2e 断言不变（+1 −0 仍成立但文案更新）。
