# 18 · 严格类型与静态检查（防止技术性 bug）

> 目标：**让"能编译"等价于"类型/边界/错误处理都过关"**。TS 与 Rust 都开最严格档；
> 所有外部输入（IPC/WS/HTTP/配置/磁盘）必须运行时校验；CI 全量强制执行，例外必须写明理由。

> 参考配置已放在 [`configs/`](../configs/)（scaffold 时复制到仓库根）。

## 1. TypeScript：最严格 tsconfig

```jsonc
// tsconfig.base.json（所有前端包继承）
{
  "compilerOptions": {
    "target": "ES2023",
    "lib": ["ES2023", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "moduleDetection": "force",
    "jsx": "react-jsx",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "verbatimModuleSyntax": true,
    "isolatedDeclarations": true,        // 库包必须能独立 emit d.ts
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "noEmit": true,

    "strict": true,
    "noUncheckedIndexedAccess": true,    // arr[i] 可能是 undefined
    "exactOptionalPropertyTypes": true,  // 区分 missing 与 undefined
    "noImplicitOverride": true,
    "noPropertyAccessFromIndexSignature": true,
    "noFallthroughCasesInSwitch": true,
    "noImplicitReturns": true,
    "allowUnreachableCode": false,
    "allowUnusedLabels": false,
    "useUnknownInCatchVariables": true,
    "forceConsistentCasingInFileNames": true,
    "noErrorTruncation": true,

    "skipLibCheck": true                 // 唯一妥协：第三方 d.ts；自有包必须全绿
  }
}
```

规则：

- **禁止 `any`**；用 `unknown` + 运行时 parse。
- **禁止类型断言 `as`**（`as const` 除外）；需要断言 = 说明边界没校验。
- 每个包 `tsc -b` 必须 0 error；CI 用 `tsc -b --pretty false`。
- 类型级测试：`expect-type`/`vitest --typecheck` 断言"错误用法编译不过"。

## 2. ESLint：类型感知的最严规则

```js
// eslint.config.js（flat config，各包复用）
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";

export default tseslint.config(
  ...tseslint.configs.strictTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,
  {
    languageOptions: {
      parserOptions: { projectService: true, tsconfigRootDir: import.meta.dirname },
    },
    plugins: { "react-hooks": reactHooks },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/no-floating-promises": "error",
      "@typescript-eslint/no-misused-promises": "error",
      "@typescript-eslint/await-thenable": "error",
      "@typescript-eslint/no-unnecessary-condition": "error",
      "@typescript-eslint/switch-exhaustiveness-check": "error",
      "@typescript-eslint/consistent-type-imports": "error",
      "@typescript-eslint/no-unnecessary-type-assertion": "error",
      "@typescript-eslint/prefer-readonly": "error",
      "@typescript-eslint/restrict-template-expressions": "error",
      "@typescript-eslint/no-base-to-string": "error",
      "no-console": ["error", { allow: ["error", "warn"] }],
      "eqeqeq": ["error", "always"],
      "no-restricted-syntax": ["error", "WithStatement", "LabeledStatement"],
    },
  },
);
```

CI：`eslint . --max-warnings=0`（warning 也算失败）。

## 3. 运行时校验：边界处 parse，不做类型断言

- Rust 契约用 `schemars` 生成 JSON Schema（`shared/contracts/*.schema.json`），入库 + `--check`。
- 前端所有边界（IPC 响应、WS 事件、HTTP、配置、localStorage）用 **Ajv** 按生成的 schema 校验；
  校验失败 → 结构化错误 + 上报，不进入业务逻辑。
- 禁止手写 zod 与 Rust 类型"双份维护"；若用 zod，必须由 schema 生成。
- 数值边界：金额用**整数微元**（JSON number），TS 侧 `Number.isSafeInteger` 断言；
  超过 `2^53-1` 一律拒绝（应改为字符串传输）。
- 时间统一 UTC ISO 字符串 + 显式时区，禁止本地时区隐式转换。

## 4. Rust：workspace 级最严 lints

