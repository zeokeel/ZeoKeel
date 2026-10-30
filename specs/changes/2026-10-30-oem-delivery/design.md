# 2026-10-30-oem-delivery · 设计（M5）

- 状态：`draft`

## 1. 命令/生成

- `tools/zk` 新增 `channel validate --index <dir> --all`、`channel verify --channel <id> --dist <dir>`（docs/03 §6.2）；现有 `compose` 扩展读取 docs/09 channel.json 全字段。
- 生成物：`brand.ts`/`brand.json`（docs/27 §4）+ `channel.json`（随包分发）；逐字节 `--check`；打包后 `channel verify` 重派生。
- `ChannelFile` 解析 docs/09：`identity`/`brand`/`assets`/`copy`/`defaults`；edition 不再从 channel 读取；兼容旧 `brand.name` 仅作为 product_name fallback，M5 内淘汰。

## 2. 品牌残留

- 扫描范围：dist 安装包解包/镜像层/生成物；二进制 strings + 文本。
- allowlist：`LICENSE`/`NOTICE` 法律文本、包名 `zeokeel`、生成 `channel.json` 的合法 identity slug、`Powered by ZeoKeel`（仅 show）。
- `hide` 分支：官方 logo 文件 hash、`ZeoKeel` 显示串、官方域名出现即失败；白名单文件按生成指纹，不按路径。

## 3. OEM 索引

- fixture `oem/fixtures/index.toml`、`.gitmodules` 模拟、客户目录、manifest；pin=目录内容 SHA256；真实仓用 `git ls-tree` 校验 gitlink。
- deep 覆盖用 profiles/editions 机制；`zk plan`/`compose --check` + 包四件套/conformance。

## 4. 交付

- `scripts/oem-e2e.sh` 构造 demo 渠道产物（不做真实镜像/签名），断言树/校验和/CLIENT-RELEASE/CHANNEL.json。
- `deploy/portal` 增加渠道下载页；`/api/updates/manifest` 从 `CLIENT-RELEASE.json` 读并 1:1 校验。
- `scripts/upgrade-drill.sh` 参数 `--dry-run|--docker`；无 Docker 时 dry-run。

## 5. 测试

- `cargo test -p zk`（channel schema/validate/verify）、`scripts/oem-e2e.sh`、`e2e/m5-portal.spec.ts`、`upgrade-drill.sh`、docs/spec/review。
- 独立评审 audit-arch R1/R2。
