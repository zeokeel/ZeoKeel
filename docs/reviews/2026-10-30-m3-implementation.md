# 2026-10-30 · M3 内容平台实现对抗性验证

- **评审对象**：`2026-10-30-m3-content-platform` 实现（contracts/kit/server/admin/ui/desktop/scripts/e2e）
- **评审者**：audit-arch（独立只读）
- **结论**：CHANGES-REQUESTED
- **关联 spec**：`specs/changes/2026-10-30-m3-content-platform`
- 类型：C19 实现验收 / 独立只读对抗性审查（未改产品代码、spec、docs；本文件为唯一新增）
- 范围：`packages/contracts/{content,skill,connector}`、`crates/zk-kit-content`、`packages/server/{content,skills,connectors}`、`packages/admin/{admin-skills,admin-connectors}`、`packages/ui/{ui-skills,ui-connectors}`、`apps/desktop/src/ipc.rs` 的 `skills.install/remove/installed`、`apps/desktop/web/{store,oauth,App}`、`scripts/{srv-store-test,admin-e2e,login-e2e}.sh`、`e2e/m3-*.spec.ts`、docs/11/21、ADR-060/061、M3 spec。
- 结论：**CHANGES REQUIRED**

## 1. 结论先行

M3 的纵向链路已落地：contracts/kit/content/skills/connectors/admin/ui、真实 PG 迁移、上传/审批/授权/下载/回执/tombstone、connector 无 secret、OAuth PKCE 客户端流程、M3 admin/client e2e 都能跑通。**但不能 APPROVED**：

1. **P0 本地路径穿越/删除**：`safe_component` 允许 `.`/`..`，`skills.remove {name:"."}` 可删除全部已安装技能（实测，返回错误但数据已删）。
2. **P0 安全测试假绿**：`scripts/srv-store-test.sh` 的 connector“无 token/secret”断言实际失败（Python `AssertionError`），却因为 `[[ "$(python -c 'assert ...')" == "" ]]` 的空串比较仍然 exit 0、打印 `store ok`；该安全出口没有被真正验证。
3. **P1 构建/验收门禁不闭合**：`zk compose --edition enterprise --check` 报生成物差异；tasks/acceptance 依赖的 `zk api --check` 命令不存在；`admin-e2e.sh`/`login-e2e.sh` 整体 exit 1（M3 spec 本身通过，但脚本整体非绿）。
4. **P1 安全/正确性**：OAuth token 存 `localStorage`（docs/21 要求系统密钥库）；客户端 install 未校验归档 sha/tree；`is_safe_archive_path` 未拒绝 Windows drive/`:`；install receipt 可用任意 64 hex 通过；tree hash 单测只断言长度，未钉 golden。
5. **P2**：admin crate 名 `zk-admin-admin-*` 不符合 `zk-admin-<page>` 命名；UI/Rust conformance 仅检查 `zk.toml` 存在；tombstone 返回全局已删名称；M3 UI 无 store 单测（仅 OAuth 单测/e2e）。

## 2. 证据命令与结果