```toml
# 根 Cargo.toml
[workspace.lints.rust]
unsafe_code = "forbid"
unused_must_use = "deny"
rust_2018_idioms = "deny"
non_ascii_idents = "deny"
missing_abi = "deny"

[workspace.lints.clippy]
all = "deny"
pedantic = "deny"
nursery = "deny"
cargo = "deny"
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
print_stdout = "deny"
print_stderr = "deny"
indexing_slicing = "deny"
arithmetic_side_effects = "deny"
as_conversions = "deny"
```

- 每个 crate `[lints] workspace = true`；测试模块可 `#[allow(clippy::unwrap_used, clippy::expect_used)]`。
- **Rust 手写代码零 `unsafe`**：产品 crate（`packages/`、`apps/`、`tools/`、`crates/`）每个
  `#![forbid(unsafe_code)]`，禁止手写 `unsafe` 块/函数/trait impl，禁止手写 `#[allow(unsafe_code)]`。
- FFI/WASM 导入的例外只能来自：① **自动生成绑定**（`@generated` 文件，生成器负责，单独审计）；
  ② 独立 `*_sys` crate（**不属于 workspace 产品 crate**，放 `vendor/` 或独立仓；必须最小、锁定、审计并在 ADR 登记）。业务/UI/服务端代码不允许出现 unsafe。
- 第三方依赖内的 unsafe 通过锁定版本 + `cargo-geiger` + `cargo-deny` 管理；**不接受为了性能/方便的例外**。
- 生成绑定/依赖的 unsafe 边界已决（ADR-040 / `AGENTS.md` C9b）：只允许**自动生成绑定**与独立 `*_sys` crate，
  禁止手写 unsafe；**不要求**"依赖树零 unsafe"（否则等于放弃 WASM 运行时/桌面壳）。
- release 保持 `panic = "unwind"`（cordis 要求）+ `overflow-checks = true`（已定）。
- 数值：金额/计数用 `checked_*`/`saturating_*`；禁止裸 `+ - *`（`arithmetic_side_effects`）；
  索引用 `.get()`（`indexing_slicing`）；整数转换用 `TryFrom`（`as_conversions`）。
- 错误：库用 `thiserror`，边界用 `anyhow`；禁止 `unwrap/expect/panic`。
- 并发：跨线程类型必须 `Send + Sync`；定时器/任务必须归 effect 所有（见 05/06）。

## 5. 契约一致性（两端防漂移）

| 检查 | 工具 | 失败即阻断 |
| --- | --- | --- |
| DTO → TS 类型 | schemars `export_schema` + `scripts/schema-ts.py` + `git diff --exit-code` | ✅ |
| DTO → JSON Schema | schemars + `--check` | ✅ |
| 示例数据 round-trip | Rust 序列化 → TS 反序列化 → 再序列化，字节一致 | ✅ |
| 枚举穷举 | TS `switch-exhaustiveness-check`；Rust `match` 穷举 | ✅ |
| 未知字段兼容 | 前端 schema 允许 unknown；未知枚举降级 Unknown | ✅ |
| 契约 API 破坏 | `cargo public-api diff` / `cargo-semver-checks` | UNVERIFIED（本机/CI 均未安装，见 §7.2） |

## 6. 测试增强（针对"技术性 bug"）

| 手段 | 用在 |
| --- | --- |
| 属性测试（proptest） | 帧编解码、reducer、金额计算、分页游标 |
| 模糊测试（cargo-fuzz） | WASM 帧解析、IPC JSON、配置解析 |
| golden traces | agent 事件流 → UI reducer 回放 |
| 变异测试（可选 cargo-mutants） | 核心 reducer / 计费公式 |
| 类型测试（expect-type） | 契约类型不可被错误使用 |
| 覆盖率下限 | 核心包（wallet/gateway/agent/reducer）≥80%，整体不设一刀切 |

## 7. CI 门禁（全部 0 容忍）

