# zk-srv-updates (`srv.updates`)

**功能说明（M5）**：企业版稳定更新面。

- `GET /api/updates/manifest`：从镜像内 `CLIENT-RELEASE.json` 读清单；`X-ZeoKeel-Client` 的 channel/edition 不符拒绝整份。
- `GET /api/updates/client/{artifact}`：从 `ZEOKEEL_UPDATE_DIR/clients/` 下载，路径安全。
- 个人版不走此接口；Windows 不签名，SHA256 由清单提供。
