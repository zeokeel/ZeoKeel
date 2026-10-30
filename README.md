# ZeoKeel

> **一切皆包的 Rust 企业级 Agent 平台。**
> 客户端 + 服务端 + 管理后台全部是一个个可替换的包；构建期组合（profile → `zk` → 生成粘合代码），
> 内核 [zeokeel-cordis](https://github.com/zeokeel/zeokeel-cordis)，模型层 [rig](https://github.com/0xPlaygrounds/rig)，
> 组织方式参考 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) 与
> [ZeoKeel-Harness](https://github.com/zeokeel/ZeoKeel-Harness)。

本文档集是**实施前规划 v3.2**（v1/v2 是单机客户端规划；v3 扩展为平台：服务端、双版本、OEM）。

> **优先级：个人版客户端是主体。** 先做个人版（客户端壳/UI/agent/工具/本地 provider）；
> **企业版 = (个人版 − 个人专属包) + 企业管控包**（登录、网关、商店、钱包、审计、服务端），不是独立产品线；
> WASM 应用中心后置。详见 `docs/PLAN.md` §4.5。

## 一页看懂

| 产物 | 内容 | 交付 | 许可 |
| --- | --- | --- | --- |
| **企业版** | 服务端（认证/用户/钱包/AI 网关/技能/连接器/WASM 应用/审计/管理台）+ 客户端（本地 agent） | 一个 docker 镜像导出包：导入 → `compose up` → 员工从企业门户下载三平台客户端 | MIT |
| **个人版** | 纯客户端、无服务端、无需登录、本地数据、自配 AI provider | 独立安装包（三平台） | MIT |
| **OEM 版** | 在上述基础上按客户替换品牌/文案/默认配置/（深度 OEM）代码包 | 每客户独立的镜像 + 三平台安装包，构建期注入渠道 | 客户约定 |

**企业版的硬约束**：员工不能自配 AI provider；登录后由服务端下发，AI 请求经服务端网关并计费。
**企业版是弱管控**：服务端只管身份、钱、商店内容、审计与版本；客户端能力完整下发，工具默认全开、
网络不限制、不做企业审批（见 `docs/00`、`docs/15`）。
**个人版**：反过来，允许自配 provider，数据全在本地。

## 快速开始（当前 M0 骨架 + 最小 demo）

仓库已从 Plan 阶段进入 **M0 骨架**：单一 Cargo workspace、契约/包/`zk` 组合器/两端 app 可运行，
最小 demo 是「桌面窗口 → 本地 agent → mock LLM → `fs.read` 工具 → 返回结果」。

```bash
bash scripts/bootstrap.sh          # 构建 zk、validate、compose、生成能力索引、跑测试
./scripts/dev-desktop.sh           # 桌面 demo（Tauri；无 DISPLAY 时自动用 xvfb-run）
./scripts/dev-server.sh            # 服务端骨架：http://127.0.0.1:8787/healthz
```

桌面 demo：输入 `读取 demo-data/hello.txt`，应看到文件正文和一条 `fs.read` 工具调用记录。

生成物（`crates/zk-composed/`、`apps/*/Cargo.toml`、`apps/*/src/generated/`、`shared/contracts/*`、
`docs/generated/*`）只由 `zk` 写；改组合请改 `editions/`、`profiles/`、`zk.toml` 后重跑
`zk compose --edition personal --profile default --channel official --check`。

> 当前 demo 是 M0 子集：未接 cordis fiber/Registrar、DB/认证/网关/管理台/React UI；
> 这些按 `docs/12` 的 M0/M1 路线图推进。Spec：`specs/changes/2026-10-30-m0-scaffold-minimal-demo/`。

## 核心机制（与 v2 一脉相承）

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
所有"可替换"的证明来自 CI 的组合矩阵（edition × 渠道 × 8 个 swap 角色）。

## 仓库结构（规划）

```text
ZeoKeel/
├─ packages/                 # 一切皆包：契约 / 运行时 / 客户端 / 服务端 / UI
│  ├─ contracts/             # zk-contract-*（trait/DTO/API 契约）
│  ├─ runtime/               # zk-runtime-*（cordis 宿主、配置、IPC、HTTP、DB、迁移）
│  ├─ client/                # agent、工具、provider、客户端功能包、UI 包
│  ├─ server/                # srv-auth/users/wallet/gateway/skills/connectors/apps/audit/admin
│  └─ ui/                    # 客户端与管理台共享的 UI 基建/组件包
├─ apps/
│  ├─ desktop/               # Tauri 客户端（清单由 zk 生成）
│  ├─ server/                # axum 服务端（清单由 zk 生成）
│  └─ admin/                 # 管理台 SPA（包化，构建产物内嵌服务端）
├─ editions/                 # enterprise.toml / personal.toml
├─ profiles/                 # base/default/mock/minimal/hardened/...
├─ oem/                      # OEM 索引仓的 submodule 挂载点（私有仓）
├─ tools/zk/                 # 组合器 CLI
├─ deploy/                   # docker：Dockerfile / compose / Caddyfile / 门户
├─ shared/                   # 生成的 API 契约、JSON Schema、TS 类型
├─ brands/                   # official 渠道品牌资产（logo/文案/主题）
├─ specs/                    # 变更规格（changes/templates/archive）
├─ .agents/skills/           # 项目 Agent Skills（change/add-package/docs）
├─ experiments/              # composition-poc / wasm-poc
├─ research/                 # 一手研究 + 评审
└─ docs/                     # 本规划
```

## 文档地图

**唯一完整索引 / SSOT / 术语表见 [`docs/README.md`](docs/README.md)。** 下面是 10 秒入口：

- 给 AI/贡献者：`AGENTS.md` → `docs/README.md`
- 产品/版本/渠道：`01`、`09`、`23`
- 架构/组合：`02`、`03`、`24`（组合与 manifest 以 24 为准）
- 客户端：`05`、`19`；服务端：`06`、`07`、`04`
- 运维/安全/钱：`20`、`21`、`22`
- 测试/决策：`11`、`13`、`15`
- 做变更/加包：`specs/README.md` + `.agents/skills/`

文档门禁：`python3 scripts/docs-check.py`（0 error / 0 warning 才算通过）。

## 已验证 / 未验证

`experiments/composition-poc/` 验证了**构建期换包**（契约漂移的设计性失败已论证，但 POC 未入库负向 fixture，见该 README §未验证）；
`experiments/wasm-poc/` 验证了 **Go/Rust(wasip1) 双 guest + 帧 ABI + 宿主 SQLite + fuel 限额**；
它不含 cordis、服务端、UI、锁、CI。服务端设计主要来自对
[ZeoKeel-Harness](https://github.com/zeokeel/ZeoKeel-Harness) 的一手研究（见
[`research/harness-server.md`](research/harness-server.md)、
[`research/harness-client-channel.md`](research/harness-client-channel.md)），不是已经实现的事实。

## 下一步

先做 S0 的 6 个 spike（双 app 组合、WASM 运行时与每应用 DB、认证与 JWT、OEM submodule CI、
迁移不可变、管理台包化），每个 ≤2 天并落 ADR；细节与"待你拍板"的问题见
[`docs/15-open-questions.md`](docs/15-open-questions.md)。
