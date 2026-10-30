# zk-store-settings-json

本地设置实现：`$ZEOKEEL_HOME/settings.json`（否则系统 data 目录）。原子写（临时文件 + rename）；
Unix 权限 0600。API key 只存本地，M1 后续接系统密钥库。规范见 `docs/05` §5、`docs/19` §3.1。
