# 2026-10-31-round6-hardening · 设计

- 状态：`approved`

## 1. web
新增 `serverFetch`/`readJsonOrUpgrade` 统一封装（放在 protocol/store 共享）；client.ts 生成 platform/request-id；UpgradeGate 增加版本说明块与外部打开抽象；store 全部 fetch 替换。

## 2. server
426 envelope 扩展；min version 从 channel 配置读取（已有 channel settings 机制/`ZEOKEEL_MIN_CLIENT_VERSION` 兜底）；middleware 读取请求 channel 头。OEM pin 分支在 tools/zk。

## 3. Tauri
CSP 字符串替换；`system.open_url` in apps/desktop/src/ipc.rs + capability allow；web detect Tauri global。

## 4. e2e
新增 `scripts/client-upgrade-e2e.sh`：启动真实 server（min=9.9.9）+ desktop serve + fake upstream，Playwright `e2e/client-upgrade.spec.ts` 断言登录后业务调用 426→升级页、无重试、portal 链接；结束后清理。
