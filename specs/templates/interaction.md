# <surface> · 交互契约

> 本文件是**一次变更**的交互契约（`specs/changes/<id>/interaction.md`），不是长期规范。
> 长期规则见 `docs/19` §11、`docs/27` §2.5、`docs/11` §8.6；不要在这里复制定义。
> 交付要求：P0/P1 条目必须映射 `@interaction:<ID>` 测试与证据；没有测试/证据的条目不算完成。
> UI 变更还需同目录 `intuition.md`（≥3 条 INT，P0 交互 ID 各映射 ≥1）与 `intuition-walkthrough.md`（3 persona × 3 top task × 4 问）；见 `docs/19` §11.1、`docs/27` §2.7/§2.8、`docs/11` §8.6。

- 表面 / 入口：<composer / settings dialog / inspector / command palette / sessions / ...>
- 归属包：<@zeokeel/ui-chat / @zeokeel/ui-settings / ...>
- 权威文档：<docs/19 §X、docs/27 §Y、docs/28 §Z>
- 状态：`active`（参与门禁）| `planned`（实现开始时改 active）
- 证据根：`target/evidence/<change-id>/interaction/`
- 测试：`e2e/interaction/<surface>.spec.ts`（带 `@interaction:<ID>` 标签）

## 1. 元素与语义

| 元素 | role / accessible name | 键盘 | 默认焦点 | 备注 |
| --- | --- | --- | --- | --- |
| <输入框> | `textbox` / <aria-label> | Enter/Shift+Enter/Esc | 是 | |
| <按钮> | `button` / <名称> | Enter/Space | 否 | |

## 2. 状态模型

状态：`empty` / `loading` / `error` / `offline` / `streaming` / `cancelled` + `busy` / `disabled` / `composing` / `menu-open`（按适用）。

| 状态 | 进入条件 | 可见反馈 | aria（live/expanded/selected…） | 退出 |
| --- | --- | --- | --- | --- |
| empty | <...> | <空态 + CTA> | <...> | <...> |
| error | <...> | <内联/横幅/toast> | `role=alert` | <...> |

## 3. 交互条目（必须 1:1 映射测试）

| ID | 前置 | 人的动作（点击/按键/输入/拖放） | 期望（可见 + aria + 焦点 + 持久化） | 优先级 | 测试落点 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| `<SURFACE>-<AREA>-01` | <...> | <...> | <...> | P0 | `e2e/interaction/<surface>.spec.ts` | `<ID>.json` |
| `<SURFACE>-<AREA>-02` | <...> | <...> | <...> | P1 | 同上 | `<ID>.json` |

- ID 规则：`<SURFACE>-<AREA>-<NN>`（大写；AREA 如 `KEY`/`FOCUS`/`IME`/`A11Y`/`VIS`/`PERSIST`）。
- 优先级：P0 = 主路径/安全/数据一致性；P1 = 高频细节/可达性/回归基线；P2 = 可选增强（不进门禁）。
- 一个 ID 至少一个测试；一个测试可覆盖多个 ID（标签可多个），但不得引用未定义 ID。

## 4. 视觉 / ARIA 基线

| ID | 状态 | 主题 | 宽度 | 基线类型 | 基线文件 | 掩码/阈值 |
| --- | --- | --- | --- | --- | --- | --- |
| `<SURFACE>-VIS-01` | <empty> | light | 1280 | `toHaveScreenshot` | `e2e/interaction/__snapshots__/...png` | `maxDiffPixelRatio=0.002` |
| `<SURFACE>-ARIA-01` | <empty> | — | 1280 | `toMatchAriaSnapshot` | `e2e/interaction/...-snapshots/<name>.aria.yml` | 动态项用正则/部分匹配 |

## 5. 预算与一致性

- 交互延迟：输入 <16ms；面板切换 <100ms（超过则显示 busy）。
- 复用：只用 ui-kit 的 Button/Dialog/Menu/Tooltip/Toast/Empty/Error；禁止第二套 focus/toast/dialog/menu（C17）。
- token：颜色/圆角/间距/动效只用 CSS vars；禁止 raw hex / magic px。
- i18n：所有文案来自字典（zh/en）；禁止硬编码中文/英文。
- 六态：每个交互组件必须覆盖；缺一即不通过评审。
- 视觉更新：只允许人确认后 `pnpm e2e:ui:update`；CI 不得自动接受。
