# CI 优化报告（2026-11-02）

> 范围：`.github/workflows/{ci,security,release,build-desktop}.yml`、`.github/actions/setup/action.yml`、
> 相关 scripts 与 docs/03/09/11/18/37。只读分析，未修改 workflow；结论均为"建议"，实施前按 C16/C19
> 走 spec + 评审（CI 是门禁真源，`docs/18` 的 gate 口径不能静默改）。
> 数据来源（GitHub Actions API 实测）：
> - `ci` run `41270910142`（2026-11-01T03:53Z，success）
> - `security` run `41270910137`（同时间，success，16m13s）
> - `release` run `41267556243`（2026-11-01T03:02Z，success，37m32s，v1.0.0-rc.1）
> - `release` run `41326919299`（v1.0.0-rc.2，macOS job 排队 15m 后 cancelled，见 §2.3）
> 观测值是"缓存命中"的 run；冷缓存会显著更长，收益按相对值看。

## 1. 现状盘点

### 1.1 触发器与工作流

| workflow | 触发 | job 数 | 观测耗时（wall） | 说明 |
|---|---|---|---|---|
| ci | push main / PR→main | 9（本地分支含 `ui-interaction` 共 10） | ~10m | rust、web、integration、e2e、ui-interaction、a11y、e2e-oem、e2e-client-upgrade、docs、docker；全部 `ubuntu-24.04` |
| security | push main / PR→main | 1（hardening） | 16m13s | toolchain + PG + 4 个 cargo 审计工具 + 镜像 buildx + trivy + strict audit |
| release | push tag `v*` / workflow_dispatch | 9 | 37m32s | version gate + 6 个桌面构建（2 edition × 3 平台）+ GitHub Release + enterprise image |
| build-desktop | reusable | 1 job ×6 | 8–22m/job | 每个 job 独立 checkout/setup/pnpm/web build/tauri build |

### 1.2 每次 main push 的 runner 分钟（观测，cache-warm）

`ci` run 41270910142 各 job 时长（分钟）：docs 0.1、e2e-oem 0.35、web 2.4、rust 3.8、e2e 4.2、
integration 4.5、a11y 5.8、e2e-client-upgrade 2.3、docker 10.0 → **合计 ≈33.6 min**；
加上 security hardening **16.2 min** → **≈49.8 min/push**（合并 `ui-interaction` 后约 +4 min ≈54 min）。

### 1.3 每次 release tag 的 runner 分钟（观测）

rc.1 release 各 job（分钟）：enterprise linux 14.2、enterprise macos 10.5、personal windows 14.2、
enterprise windows 22.3、personal macos 8.4、personal linux 9.7、personal release 0.6、enterprise image 13.7
→ **合计 ≈93.6 raw min/tag**。按 GitHub 标准 runner 计费倍率（macOS ×10、Windows ×2）折算约 **≈300 计费分钟/tag**
（本仓库为 public，标准 runner 额度免费，但 macOS/Windows 共享容量与 wall-clock 仍是真实成本）。

### 1.4 已做得好的部分（不要动）