| 命令 | 结果 |
| --- | --- |
| `cargo test --workspace --offline` | **PASS**：93 passed / 0 failed；EXIT 0 |
| `pnpm --dir apps/desktop/web test` | **PASS**：11 files / 44 tests passed |
| `bash scripts/srv-store-test.sh` | **EXIT 0 但假绿**：日志含 `Traceback ... AssertionError`，随后 `store ok: content schema + skill archive + connectors`；见 P0-2 |
| `bash scripts/admin-e2e.sh` | **EXIT 1**：6 passed / 2 failed；M3 admin 两个 spec 通过，失败为 M2 admin 用例 |
| `bash scripts/login-e2e.sh` | **EXIT 1**：2 passed / 1 failed；`e2e/m3-client-content.spec.ts` 通过，失败为 M2 gateway chat |
| `./target/debug/zk validate` | **PASS**：`ok (63 packages scanned)`，enterprise 39 packages/41 roles |
| `./target/debug/zk compose --edition enterprise --profile default --channel official --check` | **FAIL**：`generated files differ: apps/desktop/src/generated/packages.rs, apps/desktop/web/src/generated/registry.ts, apps/server/src/generated/packages.rs, crates/zk-composed/{Cargo.toml,src/lib.rs}, docs/generated/PACKAGE-INDEX.md, shared/contracts/{packages,roles,ui-contributions}.json` |
| `./target/debug/zk api --edition enterprise --profile default --channel official --check` | **FAIL**：`zk: unknown command: api`（tasks/acceptance 依赖此命令） |
| `./target/debug/zk capability --check` | **PASS**：`up to date (13 entries)` |
| `bash scripts/schema-check.sh` | **PASS**：`schema-ts: up to date` / `schema-check ok` |
| `./target/debug/zk db lock --check` | **PASS**：`up to date (11 migrations)` |
| M3 admin e2e specs | `m3-admin-skills.spec.ts`、`m3-admin-connectors.spec.ts` 在 `admin-e2e.sh` 中通过（含 axe + 截图） |
| M3 client e2e spec | `m3-client-content.spec.ts` 在 `login-e2e.sh` 中通过（安装/删除/connector OAuth/toggle） |

## 3. P0（阻塞）

### P0-1 · `skills.remove` 本地路径穿越，可删除全部已安装技能

**证据**

- `apps/desktop/src/ipc.rs:258-259`：
  ```rust
  fn safe_component(value: &str) -> bool {
      !value.is_empty() && value.len() <= 128 && value.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
  }
  ```
  允许 `.`、`..`。
- `apps/desktop/src/ipc.rs:353-357`：`target = root.join(name)`；`remove_dir_all(&target)`。
- 实测（临时 `ZEOKEEL_HOME`，启动 serve，创建 `skills/demo/x` 后调用 IPC）：
  ```bash
  curl -b cookie -X POST http://127.0.0.1:8812/ipc \
    -H 'content-type: application/json' \
    -d '{"name":"skills.remove","payload":{"name":"."}}'
  # -> {"ok":false,"error":"skills remove: Invalid argument (os error 22)"}
  # 但 find "$H/skills" 已无 demo/x；目录内容被删除
  ```
  返回错误不代表未删除；删除发生在 `remove_dir_all` 的失败之前/过程中。
- 同类输入 `..` 也可构造 `root/..`；`install_skill` 对 name/version 使用同一 `safe_component`。

**修法**：`safe_component` 拒绝 `.`/`..`/空/前导点（至少 `value != "." && value != ".."`），并在 `install_skill`/`remove_skill`/server `CreateContent`/路径查询统一校验；补 IPC 负向测试（`.`、`..`、`...`、绝对/斜杠/反斜杠）。

### P0-2 · `srv-store-test.sh` 安全断言假绿（connector 无 secret/token 未真正验证）

**证据**

- `scripts/srv-store-test.sh:101`：
  ```bash
  [[ "$(printf '%s' "$CATALOG" | python3 -c '... assert "secret" not in str(d) and "token" not in str(d).lower()')" == "" ]]
  ```
  `catalog` 的 OAuth 元数据含 `token_url`（不是 token 本身），Python 断言**必然失败**；但失败时命令替换输出 `""`，外层 `[[ "" == "" ]]` 为真 → 测试通过。
- 实测日志：
  ```text
  Traceback (most recent call last):
    File "<string>", line 1, in <module>
  AssertionError
  store ok: content schema + skill archive + connectors
  ```
  `bash -x` 确认失败断言在 `:101`，脚本仍 exit 0。
- `:82`、`:90` 同样是 `[[ "$(python -c 'assert ...')" == "" ]]`；任何断言失败都会变空串并“通过”，不是只此一处。
- **影响**：M3 的“connector 不含 secret/token”安全出口、以及 `:82/:90` 的目录断言没有被真实验证；acceptance/成功指标会误报。