> **门禁真源**：本节是"当前门禁"的唯一清单。`AGENTS.md` §6 与 `docs/03` §6 只能引用本节，不得新增未在
> `.github/workflows/{ci,security,release}.yml` 实跑的条目。状态三态：**CI**（每次 push/PR 在 GitHub Actions
> 实跑）、**本地**（开发者机器可跑，工具缺失时按 UNVERIFIED）、**UNVERIFIED**（无执行位置，必须写明原因与
> 替代验证）。新增/删除 workflow step 必须同 PR 更新本表（spec R2，禁止 false-green）。
> 行号会随文件变动，review 时以 step 名称为准。

### 7.1 实际执行的门禁（逐条对应文件:行）

| 类别 | 门禁命令 | 实际执行位置 | 状态 |
| --- | --- | --- | --- |
| TS 类型 | `pnpm -r typecheck` | `.github/workflows/ci.yml:80`（web job） | CI |
| TS lint | `pnpm -r lint` | `.github/workflows/ci.yml:82` | CI |
| TS 单测 | `pnpm -r test:unit` | `.github/workflows/ci.yml:84` | CI |
| TS 构建 | `pnpm -r build` | `.github/workflows/ci.yml:86`（personal 基线）与 `:90`（enterprise 组合后） | CI |
| 浏览器 e2e | `pnpm e2e`（= `bash scripts/m1-e2e.sh`） | `.github/workflows/ci.yml:149` | CI |
| agent e2e | `bash scripts/agent-e2e.sh` | `.github/workflows/ci.yml:151` | CI |
| schedule e2e | `bash e2e/serve-schedule.sh` | `.github/workflows/ci.yml:153` | CI |
| 服务端集成 | `bash scripts/srv-auth-test.sh`（真 PG） | `.github/workflows/ci.yml:108` | CI |
| updates e2e | `bash scripts/updates-e2e.sh`（真 PG） | `.github/workflows/ci.yml:110` | CI |
| apps e2e | `bash scripts/apps-e2e.sh`（真 PG + wasip1 guest） | `.github/workflows/ci.yml:120` | CI |
| client-upgrade e2e | `bash scripts/client-upgrade-e2e.sh`（真 PG + serve 载体） | `.github/workflows/ci.yml:248` | CI |
| OEM e2e | `bash scripts/oem-e2e.sh` | `.github/workflows/ci.yml:221` | CI |
| a11y 矩阵 | axe（serious/critical = 0）：`scripts/login-e2e.sh:132,134`（m2-login / m3-client-content / m4-client-apps）与 `scripts/admin-e2e.sh:51`（m5-portal / m6-a11y） | `.github/workflows/ci.yml:199,201`（a11y job） | CI |
| Rust 格式 | `cargo fmt --all -- --check` | `.github/workflows/ci.yml:36` | CI |
| Rust clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `.github/workflows/ci.yml:53` | CI |
| Rust 测试 | `cargo test --workspace` | `.github/workflows/ci.yml:57` | CI |
| rustdoc | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | `.github/workflows/ci.yml:58-61` | CI（A1 修完 rustdoc 后转绿，之前如实红） |
| 生成物漂移 | `cargo test --workspace export_schema && git diff --exit-code && bash scripts/schema-check.sh` | `.github/workflows/ci.yml:49-52` | CI |
| 组合/索引门禁 | `zk validate` / `zk compose --check` / `zk capability --check` / `zk lint --duplicates` / `zk docs --check` / `zk db lock --check` | `.github/workflows/ci.yml:39-46` | CI |
| 文档门禁 | `python3 scripts/{docs,spec,review}-check.py`、`bash scripts/schema-check.sh` | `.github/workflows/ci.yml:263-266`（docs job） | CI |
| 安全审计前置 | PostgreSQL + `cargo fetch --locked` + `cargo build -p zk-server -p zk-desktop`（gates 2/3/7/12 的运行前置） | `.github/workflows/security.yml:58-78` | CI |
| 供应链（Rust） | `cargo deny check` / `cargo audit` / `cargo machete` | `.github/workflows/security.yml:79,81,83` | CI |
| unsafe 统计 | `cargo geiger --all-features`（输出存证据） | `.github/workflows/security.yml:87` | CI |
| 供应链（npm） | `pnpm audit --prod` | `.github/workflows/security.yml:95` | CI |
| 镜像扫描（安全 gate 10） | pinned trivy 0.76.1 扫描真实 server image，JSON 经 `ZEOKEEL_SECURITY_TRIVY_REPORT` 交给审计 | `.github/workflows/security.yml:100-143` | CI |
| 严格安全审计（13 项，5b 拆分单列） | `ZEOKEEL_SECURITY_STRICT=1 bash scripts/security-audit.sh`（前置齐备；`unverified_external`+`env_only` 不使 strict 失败；允许清单 `ZEOKEEL_SECURITY_KNOWN_GAPS` 为空） | `.github/workflows/security.yml:145-210`；审计语义见 `docs/21` §10、`scripts/security-audit.sh` | CI（可 exit 0；证据 JSON 如实记录 overall/unverified） |
| 安全 gate 5b「`mcp-stdio-transport`」（审计 JSON id=14）**已关闭** | B16 落地后 MCP 客户端传输实现（`cargo test -p zk-connector-mcp`），审计 gate 14 = `pass`；CI 允许清单已清空（`ZEOKEEL_SECURITY_KNOWN_GAPS=""`），如回归即 strict 红 | `scripts/security-audit.sh` gate14；本地最终证据 `target/evidence/release-1.0-rc1/security2/security.json`（HEAD `6d89107`） | 本地已通过；hosted runner 仍 UNVERIFIED |
| 版本/tag | tag `v<version>` == Cargo workspace == `package.json` == `tauri.conf.json` | `.github/workflows/release.yml:52-152`（version-check job，含 `:144` 的通过断言） | CI（tag 触发） |
| dist 真实性 | `bash scripts/build-dist.sh`（占位/fixture exit 1、缺真实输入 exit 2、`--self-test`） | `.github/workflows/release.yml:360-372`；`scripts/build-dist.sh:46-119`（自测）、`:213-360`（输入判定） | CI（release） |
| dist 验证 | `zk channel verify --dist <dist>` + `sha256sum -c SHA256SUMS` | `.github/workflows/release.yml:373-382`；`scripts/build-dist.sh:509` | CI（release） |
| GitHub Release | `gh release create`/`edit`/`upload` + 资产断言 | `.github/workflows/release.yml:567-615` | CI（release，tag 触发） |

