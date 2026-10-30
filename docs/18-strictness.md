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
| DTO → TS 类型 | ts-rs `export_bindings` + `git diff --exit-code` | ✅ |
| DTO → JSON Schema | schemars + `--check` | ✅ |
| 示例数据 round-trip | Rust 序列化 → TS 反序列化 → 再序列化，字节一致 | ✅ |
| 枚举穷举 | TS `switch-exhaustiveness-check`；Rust `match` 穷举 | ✅ |
| 未知字段兼容 | 前端 schema 允许 unknown；未知枚举降级 Unknown | ✅ |
| 契约 API 破坏 | `cargo public-api diff` / `cargo-semver-checks` | ✅ |

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

```bash
# TS
pnpm -r typecheck          # tsc -b
pnpm -r lint               # eslint --max-warnings 0
pnpm -r test:unit          # vitest（含 typecheck）
pnpm -r build              # vite build + bundle 预算
pnpm test:e2e              # playwright

# Rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo nextest run --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo deny check           # license/advisory/ban/source
cargo geiger --all-features # 依赖树 unsafe 统计（报告变化；新增高危需说明）
cargo machete              # 未使用依赖
cargo public-api diff      # 契约包

# 生成物
zk compose --edition personal --profile default --channel official --check
zk api --edition personal --profile default --channel official --check
zk docs --check
zk db lock --check
cargo test --workspace export_bindings && git diff --exit-code
cargo test --workspace export_schema && git diff --exit-code
```

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