**修法**：不要在 `[[ ... == "" ]]` 里用 `python -c` 断言；改为 `python3 - <<'PY'` 显式退出/`set -e`，或让 Python 输出 `ok` 再比较 `ok`；补真正检查 `secret`/`token` 字段（排除合法的 `token_url`/`authorize_url`），并让失败 exit 非零。补 M3 安全负向测试（config 含 `api_key`/`secret`、catalog 不含 token、日志无 token）。

## 4. P1（应修）

### P1-1 · 构建/组合门禁不闭合：生成物差异 + `zk api` 不存在

- `zk compose --edition enterprise --check` exit 1，生成物差异列表见 §2；C11“生成物只能由生成器写、`--check` 绿”不成立。
- tasks/acceptance 写 `zk api --check`，但 `zk` 无 `api` 命令（实际报 `unknown command: api`）；该验收 gate 不可执行。
- 因此 M3 的 `apps/server` 组合根/registry/roles/packages 生成物无法证明与源同步；`zk validate` 通过不足以替代 `compose --check`。

**修法**：`zk compose` 重新生成并提交生成物，或修正生成器输入；实现/接入 `zk api --check` 或从 tasks/acceptance 删除该门禁并改可执行命令；CI 增加“`zk` 命令存在性”断言。

### P1-2 · OAuth token 存 `localStorage`，与 docs/21 系统密钥库要求冲突

- `apps/desktop/web/src/oauth.ts:100-135`：`const TOKEN_KEY = "zk.connector.oauth"`，`localStorage.setItem(TOKEN_KEY, JSON.stringify(store))`；access/refresh token 明文落在 WebView localStorage。
- `docs/21:76`：MCP OAuth token 存系统密钥库；`docs/09`/`docs/21` 要求 client secret/token 不出现在日志/存储；WebView XSS 可直接读取 localStorage。
- 虽服务端不存 token（ADR-060），客户端本地存储仍不满足密钥库目标。

**修法**：token 移到系统密钥库/Tauri secret service（或明确的受保护平台存储），localStorage 只存非敏感 metadata；补 XSS/日志脱敏测试；如 M3 不接受系统密钥库，需 ADR/docs/21 明确降级并列为 P1 风险。

### P1-3 · 客户端 install 未校验归档 sha/tree，receipt 可用任意 64 hex 通过

- `apps/desktop/web/src/store.ts:93-101` 只读 `X-ZeoKeel-Skill-Sha256` 与 catalog 比较（若 header 存在）；不重算下载 bytes sha；`skills.install` 只接 `archive_base64`，不接收/校验 catalog 的 archive sha。
- `apps/desktop/src/ipc.rs:334-342` 返回本地 `tree_sha256`；`App.tsx:209-217` 将 IPC 返回的 tree 直接作为 receipt；客户端未独立重算/对比。
- `scripts/srv-store-test.sh:83` 用 `"a"*64` 任意 tree 调用 install，返回 200；服务端不校验 tree（ADR-061 服务端无 tree），receipt 也不校验。
- M3 退出条件“tree_sha256 不匹配拒绝安装”没有实现/测试；只有 archive header 与 catalog 的弱检查。

**修法**：客户端解压到临时目录后独立计算 tree（TS 用同一 canonical algorithm，或复用 Rust IPC 后对比 bundle 的 expected tree）；install 前校验 archive sha；补篡改 archive/错误 tree/错误 receipt 负向测试；明确 receipt 仅运营事实不用于授权。

### P1-4 · `is_safe_archive_path` 未拒绝 Windows drive/冒号；路径规则不完整

