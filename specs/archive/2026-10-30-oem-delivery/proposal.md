# 2026-10-30-oem-delivery · 提案（M5）

- 状态：`approved`
- 评审：`docs/reviews/2026-10-30-m5-spec.md`
- 依据：`docs/09`、`docs/23`、`docs/10`、`docs/03`、`docs/27`、ADR-033/037/041/042/044/D61/D62
- 关联：M0–M4 已交付；M5 只做渠道/品牌/OEM 索引/产物/更新/门户/交付文档

## 1. 背景

M0–M4 已能产出个人版/企业版；M5 要让“新增白标客户 = 建目录/仓 + 跑流水线”，且品牌/地址/更新/签名/门户全部构建期固定。

## 2. 目标

- 渠道/品牌：`channel.json` schema、id 规则/保留字/跨渠道唯一、logo/文案/`powered_by`、构建期注入、品牌残留门禁；
- OEM 索引仓：`oem/index.toml` + submodule/pin + `channels.manifest.json`，basic/deep 两种；
- 产物矩阵：personal×official、enterprise×official、enterprise×oem-*；Docker/compose/portal/三平台安装包；
- 更新：个人版 GitHub Releases；企业版从登录服务端 1:1；服务端自建更新面 + 手动导入镜像；
- 签名：Windows 不签名（SHA256+文档）、macOS 公证 CI 就绪、Linux AppImage SHA256；
- 交付文档：部署/升级/回滚/OEM 接入/客户验收。

## 3. 非目标

- 不做 OEM 运行期换牌/换地址；不签名 Windows；不做客户闭源预构建包；不做多租户。
- 不做 M6 安全/性能加固。

## 4. 成功指标

- demo 渠道 `oem-demo` 通过 `zk channel --check` 并产出品牌化客户端/服务端镜像；
- 品牌残留门禁：`powered_by=hide` 二进制/构建产物零 “ZeoKeel” 厂商串（白标固定文案除外）；
- demo OEM 索引仓 + submodule pin 校验通过；deep 包换包矩阵通过；
- macOS 公证 CI 就绪（无证书时 dry-run）；Windows SHA256 文档；Linux AppImage SHA256；
- 企业版门户/更新端点/升级演练（无 Docker 时脚本 dry-run + 证据）。

## 5. 风险

- 无 Docker/证书/平台 runner，产物与签名只能本地脚本/dry-run；必须明确未验证边界。
- OEM submodule 需要外部私有仓，本地用 `oem/fixtures` 模拟。

## 6. 需用户拍板

- demo 渠道 id：`oem-demo`（默认）。
- macOS 公证证书后配，CI 先就绪（ADR-037）；Windows 不签名（ADR-033）。

## 7. 变更预算

`zk` 子命令 channel/brand 校验、`brands/oem-demo`、`oem/fixtures`、deploy portal/update 端点、安装包脚本、文档、e2e/脚本。
