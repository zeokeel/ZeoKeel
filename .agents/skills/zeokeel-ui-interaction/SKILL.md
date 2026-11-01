---
name: zeokeel-ui-interaction
description: Use when adding or changing any ZeoKeel client UI interaction or new UI feature. Runs a cognitive walkthrough (3 personas × top tasks), writes intuition.md (INT mapping) + intuition-walkthrough.md, then the interaction contract (unique IDs), ID-tagged browser tests and evidence before code; e2e cannot prove intuition.
---

# ZeoKeel UI 交互闭环

## 何时使用

客户端 surface（页面/面板/弹窗/菜单/输入区）新增或修改交互行为/新功能时；包括键盘/焦点/IME、焦点归还、Esc 层叠、
错误提示层级、视觉/ARIA 基线、认知走查与用户测试证据。纯文案/纯样式微调可跳过，但仍要跑 `docs-check`。

## 读序（只读）

1. `AGENTS.md` C20 + §6/§8/§13（硬约束、门禁与证据）。
2. `docs/27` §2.7/§2.8（INT 判定、认知走查、用户测试，唯一权威）。
3. `docs/19` §11/§11.1（交互契约/ID 与 `intuition.md`/`intuition-walkthrough.md` 必填要求）。
4. `docs/27` §2.5/§2.6（唯一交互模式清单、视觉/ARIA 基线）。
5. `docs/11` §8.6（证据/门禁口径；直觉质量不由 gate 判定）。
6. 本次 `specs/changes/<id>/`（`spec.md` + 三件套）与 `specs/templates/{interaction,intuition,intuition-walkthrough}.md`。

## 步骤

0. **先走查（C20）**：按 `docs/27` §2.8 做 3 persona × 3 top task × 每步 4 问认知走查，写同目录 `intuition.md`
   （≥3 条 INT，每个 P0 交互 ID 映射 ≥1 条）与 `intuition-walkthrough.md`（persona/top task/摩擦表）；
   原则见 `docs/27` §2.7，文件要求见 `docs/19` §11.1。**P0 摩擦未解决不动手**；`planned` 契约可暂缺（gate 只 WARN）。
1. **契约先行**：先写/更新 `specs/changes/<id>/interaction.md`（`状态：active`；在途 spec 先 `planned`）。
   必填字段与 ID 规则按 `docs/19` §11；P0/P1 每条都要有测试落点与证据路径。先用
   `bash scripts/new-change.sh <slug>` 生成（自动复制三件模板），或复制 `specs/templates/*`。
2. **测试先行**：在 `e2e/interaction/` 加 Playwright 测试，带 `{ tag: ["@interaction:<ID>"] }`；
   只用 role/accessible name/label 定位，键盘用真实按键序列；同一 ID 至少一个测试。
3. **实现最小切片**：复用 `@zeokeel/ui-kit` 原语与 token；禁止第二套 focus/dialog/menu/toast（C17）。
4. **跑**：
   ```bash
   pnpm e2e:ui
   python3 scripts/ui-interaction-gate.py
   ```
   测试末尾写 `target/evidence/<change>/interaction/<ID>.json`；只有通过的测试才写证据。
5. **失败循环**：读 trace / screenshot / aria diff → 修 → 重跑。同一 ID 最多 3 次；仍不过先改契约/spec
   （可能测试暴露了设计缺口），不删断言/不跳过。
6. **收尾**：`interaction.md` 改 `active`（若原为 planned）；记录命令/证据路径到 acceptance；提交 PR 附快照。
7. **评审/用户测试**：fresh-context 非实现者按 3 persona 走查并核对 INT 覆盖（P0 摩擦未解决 → 不通过）；
   发布前 P0 旅程须有 ≥5 名真实用户 think-aloud 证据（`target/evidence/<change>/usability/`），未跑标 UNVERIFIED，
   不得声称"符合直觉"（`docs/11` §8.6、`docs/27` §2.8）。

## 命令

| 命令 | 用途 |
| --- | --- |
| `pnpm e2e:ui` | 跑交互契约套件（真实 `serve` 载体） |
| `pnpm e2e:ui:update` | **仅人工确认后**更新视觉/ARIA 基线 |
| `python3 scripts/ui-interaction-gate.py --self-test && python3 scripts/ui-interaction-gate.py` | 契约 ID ↔ 标签 ↔ 证据 + 反模式门禁 |
| `pnpm e2e` | 主 e2e；不跑 `e2e/interaction/`（testIgnore） |

## 证据与判据

- 证据：`target/evidence/<change>/interaction/<ID>.json`（`{id,title,at,passed}`，测试末尾覆盖写）。
- 完成判据：P0/P1 ID 全部有标签 + 通过测试 + 证据；gate exit 0；axe 0 serious/critical；视觉/ARIA 基线人工确认。
- fresh-context 评审：实现完成后由非实现者只读评审「契约 → 测试 → 证据」，记录 P0/P1 与结论（`docs/11` §9）。

## 禁止

- `waitForTimeout`、`force: true`、无 `@known-issue` 的 `test.skip`、CSS 选择器作主定位。
- 自动接受快照（CI 中 `-u`；agent 自行更新基线）。
- 用"手动点过/我保证"代替命令与证据。
- 在 `interaction.md` 复制 `docs/` 规范正文（只链接 SSOT）。
- 新增第二套 focus/trap/Toast/Dialog（C17）。