- `crates/zk-kit-content/src/lib.rs:7-13` 只拒绝绝对 `/`、`\`、NUL、空段、`.`/`..`；`C:/evil`、`C:\evil`（反斜杠被拒但 `C:/` 过）、`C:evil` 不被拒绝。
- 服务端 `skills.rs::validate_archive` 使用该函数；zip 条目 `C:/x` 在 Windows 客户端可能成为 drive-qualified；spec §4 要求拒绝 Windows drive/backslash。
- 客户端 `install_skill` 用 `enclosed_name`（Linux 安全），但 Windows 平台仍依赖该检查；跨平台安全边界不一致。

**修法**：拒绝 drive prefix（`^[A-Za-z]:`）、UNC、`:`，统一 `path` 规范化；补 Windows/`C:`/`C:\`/UNC 测试；服务端和客户端共用同一 `zk-kit-content` 规则。

### P1-5 · 安全测试/门禁整体非绿，M3 evidence 有误报

- `admin-e2e.sh` exit 1（6 passed/2 failed，失败是 M2 admin，但脚本整体非绿）；`login-e2e.sh` exit 1（2 passed/1 failed，失败 M2 gateway，但脚本整体非绿）；acceptance `bash scripts/admin-e2e.sh` 不能按“全绿”填写。
- `srv-store-test.sh` exit 0 但内部 traceback（P0-2）。
- `e2e/m3-admin-*.spec.ts`/`m3-client-content.spec.ts` 本身通过，M3 的 UI 真实操作/axe 有证据；但整体脚本验收 gate 不成立。
- `pnpm --dir apps/desktop/web test` 44 tests 均为现有 app/oauth，未覆盖 `store.ts` 的 install/remove/tombstone/connector 状态（M3 UI e2e 才覆盖部分）。

**修法**：把 M3 e2e 从 M2 失败中隔离或修复 M2 回归，使脚本整体 exit 0；`srv-store-test` 失败即非零；补 `store.ts`/`ui-skills`/`ui-connectors` 单测（安装、删除、tree mismatch、can_install=false、OAuth refresh/expiry）。

## 5. P2（建议）

- `packages/admin/admin-skills/Cargo.toml:2`、`admin-connectors/Cargo.toml:2` crate 名为 `zk-admin-admin-skills`/`zk-admin-admin-connectors`；`docs/02`/`docs/08` 约定 `zk-admin-<page>`（`zk-admin-skills`）。C3 命名/可替换性受影响。
- `packages/ui/ui-skills/tests/conformance.rs:2-3`、`ui-connectors/tests/conformance.rs:2-3` 只断言 `zk.toml` 存在；不是实际 conformance；admin 页同样仅 manifest 测试。
- `crates/zk-kit-content/tests/conformance.rs:17-19` 只断言 tree hash 长度 64，未钉固定 golden 值；算法改动不会被发现。
- `scripts/srv-store-test.sh:82,90` 同 P0-2 的空串假绿；`:83` 用 `"a"*64` tree 未做负向。
- `packages/server/skills/src/lib.rs:225-228` tombstones 向任意已认证用户返回全部已删技能名（无 visible/grant 过滤）；建议按该用户曾安装/可见过滤或仅返回本地相关名。
- M3 admin/client e2e 是真实 UI 操作 + axe；但 `scripts/admin-e2e.sh` 混跑 M2 用例、`scripts/login-e2e.sh` 混跑 M2 gateway，M3 证据需要独立脚本/独立证据路径。
- M3 spec 允许的 `force_update`/低版本拒绝在 srv-store-test 中未覆盖；`e2e/m3-*` 未验证强制更新/回滚/配额边界。
- `install_skill` 未检查 SKILL.md/条目数上限/重复路径（服务端已检查，客户端 IPC 直接接收 base64）；建议客户端也校验并拒绝。
- OAuth `refreshOAuthToken` 已有单测，但 e2e 未覆盖 expiry→refresh→retry；consent state 校验在 `App.tsx:45-46` 已有，但无错误路径测试。

## 6. M3 退出条件核对

| 退出条件 | 结果 |
| --- | --- |
| 上传→审批→可见→安装→强制更新→删除自动移除 | **PARTIAL**：上传/审批/安装/删除/tombstone 有 e2e；强制更新/低版本拒绝未在 srv-store-test/e2e 覆盖；tree mismatch 未验证 |
| 非归属人不能更新；转交后权限 | **PASS（基本）**：srv-store-test 403/transfer 通过；无独立 UI 权限负向 |
| connector 无 secret/token；OAuth refresh | **FAIL/未验证**：srv-store-test 断言假绿；`oauth` 单测通过但 e2e expiry/refresh 未覆盖；token 存 localStorage |
| 审计完整链路 | **PARTIAL**：服务端有 audit 调用/事件；没有链/WORM 实测，srv-store-test 未断言审计表 |
| zip/路径/symlink/zip bomb/server-vs-client | **PARTIAL**：服务端只审元数据、客户端解压；zip path 函数缺 Windows drive；客户端 install 路径可穿越（P0-1） |
| C3/C4 包/契约/四件套 | **PARTIAL**：包/契约存在；admin crate 命名、UI/Rust conformance 浅、`zk compose --check` 红 |
| C10/C17 | **PARTIAL**：schema-check 通过；`@capability`/kit-content 存在；tree hash golden 弱 |
| UI 真实操作/a11y | **PASS（M3 spec 本身）**：M3 admin/client e2e 通过并含 axe；整体脚本 exit1 |
| 测试证据 | **FAIL**：P0-2 假绿、整体脚本 exit1、`zk api` 不存在、compose check 红 |

## 7. 结论

- **P0：2**（P0-1 本地路径穿越删除；P0-2 安全测试假绿）。
- **P1：5**（生成物/`zk api`、OAuth token 存储、客户端 install 校验、Windows drive 路径、脚本非绿/测试缺口）。
- **P2**：admin crate 命名、conformance 浅、tree hash golden 弱、tombstone 全量名称、强制更新/回滚/配额 e2e 缺失、客户端 SKILL.md/条目数校验缺失。
- **M3 实现不能 APPROVED**；先修 P0/P1，重新生成/提交 `zk compose` 产物，补独立 M3 e2e/安全负向，再由同一评审者复评。
- 本报告只读；未修改产品代码/spec/docs。

---

## 8. 第二轮复评（R2，2026-10-30）

> 对象：R1 修复后的实现。只读复跑；未改产品代码/spec/docs（仅本记录）。
> 仅列剩余 P0/P1。

### 8.1 结论

**CHANGES REQUIRED（CHANGES-REQUESTED）**

- **P0：0**。R1 P0-1/P0-2 已关闭。
- **P1：3 组剩余**：enterprise compose check 仍红、admin-e2e 整体仍非 0、OAuth token 仍 localStorage（与 docs/21 冲突）。
- **M3 实现仍不可 APPROVED**。

### 8.2 已关闭

- **P0-1 路径穿越：RESOLVED**：`apps/desktop/src/ipc.rs:258-266` 的 `safe_component` 现拒绝 `.`/`..`/前导点/`:`/斜杠；`:597-602` 增加 hostile-name 测试并断言 installed skills 未被删除；`cargo test -p zk-desktop` 通过。
- **P0-2 安全测试假绿：RESOLVED**：`scripts/srv-store-test.sh:82,90,101` 改为直接把 JSON 管道给 `python3 -c`，断言失败会使脚本非 0；connector 检查改为只断言 `access_token`/`refresh_token`/`client_secret` 不存在，合法 `token_url` 不再误报；实测 `EXIT=0` 且无 traceback，`store ok`。
- **P1 Windows drive/`: `RESOLVED`**：`crates/zk-kit-content/src/lib.rs:7-13` 增加 `path.contains(':')`；`:test` 覆盖 `C:/evil`、`C:evil`、`//server/share`；tree hash golden 已钉 `e38ea953...`。
- **P1 浏览器归档 SHA：RESOLVED**：`apps/desktop/web/src/store.ts:93-104` 下载后 `crypto.subtle.digest("SHA-256")` 重算并与 catalog 比较，不再只信响应头。
- **P1 `zk api` 不存在：RESOLVED（移除 gate）**：tasks/acceptance 已删除 `zk api --check` 依赖，改为 `zk validate`/`zk compose --check`；不再把不存在命令当验收。
- **P1 login-e2e：RESOLVED**：实测 `EXIT=0`，3 passed（M2 login、M2 gateway、M3 client content）。

