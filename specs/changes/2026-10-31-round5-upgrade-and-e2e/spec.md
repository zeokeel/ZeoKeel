# 2026-10-31-round5-upgrade-and-e2e · 规格

- 状态：`approved`

## 1. 业务面 426

- 中间件/层：仅作用于 `/api/client/v1/*`；排除 `auth/login`、`auth/refresh`、`/api/meta/*`、`/api/channel`、`/api/updates/*`（稳定面已在各自 handler 处理）。
- 解析 `X-ZeoKeel-Client: <semver>;<channel>;<platform>;<device_id>`；缺失/非法按最低版本处理还是放行？按 docs：客户端必须带；缺失 → 400 CLIENT_HEADER_REQUIRED（新增封闭错误码）或 426？选 426 + 说明，保持 fail-closed 升级可用。
- 版本低于 `min_client_version`（`ZEOKEEL_MIN_CLIENT_VERSION`，默认 0.1.0）→ 426 `CLIENT_TOO_OLD`，body 含 current/min/portal_url。
- 登录/刷新/稳定面始终可用。

## 2. 客户端强制升级

- transport 对 426 统一抛出 `UpgradeRequired { current, minimum, portal_url }`；app 顶层进入强制升级门禁（全屏、不可关闭、不重试业务请求），显示版本与 `打开下载页`（server_url + /portal，外部打开）。
- 其它错误保持现有处理。单测覆盖 426 解析/门禁渲染/不重试。

## 3. e2e

- m1-serve（真实 serve + SQLite）：发送一轮 → reload → usage 元素仍显示；inspector 变更 tab 在真实 git 仓库中显示修改/新增/删除项；空仓库空态。
- 不依赖外部网络，使用 mock provider/现有 e2e harness。

## 4. OEM pin

- 当 OEM 索引根是 git 仓库且目标子模块存在 gitlink（`git ls-files -s` mode 160000）时，`pin` 必须等于 gitlink commit；否则（fixture/导出包）回退目录哈希并要求 pin 等于目录哈希。
- 校验错误信息区分两种模式；docs/09 更新；单测/脚本覆盖。

## 5. 验收

见 acceptance.md。
