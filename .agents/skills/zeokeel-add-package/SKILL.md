---
name: zeokeel-add-package
description: Use when adding, replacing, or removing a ZeoKeel package (Rust crate, UI package, service domain, or admin page). Enforces zk.toml v1, role/contract registration, edition wiring, conformance, and generated-artifact ownership.
---

# 新增 / 替换 ZeoKeel 包

## 何时使用

新增包、替换角色实现、拆分包、给包加 UI facet、把服务端域包化时。

## 先读

- `docs/02-package-model.md`：包/角色/契约语义
- `docs/24-composition-and-runtime-spec.md` §2：`zk.toml` v1 冻结字段
- `docs/03-composition-and-ci.md`：edition/profile/channel 叠层
- `docs/10-repo-and-build.md`：目录/workspace/生成物 owner
- `docs/18-strictness.md`：Rust/TS 严格检查
- `docs/25-reuse-and-dedup.md`：复用优先/单一实现；先查能力索引，避免造第二份实现

> 当前仓库处于 Plan 阶段，`packages/` 尚不存在；先确认 scaffold 已完成，不要假装目录存在。

## 决策清单（写进 design.md）

| 项 | 必须明确 |
| --- | --- |
| 包 id | `<vendor>/<name>`（本仓 `zeokeel/<name>`） |
| kind | 封闭枚举，见 `24` §2 |
| 角色 | 提供哪些 role；角色必须在 `24` §1.5 注册表 |
| 契约 | 依赖哪个 `zk-contract-*`；契约单实例；不依赖别的实现 |
| cardinality | `singleton`/`multi`；singleton 只能有一个提供者 |
| editions | `personal`/`enterprise`；personal-only 包登记到 `editions/personal-only.toml` |
| facet | rust / ui / assets；UI 必须有静态 `[[ui.contributions]]` |
| 生成物 | 哪些是 `@generated`，由哪个 writer 写 |
| 复用 | 是否已有包/kit/契约能扩展？为什么必须新建包？（`25` §2/§5） |

## 落地步骤

0. **先搜复用**：`zk find <能力>` / 读能力索引 / 检查现有包与 `crates/zk-kit-*`；能用现有包扩展就不新建包（`25`）。
1. 建/复用 change spec（`bash scripts/new-change.sh add-package-<name>`）。
2. 建目录：`packages/<group>/<name>/`；Rust crate 名 `zk-<name>`（服务端 `zk-srv-<name>`）。
3. 写 `zk.toml`：字段只按 `24` §2；未知键 = 构建失败。
4. 写 Rust：`#![forbid(unsafe_code)]`；只依赖契约；实现 `install()`；错误用封闭错误码。
5. 写 conformance：契约套件 + 本包负向用例；服务端包补迁移/jobs 测试。
6. UI facet：`web/` + 静态 contributions；slotId 必须来自 `16` §3 `SLOT_DEFS`。
7. 注册：edition/profile 的 `[packages]` 角色映射；不要改角色名。
8. 生成与校验：

```bash
zk validate
zk compose --edition personal --profile default --channel official --check
zk compose --edition enterprise --profile default --channel official --check
zk api --check
cargo nextest run -p <crate>
cargo clippy -p <crate> --all-targets -- -D warnings
# UI:
pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit
python3 scripts/docs-check.py
```

## 完成判据

- 角色/契约闭包通过；无重复 singleton；无第二份契约实例。
- `--check` 生成物与提交一致；没有手改生成物。
- conformance + 负向测试 + 文档更新（`docs/README.md` 主题、包 README）。
- `spec-check`/`docs-check` 0 error。

## 禁止

- 新增未注册角色、包名代替角色名、`requires` 写包名。
- 直接依赖别的包实现类型；运行期加载/热插拔。
- 手写平台 `#[tauri::command]`；UI 包互相 import 内部组件。
- 为通过检查放宽 lint 或改生成物。
- 新建包却复制现有包实现；在包内新建 `utils/helpers/common`；未登记 `@capability`。