### 8.3 剩余 P1

#### P1-1 · `zk compose --edition enterprise --check` 仍失败

实测（未改生成物）：

```text
$ ./target/debug/zk compose --edition enterprise --profile default --channel official --check
zk: generated files differ: apps/desktop/src/generated/packages.rs, apps/desktop/web/src/generated/registry.ts,
apps/server/src/generated/packages.rs, crates/zk-composed/Cargo.toml, crates/zk-composed/src/lib.rs,
docs/generated/PACKAGE-INDEX.md, shared/contracts/{packages,roles,ui-contributions}.json
EXIT=1
```

与“regenerated enterprise compose (check green)”声明不符。可能原因：`login-e2e.sh` cleanup 在结束时执行 `compose --edition personal`（`scripts/login-e2e.sh:13`），当前工作区生成物是 personal，随后 enterprise check 必然差异；若是此原因，需修 cleanup 不污染验收状态，或在验收前显式生成 enterprise 并检查。无论原因，**当前 exact 命令不绿**，C11/M3 gate 不能通过。

#### P1-2 · `admin-e2e.sh` 整体仍 exit 1

实测：

```text
$ bash scripts/admin-e2e.sh
...
7 passed (34.6s)
1 failed: e2e/m2-admin.spec.ts:44 jobs page can trigger reconcile
EXIT=1
```