### 7.2 UNVERIFIED（不得描述为"当前门禁"）

| 命令/门禁 | 原因 | 替代验证 / 出处 |
| --- | --- | --- |
| `cargo nextest run --workspace` | 本机与 CI 均未安装 nextest | `.github/workflows/ci.yml:57` 用 `cargo test --workspace`（同一 spec 集） |
| `cargo public-api diff` / `cargo-semver-checks` | 未安装且无 CI job；契约破坏检查未接线 | 契约 conformance 测试 + `cargo test --workspace`；§5 已改标 UNVERIFIED |
| `cargo geiger`（本地） | 本机未安装 | CI：`.github/workflows/security.yml:87`，报告作为 artifact 上传 |
| `cargo deny` / `cargo audit` / `cargo machete`（本地） | 本机未安装 | CI：`.github/workflows/security.yml:79,81,83` |
| `pnpm audit --prod`（本地） | 需要网络与 npm registry | CI：`.github/workflows/security.yml:95` |
| 安全门禁 env-only 项（gate 7 WORM 日锚定、gate 9 产物级 SBOM；gate 10 无 trivy report 时） | 无法由 CI 满足：WORM 外部锚定是部署方控制（`docs/20` §3；ADR-068 `accepted`，降级为部署控制）；产物 SBOM 只由 release dist 生成；CI 有 trivy report 时 gate 10 走 `pass/fail` | `scripts/security-audit.sh` 以 `record_env` 输出 `unverified_external` + `env_only: true`，strict 不失败且**从不记为 pass**（overall 不为 `pass`）；SSOT `docs/21` §10、`KNOWN-LIMITATIONS.md` §4；CI 证据 `target/evidence/release-1.0-rc1/security/security.json`（+ `security2/`） |
| 安全 gate 1 威胁模型评审 | 只有评审记录含精确行 `- 威胁模型评审结论：APPROVED` 且无 open P0 才算 pass；当前记录为 CHANGES-REQUESTED | 复评 APPROVED 前 strict 如实红（CI 仍执行其余 gate）；记录 `docs/reviews/2026-10-31-threat-model.md`，规则见 `scripts/security-audit.sh` gate1 |
| `cargo test --locked` / 变体锁矩阵 | `zk verify-lock` 未实现（`docs/03` §4 backlog） | UNVERIFIED；组合锁不在当前门禁 |
| 换包矩阵（swap matrix） | `zk diff` 未实现（`docs/03` §6.2） | UNVERIFIED；`docs/03` §6.2 已标注 |
| 生成物 diff 最小性 | 工具未实现（`docs/03` §6.3） | UNVERIFIED；`docs/03` §6.3 已标注 |
| bundle 预算 | 只有 `pnpm -r build` 成功；预算断言未实现 | `.github/workflows/ci.yml:86,90` 只断言 build 成功 |
| `zk api --check` | 未实现（`docs/24` §3 backlog） | 无；backlog |
| PR 三平台 `cargo check` 矩阵 | PR 只在 ubuntu 跑；macOS/Windows 仅在 tag 发布时构建 | `.github/workflows/release.yml:207-295`（installers matrix） |
| macOS 公证 / Windows 签名 | 证书后配（ADR-037，`release.yml:416-465` 的 gated job 默认跳过）；Windows 按决策不签名（ADR-033） | release body 显式 UNVERIFIED；无 `|| true` 吞失败 |