- 触发面窄：ci/security 只在 `main` push + PR；tag 只触发 release（[ci.yml:9-14](../.github/workflows/ci.yml#L9-L14)）。
- `concurrency` + `cancel-in-progress` 已配置（[ci.yml:19-21](../.github/workflows/ci.yml#L19-L21)）；release 不取消历史构建。
- `Swatinem/rust-cache`、pnpm/Node cache 已在 setup 统一（[setup/action.yml:42-60](../.github/actions/setup/action.yml#L42-L60)）。
- build-desktop 已是 reusable workflow，避免 6 份 workflow 复制。
- 版本门、compose/capability/docs 漂移门、export_schema 幂等都集中在 ci，口径集中。

## 2. 主要浪费点（按影响排序）

### P0-1 同一个 server 镜像每次 push 构建两遍（≈10–12 min/push）

- `ci` 的 `docker` job 用 **plain `docker build`**（[ci.yml:323](../.github/workflows/ci.yml#L323)），无 buildx 缓存；
  随后 security 的 hardening 又用 `docker/build-push-action` 构建**同一镜像**（[security.yml:122-133](../.github/workflows/security.yml#L122-L133)）。
- 观测：ci docker job 10.0 min（03:53:12→04:03:14）；security 镜像构建 12.0 min（03:55:39→04:07:39）。
  两者 cache scope 还不一致（security 用 `security-server-image`），互不命中。
- 建议：把镜像构建抽成 reusable job（buildx + `cache-to/from type=gha,scope=server-image` + `load:true`
  或 `docker save` artifact），ci/security 共用；至少把 ci 的 `docker build` 换成 buildx 并统一 cache scope。
  预计 **−8…−10 min/push**。

### P0-2 release 的桌面构建矩阵重复 setup，且 macOS 两个 job 排队被取消（≈2 min setup ×6 + 队列风险）

- 6 个矩阵 job 各自执行 `pnpm install` + `pnpm --dir apps/desktop/web build` + `pnpm dlx tauri icon/build`
  （[build-desktop.yml:60-112](../.github/workflows/build-desktop.yml#L60-L112)），personal/enterprise 各 3 平台完全独立。
- rc.2 的两个 macOS job 在 `runner_id=0`、`steps=[]` 情况下排队 15 分钟后被 GitHub 取消两次
  （run `41326919299`）：不是编译失败，是 macOS runner 未被分配；两个 macOS job 同时排队放大了风险。
- 建议（分两步）：
  1. **只把 macOS 的两个 edition 合并成一个 job**（同一 macOS runner 顺序构建 personal/enterprise，产物两个
     artifact），减少一个 macOS 队列名额、少一次 setup；Linux/Windows 保持并行。macOS 关键路径 8.4+10.5≈19 min，
     仍在 Windows 22 min 之下，wall 时间基本不变。
  2. 给 macOS job 加"失败/取消可继续发布"策略（需与 docs/18/37 门禁口径一起改，不能静默 false-green）：
     macOS 缺失时仍发布 Linux/Windows 并在 release 说明标 macOS UNVERIFIED（本次 rc.2 手工发布即此状态）。
- 另外：`version-check` 在构建前先 `gh release delete`（[release.yml:82-86](../.github/workflows/release.yml#L82-L86)），
  一旦后续 job 失败会把已有 release 删掉（rc.2 的手工 release 会被误删）。建议"构建成功后再原子更新 release"，
  或先建 draft/staging release。
- 预计 **−2 min setup/edition 合并、−1 个 macOS 队列名额、消除"release 先删后失败"风险**。

### P0-3 security 全量跑在每个 PR + main push（≈16 min/push）

- 单个 hardening job 串行执行：apt/PG、4 个 `cargo install`、enterprise compose、`cargo build`、cargo-deny/audit、
  machete、geiger、pnpm audit、buildx 镜像、trivy、full `scripts/security-audit.sh`（[security.yml:52-233](../.github/workflows/security.yml#L52-L233)）。
- 观测：setup 48s、镜像 12m、strict audit 1.3m、其余 <1m（rust-cache 命中时工具安装 2s；冷缓存时 `cargo install`
  4 个工具会明显更久）。在 PR 上每次跑 16 min，而其中约 12 min 是镜像构建（与 P0-1 重叠）。
- 建议拆 `security-fast`（cargo-deny + cargo-audit + pnpm audit + secret scan，<2 min）+ `security-deep`
  （geiger/trivy/image/full audit），后者只在 main push、nightly、以及 `Cargo.lock`/`packages/**`/`deploy/docker/**`
  变更时跑；PR 保留 fast。需要同步改 `docs/18` §7 / `docs/37` 的 gate 口径与分支保护 required checks。
- 预计 **PR 每次 −10…−14 min**；main push 基本持平但可利用 P0-1 缓存降到 −10 min。

### P1-1 每个 e2e/a11y/ui job 重复安装 Playwright 浏览器与系统依赖（4 × ~0.5–1 min）

- `playwright install --with-deps chromium` 在 e2e、ui-interaction、a11y、e2e-client-upgrade 各跑一次
  （[ci.yml:166](../.github/workflows/ci.yml#L166)、[197](../.github/workflows/ci.yml#L197)、[226](../.github/workflows/ci.yml#L226)、[276](../.github/workflows/ci.yml#L276)）。
- 建议缓存 `~/.cache/ms-playwright`，key 用 playwright 版本（`pnpm-lock.yaml` hash）；`--with-deps` 的 apt 部分可保留。
  预计 **−2…−3 min/push**，且减少外网下载失败面。

### P1-2 artifact 上传全量 evidence，默认保留 90 天

- `e2e`、`ui-interaction`、`a11y`、`integration`、`docker` 等每 run 上传 `target/evidence`/多目录
  （[ci.yml:144](../.github/workflows/ci.yml#L144)、[173](../.github/workflows/ci.yml#L173)、[202](../.github/workflows/ci.yml#L202)、[241](../.github/workflows/ci.yml#L241)、[285](../.github/workflows/ci.yml#L285)）。
- 建议：大体积 evidence 仅失败时上传（`if: failure()`），或 `retention-days: 3/7`；gate 必需的 evidence
  （ui-interaction JSON）保留 7 天即可。预计存储/带宽明显下降，逻辑零风险。

### P1-3 冷启动工具链与 Docker 依赖层未做缓存/替换

- security 的 `cargo install` 4 个工具：`cargo-deny 0.20.2`、`cargo-audit 0.22.2`、`cargo-machete 0.9.2`、
  `cargo-geiger 0.14.0`（[security.yml:75-78](../.github/workflows/security.yml#L75-L78)）。rust-cache 命中时 2s，
  冷缓存/缓存失效时需要编译安装，可能 5–10 min。
  → 替换为 `taiki-e/install-action`（有预编译）或显式缓存 `~/.cargo/bin`（key 含四工具版本）。
- `Dockerfile.server` 每次 `COPY packages/...` 后从零 `cargo build --release`（[Dockerfile.server:12-21](deploy/docker/Dockerfile.server#L12-L21)），
  BuildKit 层缓存对源码变化无效。→ 用 `cargo-chef` 或"先 COPY manifests + 预热依赖层"的 Dockerfile 重排，
  依赖层命中时镜像构建可从 ~12 min 降到 ~3–5 min。
- release 各 job 用 `pnpm dlx @tauri-apps/cli@2`（[build-desktop.yml:71](../.github/workflows/build-desktop.yml#L71)、[104](../.github/workflows/build-desktop.yml#L104)），
  每次都下载 CLI；建议把 `@tauri-apps/cli` 固定为 devDependency（lockfile 锁定，pnpm store 缓存命中）。

### P1-4 同一份 web dist 被重复构建（每个 desktop job + ci rust + security）

- `ci` 的 rust job 为 Tauri `frontendDist` 先 build 一次（[ci.yml:44-45](../.github/workflows/ci.yml#L44-L45)）；
  web job 也 build（personal + enterprise 各一次，[ci.yml:97-101](../.github/workflows/ci.yml#L97-L101)）；
  security 再 build 一次（[security.yml:94-95](../.github/workflows/security.yml#L94-L95)）；release 6 个 job 各 build 一次。
- 可共享的是 release 的 6 次：在 version-check 后加一个 `web-dist` job 构建一次并上传 artifact，6 个平台 job 下载
  直接用（注意 edition 差异：web dist 是 edition-agnostic 的 shell？当前 build-desktop 先 compose edition 再 build web，
  web 产物可能含 edition 相关 registry，需先验证；若含 edition 差异则只能 personal/enterprise 各 build 一次）。
- 预计 release **−4…6 min raw**；若要动需先验证 web dist 的 edition 相关性。

### P2-1 e2e 串行（workers=1）是刻意设计，不建议用并行省时间

- `playwright.config.ts` 与 `playwright.interaction.config.ts` 都是 `workers: 1, retries: 0, fullyParallel: false`
  （[playwright.config.ts:11-13](../playwright.config.ts#L11-L13)、[playwright.interaction.config.ts:31-33](../playwright.interaction.config.ts#L31-L33)），
  这是 docs/11 §8.6 的确定性口径；提高 workers 省 wall 但会增加内存、并发 flake 与资源，不是"省资源"的方向。
- 如要缩短 PR 反馈，用 **按 spec 文件分片 job**（每 job 仍 workers=1），而不是改 workers。

### P2-2 文档/schema 检查与 rust job 重叠、`cargo doc` 每 PR 跑

- `docs` job 只跑 docs/spec/review/schema-check（7s），与 rust job 的生成物检查部分重叠；
  `cargo doc --no-deps` 在每个 PR 的 rust job 里跑（[ci.yml:70-72](../.github/workflows/ci.yml#L70-L72)）。
- 建议：`cargo doc` 移到 main/nightly（文档构建不是 PR 阻断项），`docs` job 保留（7s 很便宜，不建议合并掉，
  否则 checkout/setup 成本反而更高）。

### P2-3 main push 与 PR 双跑（合并后的第二次全量）

- PR 绿后合并到 main 会再跑一次 ci+security。这是"合并后真源"门禁，不建议用 `[skip ci]` 规避；
  可考虑 GitHub Merge Queue（合并队列只跑一次）+ 把 main push 的 security-deep 放 nightly；属于分支策略决策。

### P2-4 多个 job 用 apt 安装 PostgreSQL

- integration、a11y、e2e-client-upgrade 各自 `apt-get install postgresql` + `systemctl start`
  （[ci.yml:110-115](../.github/workflows/ci.yml#L110-L115)、[217-222](../.github/workflows/ci.yml#L217-L222)、[269-274](../.github/workflows/ci.yml#L269-L274)）。
- 改为 GitHub Actions `services: postgres:16`（health-check + 环境变量）可省每次 apt ~30–60s，且版本固定可复现；
  需要这些脚本支持非 peer/自定义端口连接（当前用 `sudo -u postgres` peer auth），属于脚本改动，风险中等。

### P2-5 `e2e-oem`/`e2e-client-upgrade` 的独立 setup 成本

- `e2e-oem` 只有 ~0.35 min，但独立 job 仍有 checkout/setup 固定成本；可以并入 integration 末尾（共享 setup）。
  收益小（~1 min），优先级低。

## 3. 建议的目标结构

| 事件 | 跑什么 | 目的 |
|---|---|---|
| PR | docs/spec/review + rust（fmt/clippy/test，含 docs？否）+ web + security-fast + e2e smoke（m1 mock/serve 子集）+ ui-interaction | 快速反馈，<15 min |
| main push | 全量 ci（含完整 e2e/a11y/integration/docker，镜像用共享 cache）+ security-fast | 合并后真源 |
| nightly / 手动 | security-deep（geiger/trivy/full audit/镜像）+ 完整 a11y/e2e + 冷缓存验证 | 重门禁不占 PR 时间 |
| tag `v*` | release（macOS edition 合并 job；构建成功后原子更新 release；macOS 缺失可降级并标 UNVERIFIED） | 发布 |

配套：
- 一个 reusable `build-server-image`（buildx + `type=gha` cache + 镜像 artifact）供 ci/security/release 复用。
- 一个 reusable `web-dist`（若验证 edition 无关）供 release 6 个 job 复用。
- 统一 `Swatinem/rust-cache` 的 `shared-key`（跨 job 共享 cargo target）——需实测并发写 cache 的收益/冲突。

## 4. 具体改动清单（按优先级）

| # | 优先级 | 文件/位置 | 动作 | 预计节省 | 风险 |
|---|---|---|---|---|---|
| 1 | P0 | [ci.yml:323](../.github/workflows/ci.yml#L323) + [security.yml:122](../.github/workflows/security.yml#L122) | 镜像构建统一为 buildx + 同一 `type=gha` cache scope；或抽 reusable job + artifact | 8–10 min/push | 低（仅构建方式） |
| 2 | P0 | [release.yml:96,115](../.github/workflows/release.yml#L96) | macOS personal/enterprise 合并为一个 job（顺序构建，2 个 artifact） | 1 个 macOS 队列名额 + ~2 min setup；降低取消概率 | 中（矩阵/依赖需改，release 校验逻辑不变） |
| 3 | P0 | [release.yml:82-86](../.github/workflows/release.yml#L82-L86) | 构建成功后再创建/更新 release；不先 delete | 避免失败删除已有 release | 低 |
| 4 | P1 | [security.yml:52-233](../.github/workflows/security.yml#L52-L233) | 拆 security-fast/security-deep；deep 只在 main/nightly/依赖变更时跑 | PR 10–14 min | 中（需改 docs/18 门禁口径与分支保护） |
| 5 | P1 | ci 4 个 e2e/a11y job | 缓存 `~/.cache/ms-playwright`（key=playwright 版本） | 2–3 min/push | 低 |
| 6 | P1 | [security.yml:75-78](../.github/workflows/security.yml#L75-L78) | `taiki-e/install-action` 或缓存 `~/.cargo/bin`（key 含版本） | 冷缓存 5–8 min | 低 |
| 7 | P1 | [Dockerfile.server:12-21](deploy/docker/Dockerfile.server#L12-L21) | cargo-chef / 依赖层重排 | 镜像 12→3–5 min | 中（Dockerfile 改动，需本地 build 验证） |
| 8 | P1 | ci e2e/ui-interaction/a11y/integration 上传 | 失败才上传大 evidence；`retention-days: 3/7` | 存储/带宽 | 低 |
| 9 | P1 | release build-desktop | `@tauri-apps/cli` 固定 devDependency；web dist 共享（先验证 edition 无关） | 4–6 min/tag | 中（web dist 可能 edition 相关） |
| 10 | P2 | [ci.yml:70-72](../.github/workflows/ci.yml#L70-L72) | `cargo doc` 移到 main/nightly | ~1–2 min/PR | 低 |
| 11 | P2 | [ci.yml:110](../.github/workflows/ci.yml#L110) 等 3 处 | 用 `services: postgres:16` 替代 apt | 0.5–1 min/job | 中（脚本连接方式） |
| 12 | P2 | 全 workflow | 给 heavy job 加 `dorny/paths-filter` 的 job 内 early-exit（不是 job 级 skip，避免 required check 变 skipped） | docs-only PR ~50→~2 min | 中（分支保护语义） |

## 5. 预计收益

- 每次 main push：**≈50 min → ≈30–35 min（−30%…−40%）**（镜像去重 −10、security 拆分后 PR 另算、
  Playwright 缓存 −2–3、其他 −1–2）。
- 每次 release tag：raw ≈94 → ≈70–75 min，且 **macOS 队列名额减半**；计费加权约 −20%…−25%。
- PR 反馈：security 从 16 min → <2 min（快速路径），docs-only 从 ~50 min → ~2 min（若采用 #12）。
- 失败面：镜像/浏览器/CLI 的外网下载点减少，macOS/Windows 队列失败概率下降。

## 6. 不建议做的事（防止 false green）

- 不要删除/跳过 `docs/18` 声明的 gate（rust/clippy/test/export_schema/compose/capability/docs/schema/e2e/security），
  不要用 `|| true`、`continue-on-error` 掩盖真实失败；security 的 strict 语义（只允许文档化 known gap）保持。
- 不要把 e2e `workers` 改成 >1（docs/11 §8.6 确定性口径）；要快就用分片。
- 不要自动接受视觉/ARIA 基线；不要用"CI 绿"替代 C20 用户测试。
- 不要在没有更新 docs/18/37 与分支保护配置的情况下改变 required checks/触发面。

## 7. 实施顺序建议

1. **基线测量**：记录当前 main push 的 ci+security runner 分钟（本文数据）；
2. **Phase 1（低风险，单独 PR）**：Playwright 缓存、artifact retention、release 不先删、镜像 buildx 缓存统一；
3. **Phase 2（结构）**：security 拆 fast/deep、镜像 reusable、release macOS 合并、Dockerfile 依赖层；
4. **Phase 3（策略）**：paths early-exit、nightly 调度、分支保护 required checks、docs/18/37 回写；
5. 每阶段用同一 run 对比 runner 分钟与 wall time，任何 gate 数量/口径变化都写进 docs/11/18。

## 8. 附注：分支差异

本次盘点以当前工作树（`release/1.0.0-rc.1`，含 `ui-interaction` job）为准；远端 `main`（`6a6d24a`）的
`ci.yml` 还没有 `ui-interaction`（28 个提交差）。合并后 main 的 CI 会多一个 ~4 min 的交互门禁 job，
预算按 §1.2 的 ≈54 min 起算；优化落地也需要同步到 `main`。
