# 36 · 故障排查（M6）

| 现象 | 检查 |
| --- | --- |
| 启动失败 | `.env`/DATABASE_URL；`/readyz`；迁移版本；master.key 权限 |
| 登录失败 | 服务端时间/JWT kid；账号状态；token_epoch；admin CSRF |
| 网关 402/429 | wallet 余额/价格表；hold；限流；未定价模型 |
| 技能安装失败 | 归档 sha；zip 路径/链接；客户端 tree hash；授权 grants |
| 应用无法打开 | approved/grants/can_open；server 窗口 capability；DB schema_version |
| 更新被拒 | channel/edition/client_version 1:1；CLIENT-RELEASE；SHA |
| 审计异常 | audit_log 链/权限；写失败 fail-closed；保留期 |