## 8. 例外政策

- 任何 `#[allow(...)]` / `eslint-disable` 必须**行内 + 理由 + issue 链接**，reviewer 才能通过；
  禁止文件级 blanket allow（生成代码除外）。
- 禁止为了过 CI 而放宽全局配置；确需放宽必须走 ADR。
- `unsafe_code` 的 `forbid` **不允许任何 allow**；若第三方 API 必须 unsafe 包装，改用提供安全封装的依赖，
  或把该能力隔离到独立进程/服务（不在本仓写 unsafe）。
- 新依赖必须过 `cargo-deny`/`pnpm audit`，并说明用途。

## 9. 与其它文档

- 客户端 UI 质量保障见 `11-testing-and-evidence.md` §8；
- 构建/工具链见 `10-repo-and-build.md`；
- 契约与边界见 `04-contracts-and-api.md`；
- ADR 记录见 `13-risks-and-adr.md` ADR-039。

## 10. 复用/去重检查（C17）

> 规则与门禁清单见 `25`；本节只列静态检查落点。

| 检查 | 机制 | 失败 |
| --- | --- | --- |
| 能力索引 | kit/契约每个 `pub` 项必须有 `@capability <id>` 注释；`zk capability --check` 与源码一致 | PR 红 |
| 禁止目录/文件 | 业务包内出现 `utils/`、`helpers/`、`common/`、`misc/`、`shared.ts`、`lib.ts`（kit 外） | `zk lint` 红 |
| import 边界 | 业务包只能 import 契约、`zk-kit-*`、`@zeokeel/ui-kit`、基线依赖；禁止 import 其它业务包实现 | `zk lint`/ESLint `no-restricted-imports` 红 |
| 重复代码（TS） | `jscpd --threshold 70 --min-lines 30`，与 `duplication-baseline.json` 比较（只降不升） | PR 红 |
| 重复代码（Rust） | `zk lint --duplicates`（token 规范化后比对），同一 baseline ratchet | PR 红 |
| 唯一 id | 路由/命令/`tool.<name>`/componentId/slotId/权限点重复 | `zk validate`/`zk compose --check` 红 |

- 不新增第二条"共享工具"路径：共享实现只能进 `crates/zk-kit-*` 或 `packages/ui/ui-kit`（`25` §2）。
- 例外必须 spec + ADR/D 批准并在能力索引登记；审查时按 `25` §4.3 的复用四问。
