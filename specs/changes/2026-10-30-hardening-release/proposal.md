# 2026-10-30-hardening-release · 提案（M6）

- 状态：`draft`
- 评审：`docs/reviews/2026-10-30-m6-spec.md`
- 依据：`docs/12` M6、`docs/11`、`docs/18`、`docs/20`、`docs/21`、`docs/15` D60–D65、ADR-033/037/041/057

## 1. 背景

M0–M5 已实现；M6 做加固与 1.0 发布：安全评审、备份恢复、故障注入、限流/超时、日志脱敏、i18n/a11y、性能、可复现构建/SBOM、文档。

## 2. 目标

- 内部安全评审清单（认证/网关/WASM/文件/供应链/弱管控声明），无高危未修。
- 备份/恢复演练 + 故障注入脚本；审计/用量保留期；日志脱敏检查。
- 全量 axe/键盘/i18n 审计证据；性能预算脚本。
- 可复现构建校验、SBOM/许可证清单、三平台/镜像/门户/文档 1.0 清单。
- 文档：部署/运维/API/贡献/排障；弱管控风险声明。

## 3. 非目标

- 不新增业务功能；不改 M0–M5 契约语义；不做 Windows 签名；不做外部安全认证（D60）。

## 4. 成功指标

- `scripts/security-audit.sh` 0 高危；`scripts/backup-restore.sh` 演练成功；`scripts/fault-injection.sh` 不损坏数据。
- e2e axe 全绿；性能预算报告；两次构建产物 hash 一致（或记录已知非确定项）。
- SBOM/许可证清单；文档齐全；独立评审 APPROVED。

## 5. 风险/边界

- 无 Docker/证书/平台 runner：镜像/安装包/公证仍 UNVERIFIED（继承 M5）。
- 可复现构建受时间戳/路径影响，需固定 `SOURCE_DATE_EPOCH`/路径；不能保证 Tauri 安装包。

## 6. 需用户拍板

- 无；沿用 D60–D65/ADR-033/037。