M3 的 `m3-admin-skills.spec.ts`、`m3-admin-connectors.spec.ts` 均通过（含 axe）；但脚本整体非 0，acceptance 不能写“8 passed / admin-e2e ok”。失败为 M2 jobs 用例，可能是 trace/并发 flake，但仍需隔离 M3 或修复脚本使 M3 独立 gate 可靠。

#### P1-3 · OAuth token 仍 `localStorage`，与 docs/21 冲突

- `apps/desktop/web/src/oauth.ts:85-122`：`TOKEN_KEY="zk.connector.oauth"`；access/refresh token 明文存 `localStorage`。
- `docs/21:13/76`：MCP OAuth token 存系统密钥库；`docs/21:85` 本地密钥系统密钥库。
- 本轮明确“system keychain not yet available in this slice”未附 ADR/docs 降级记录，也没有替代受保护存储；仍属 P1 安全缺口，而非已接受设计。

### 8.4 其他证据（PASS）

| 命令 | 结果 |
| --- | --- |
| `cargo test --workspace --offline` | **PASS**：EXIT 0（干净重跑；此前一次失败是 login-e2e cleanup 并发改生成物导致，已复现说明是脚本隔离问题） |
| `cargo test -p zk-kit-content -p zk-desktop --offline` | **PASS**：hostile remove/path/kit 测试通过 |
| `bash scripts/srv-store-test.sh` | **PASS**：EXIT 0，无 traceback，`store ok` |
| `bash scripts/login-e2e.sh` | **PASS**：EXIT 0，3 passed |
| `pnpm --dir apps/desktop/web test` | **PASS**：44 passed |
| `zk validate` | **PASS**：63 packages scanned |
| `zk capability --check` | **PASS**：13 entries |
| `bash scripts/schema-check.sh` | **PASS** |
| `zk db lock --check` | **PASS**：11 migrations |
| `zk api --check` | 已从 M3 gate 移除；命令仍不存在，但不再作为证据 |

### 8.5 结论

- **P0：0；P1：3（compose check、admin-e2e 非绿、OAuth localStorage）**。
- **M3 实现仍不可 APPROVED**：修 P1-1/P1-2/P1-3 后复评。
- 关联 spec：`specs/changes/2026-10-30-m3-content-platform`。
- 复评结论（第二轮）：**CHANGES-REQUESTED**
