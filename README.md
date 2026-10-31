# ZeoKeel

> **一切皆包的 Rust 企业级 Agent 平台。**
> 客户端 + 服务端 + 管理后台全部是一个个可替换的包；构建期组合（profile → `zk` → 生成粘合代码），
> 内核 [zeokeel-cordis](https://github.com/zeokeel/zeokeel-cordis)，模型层 [rig](https://github.com/0xPlaygrounds/rig)。
>
> **当前版本 `1.0.0-rc.1`**（2026-10-31，本地验证）。`1.0.0` 正式版需 rc 验收 + 远端 CI/release 实跑通过后由用户确认。
> 全部已知限制与 UNVERIFIED 项见 **[`KNOWN-LIMITATIONS.md`](KNOWN-LIMITATIONS.md)**（发布前必须逐项对照）。

> **优先级：个人版客户端是主体。** 个人版已完成「客户端壳/UI/agent/工具/本地 provider」；
> **企业版 = (个人版 − 个人专属包) + 企业管控包**（登录、网关、商店、钱包、审计、服务端），不是独立产品线。
> WASM 应用中心已实现（M4）。详见 `docs/PLAN.md` §4.5。

## 一页看懂

| 产物 | 内容 | 交付 | 许可 |
| --- | --- | --- | --- |
| **企业版** | 服务端（认证/用户/钱包/AI 网关/技能/连接器/WASM 应用/审计/管理台）+ 客户端（本地 agent） | 一个 docker 镜像导出包：导入 → `compose up` → 员工从企业门户下载三平台客户端 | MIT |
| **个人版** | 纯客户端、无服务端、无需登录、本地数据、自配 AI provider | 独立安装包（三平台） | MIT |
| **OEM 版** | 在上述基础上按客户替换品牌/文案/默认配置/（深度 OEM）代码包 | 每客户独立的镜像 + 三平台安装包，构建期注入渠道 | 客户约定 |

**企业版的硬约束**：员工不能自配 AI provider；登录后由服务端下发，AI 请求经服务端网关并计费；媒体/embeddings 按设计不可用。
**企业版是弱管控**：服务端只管身份、钱、商店内容、审计与版本；客户端能力完整下发，工具默认全开、
网络不限制、不做企业审批（见 `docs/00`、`docs/15`）。
**个人版**：反过来，允许自配 provider，数据全在本地。

## 快速开始（`1.0.0-rc.1`，本地）

```bash
# 前置：rustc 1.99.1（>=1.97.1）、node >= 22、pnpm >= 10、python3；WASM POC 另需 Go
pnpm install --frozen-lockfile
bash scripts/bootstrap.sh          # 构建 zk、validate、compose、生成索引、跑测试
pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit
cargo test --workspace             # 438 tests（本地基线）

./scripts/dev-desktop.sh           # 桌面客户端（Tauri；无 DISPLAY 时自动 xvfb-run）
./scripts/dev-server.sh            # 服务端：http://127.0.0.1:8787/healthz
```

已实现并在本地验证：个人版客户端（对话/流式/工具/会话/设置/本地 provider/GitHub 更新）、
企业版服务端（本地登录/EdDSA JWT/钱包账本/网关流式结算/模型目录/审计/管理台 SPA）、
内容平台（技能/连接器/管理台/客户端安装）、WASM 应用中心（wasmtime/每应用 DB/管理台）、
OEM 渠道与构建期品牌注入；本地门禁与 10 套 e2e（真实 PG/浏览器）全绿，证据见 `target/evidence/release-check/`。

**未验证（不得当作已完成）**：Docker 镜像构建/导入、三平台安装包、macOS 公证（证书后）、Windows 真机、
真实 provider/媒体在线调用、500 用户压测、GitHub-hosted-runner CI；详见 `KNOWN-LIMITATIONS.md`。

## 核心机制

```text
profiles/editions/*.toml  +  OEM 渠道目录（私有仓 + submodule）
        │
        ▼
   zk compose  ──► 生成物：Rust glue / UI registry / 服务端路由表 / 迁移清单 / API 契约 / 文档 / SBOM
        │
        ▼
  cargo build（client/server） + pnpm build（客户端 UI / 管理台 UI） + docker 打包
```

替换 = 改 profile 或客户渠道目录里的一行，重新构建；**运行期不加载任何第三方代码**。
所有"可替换"的证明来自 CI 的组合矩阵（edition × 渠道 × swap 角色）。

## 仓库结构（当前）

```text
ZeoKeel/
├─ packages/                 # 一切皆包：契约 / 运行时 / 客户端 / 服务端 / UI
│  ├─ contracts/             # zk-contract-*（trait/DTO/API 契约）
│  ├─ runtime/               # zk-runtime-*（cordis 宿主、配置、IPC、HTTP、DB、迁移）
│  ├─ client/                # agent、工具、provider、客户端功能包
│  ├─ server/                # srv-auth/users/wallet/gateway/skills/connectors/apps/audit/admin
│  └─ ui/                    # 客户端与管理台共享的 UI 基建/组件包
├─ apps/
│  ├─ desktop/               # Tauri 客户端（清单由 zk 生成）
│  ├─ server/                # axum 服务端 composition root（清单由 zk 生成）
│  └─ admin/                 # 管理台 SPA（包化，构建产物内嵌服务端）
├─ editions/  profiles/  brands/   # 组合输入
├─ oem/                      # OEM 索引仓的 submodule 挂载点（私有仓）
├─ tools/zk/                 # 组合器 CLI
├─ deploy/                   # docker：Dockerfile / compose / Caddyfile / 门户 / 备份
├─ shared/                   # 生成的 API 契约、JSON Schema、TS 类型
├─ specs/                    # 变更规格（changes/templates/archive）
├─ .agents/skills/           # 项目 Agent Skills（change/add-package/docs）
├─ experiments/              # composition-poc / wasm-poc
├─ research/                 # 一手研究 + 评审
└─ docs/                     # 规范与路线图
```

## 文档地图

**唯一完整索引 / SSOT / 术语表见 [`docs/README.md`](docs/README.md)。** 下面是 10 秒入口：

- 给 AI/贡献者：`AGENTS.md` → `docs/README.md`
- 当前状态：`docs/12`（路线图状态块）、`docs/37`（发布清单）、`KNOWN-LIMITATIONS.md`
- 产品/版本/渠道：`01`、`09`、`23`
- 架构/组合：`02`、`03`、`24`（组合与 manifest 以 24 为准）
- 客户端：`05`、`19`；服务端：`06`、`07`、`04`
- 运维/安全/钱：`20`、`21`、`22`
- 测试/决策：`11`、`13`、`15`
- 做变更/加包：`specs/README.md` + `.agents/skills/`

文档门禁：`python3 scripts/docs-check.py`（0 error / 0 warning 才算通过）。

## 已验证 / 未验证

- 本地已验证（HEAD，2026-10-31）：Rust fmt/clippy/438 tests/doc、TS 26 包 typecheck/lint/test:unit/build、
  `zk` 组合与生成物门禁、docs/spec/review/docs-command/matrix/schema、10 套 e2e（真实 PG/浏览器）。
  证据与复现命令见 `CHANGELOG.md` §1.0.0-rc.1 与 `target/evidence/release-check/`。
- 未验证（环境阻塞）：Docker/镜像、三平台安装包与公证、真机、真实 provider/媒体、500 用户压测、
  hosted-runner CI、SBOM 打包进交付物等 —— 唯一真源 [`KNOWN-LIMITATIONS.md`](KNOWN-LIMITATIONS.md)。
- `experiments/composition-poc/`、`experiments/wasm-poc/` 是机制 POC（证据/限制见各自 README），不作为产品交付物。

## 发布到 1.0.0

1. `1.0.0-rc.1` 全量门禁 + rc 验收（含 A18 personal 目标裁剪、A6 安全 13 项、A8 M6 证据）。
2. push 分支触发远端 CI；修复 hosted runner 差异。
3. tag `v1.0.0-rc.1` 触发 release，产出真实三平台包/镜像/SBOM/SHA256SUMS 并创建 GitHub Release。
4. 以上通过后由用户确认版本转 `1.0.0`。

文档入口：[docs/README.md](docs/README.md) · [运维 Runbook](docs/34-operations-runbook.md) · [已知限制](KNOWN-LIMITATIONS.md) · [贡献](CONTRIBUTING.md) · [MIT LICENSE](LICENSE)
