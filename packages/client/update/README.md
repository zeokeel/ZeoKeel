# zk-client-update-github (`update.client`)

**功能说明（M1 / ADR-041）**：个人版更新检查。

- 只读 GitHub Releases 的 `latest.json`（不下载/不安装；安装由平台更新器后续接）。
- 比较 `semver`；当前版本 >= feed 版本 → 无更新。
- `ZEOKEEL_UPDATE_FEED` 可覆盖 feed URL（离线/e2e）。
- 平台资产按 `platform` 选择（`windows-x86_64`/`darwin-aarch64`/`linux-x86_64`）。
- 企业版不走此包；企业客户端更新只从登录服务端（ADR-041）。
