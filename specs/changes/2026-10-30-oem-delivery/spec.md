# 2026-10-30-oem-delivery · 规格（M5）

- 状态：`draft`
- 依据：`docs/09` §1–§7（channel schema/产物/签名/更新）、`docs/23`、`docs/03` §6.2（`zk channel validate|verify`）、`docs/10`、`docs/27` §4
- SSOT 声明：channel.json 字段以 `docs/09` §3 为唯一真源；本文只写验收增量，不复制 schema 正文。

## 1. 渠道/品牌

- channel.json 必含 `identity.{slug,product_name,app_id,company,copyright,deep_link_scheme,app_origin_scheme,data_dir_name,product_home_dir}`、`brand.logo.light/dark`、`brand.powered_by`、`assets.accent`、`copy.*`、`defaults.server_url`；locale/strings 在渠道目录 `strings/<locale>.json`，服务端配置在 `config/defaults.toml`。
- id：`^[a-z0-9][a-z0-9-]{0,31}$`；保留字 `official/beta/personal/demo/test`；客户 `oem-<customer>`；**channel 不含 edition**（edition 只来自 editions/index/oem.toml）。
- 跨渠道唯一：app_id、deep_link_scheme、app_origin_scheme、data_dir_name、product_home_dir、product_name。
- 官方：`brands/official/channel.json` 使用 docs/09 全字段；`defaults.server_url=""`；个人版必须空且无登录。
- logo：相对路径、canonicalize 在渠道目录内、≤256KB、禁 script/外链；生成物 `brand.ts`/`brand.json`/`channel.json` 由 `zk compose` 写，逐字节 `--check`；打包后重派生比对。
- `powered_by`：`show|hide` 必填；`hide` 产物不得出现厂商品牌（allowlist：MIT/NOTICE 法律文本、包名、生成渠道文件的 `identity.slug`）；`show` 只允许固定 `Powered by ZeoKeel` 文案。
- 单一读取点：除组装期生成器外源码不得读 `channel.json`。

## 2. OEM 索引

- `oem/index.toml` schema + `[channels.<id>] submodule/pin/edition/depth/owner`；`.gitmodules`、submodule gitlink、`index.pin` 三处一致；`channels.manifest.json` 渠道集与 `[channels]`/目录名一致。
- 客户子仓：`oem.toml`（channel/edition_base/locales/platforms/depth/[packages]）、`channel.json`、logo/strings/config；basic/deep。
- deep：`[packages]` 只能覆盖实现角色，禁止契约/内核；包必须四件套 + conformance + 许可证；`zk plan` 换包矩阵通过。
- 本地无私有仓时 `oem/fixtures` 模拟（pin=目录内容 hash），证据标注“模拟”。

## 3. 交付/签名/更新（docs/09 §4–§7、docs/23 §3）

- 产物树 `dist/<version>/<channel>/`：`clients/`（三平台安装包）、`images/`（server tar.zst）、`SHA256SUMS`、`SBOM/`（CycloneDX）、`CLIENT-RELEASE.json`、`CHANNEL.json`、`VERSION`、`deploy/`、`HANDOVER.md`。
- `CLIENT-RELEASE.json` 是门户/1:1 更新唯一版本源；镜像内 `/opt/zeokeel/client/` 含三平台客户端 + `CLIENT-RELEASE.json`；镜像 `CHANNEL.json` 脱敏。
- Windows 不签名：SHA256 + SmartScreen 文档；macOS 公证/staple：CI job 就绪，证书缺失时 dry-run 标记；Linux AppImage SHA256。
- 更新：个人版 GitHub Releases `latest.json`+`SHA256SUMS`；企业版客户端只从登录服务端 1:1；`GET /api/updates/manifest` 校验 `channel_id/edition/client_version` 匹配 `CLIENT-RELEASE.json`，不匹配整份拒绝；服务端手动 `docker load`+备份+compose。
- 门户：按渠道下载客户端，CSP 无内联脚本；portal 只读。
- 升级演练：下载→SHA256→docker load→备份→compose up→`/readyz`→失败回滚；无 Docker 时 dry-run 并标注。

## 4. 验收断言

- `zk channel validate --index <dir> --all` 全绿；非法 id/保留字/缺键/未知键/跨渠道唯一/logo 逃逸/`powered_by` 未知 → 失败。
- `zk channel verify --channel <id> --dist <dir>`：产物重派生逐字节 + `channel_id` 拆包断言 + `powered_by` 分支。
- `zk compose --edition enterprise --channel official --check`、`--channel oem-demo --check` 绿；`brands/official` 与 oem-demo 生成物一致。
- `bash scripts/oem-e2e.sh`：渠道校验、OEM fixture 三处 pin、manifest、deep 覆盖、残留扫描、`CLIENT-RELEASE.json`/`CHANNEL.json`/SHA256 断言。
- 门户/更新 e2e：Playwright 下载页 + 1:1 拒绝矩阵；`scripts/upgrade-drill.sh` dry-run/真 Docker 证据。
- 文档/ADR 回写 docs/09/20/23；独立评审 APPROVED。
