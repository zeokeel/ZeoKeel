# Changelog

## 1.0.0-rc.1 (2026-10-31)

> 发布候选（本地验证）。`1.0.0` 正式版需 rc 验收 + 远端 CI/release 实跑通过后由用户确认；
> 全部残余项见根 [`KNOWN-LIMITATIONS.md`](KNOWN-LIMITATIONS.md)。

### 版本与许可

- 产品版本统一为 `1.0.0-rc.1`：workspace `Cargo.toml`/`Cargo.lock`、根 `package.json`、
  `apps/desktop/web/package.json`、`apps/admin/web/package.json`、`apps/desktop/tauri.conf.json`。
- 内部私有 `@zeokeel/*` workspace 包（`packages/**/web/package.json`）保持 `0.1.0`：不单独发布，
  仅作为构建期组合的源码包；产品表面版本以根/应用清单为准（理由见 `KNOWN-LIMITATIONS.md` §1）。
- 新增根 `LICENSE`（MIT 全文）；`Cargo.toml` `license = "MIT"` 与之一致。
- 新增根 `KNOWN-LIMITATIONS.md`（已知限制/UNVERIFIED 唯一真源）、`docs/40-known-limitations.md`（摘要并在 `docs/README` 登记）、
  `docs/37` 发布清单与 `CHANGELOG`/`README`/`AGENTS`/`docs/12` 与当前状态对齐。

### 已验证（本地 HEAD，证据 `target/evidence/release-check/`、`target/evidence/release-1.0-rc1/`）

- Rust：`cargo fmt --check`、`cargo clippy --workspace --all-targets --all-features -D warnings`、
  `cargo test --workspace`（438 passed）、`cargo test --workspace export_schema`（无生成物漂移）exit 0；
  `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` exit 0（`packages/server/jobs` 私有 intra-doc 链接已修复）。
- TS/UI：`pnpm -r typecheck|lint|test:unit|build` exit 0（27 workspace projects，证据 `target/evidence/release-1.0-rc1/ci/pnpm-gates.log`）。
- 组合/生成物：`zk validate`、`zk compose --edition personal --profile default --channel official --check`（16 files）、
  `zk capability --check`（45 entries）、`zk docs --check`（2 files）、`zk db lock --check`（16 migrations）、
  `zk lint --duplicates`、`zk sbom --check` 全 0；`scripts/docs-check.py` 0/0、`scripts/spec-check.py` 0、
  `scripts/review-check.py`、`scripts/docs-command-check.py`（含 self-test）、`scripts/agent-tool-matrix.py --check`、
  `scripts/schema-check.sh` 全 0。
- e2e（真实 PG/浏览器）：m1、schedule、agent、login、admin、client-upgrade、apps、updates、oem、srv-auth 共 10 套 exit 0
  （`target/evidence/release-check/e2e/summary.log`）。
- SBOM：`zk sbom` 产出 personal/enterprise CycloneDX 1.5 + SHA256（`target/evidence/release-1.0-rc1/sbom/`）；
  工作区级覆盖与限制见 `KNOWN-LIMITATIONS.md` §5。
- 入口文档真源：`README.md`、`AGENTS.md` §1/§6/§12、`docs/12`、`docs/20`（失效脚本引用修正）、`docs/37`、`docs/README` 已对齐 HEAD。

### UNVERIFIED（环境/交付阻塞，不得视为已验证）

- Docker 镜像构建/导入/`compose up`、镜像内 SBOM、交付 tar；`deploy/` 仅有源文件。
- 三平台客户端安装包；macOS 公证+staple（证书后，ADR-037）；Windows 按设计**不签名**（ADR-033）。
- 备份 PITR/WAL、Docker 升级/回滚 drill、可复现构建（server/desktop/admin/镜像）、性能预算部分项。
- 500 用户压测；真实 provider/媒体/embedding 在线调用（企业版媒体/embeddings 按 C7 不提供）。
- GitHub-hosted-runner CI 与 release workflow 实跑；`v1.0.0-rc.1` tag/GitHub Release 尚未创建。
- M6 acceptance 仍 `blocked`（`specs/changes/2026-10-30-hardening-release/acceptance.md`）；威胁模型独立评审判定见 `docs/reviews/`。

### 已知限制（摘要）

- agent/rig 残余（`AgentEvent` usage 非独立事件、`SessionStore::messages_in` 回放缺、rig run-state/telemetry/effect 与
  ModelCatalog 上下文窗口/压缩未接）、工具 partial（terminal/pwsh-persistent/goal/ralph/agent-team）、
  personal 组合 `apps/server` 目标裁剪待 A18 验收后生效、SBOM 覆盖边界：详见 `KNOWN-LIMITATIONS.md`。

## 0.1.0 (2026-10-30)

- M0 骨架/组合器/契约/双端/生成物。
- M1 个人版客户端（agent/流式/会话/工具/设置/更新）。
- M2 企业版服务端（认证/用户/钱包/网关/技能/连接器/审计/管理台/登录）。
- M3 内容平台（技能/连接器/管理台/客户端安装/OAuth）。
- M4 WASM 应用中心（wasmtime/每应用 DB/管理台/客户端窗口）。
- M5 OEM/交付（渠道/品牌/更新/门户/dist dry-run；Docker/签名环境项 UNVERIFIED）。
- M6 加固启动（安全门禁/备份恢复/故障注入/性能/可复现/文档）；**历史记录**：后续审计显示 M6 未完成
  （acceptance `blocked`；Docker/证书/真机等 UNVERIFIED），不作为任何版本的完成声明。
