# 文档审计报告 B：构建期组合 / 渠道 / 交付 / 仓库构建

- 审计员：B（只读审计；仅写本文件）
- 快照时间：2026-10-30 14:10（Asia/Shanghai）
- 范围：`docs/03`、`09`、`10`、`12`、`16`、`18`、`configs/`、两个 `experiments/*/README.md`；对照 `AGENTS.md`、`PLAN`、`01`、`05`、`13`、`15`、`17`
- 并发说明：审计期间 `docs/` 在 14:01–14:09 被持续修改。14:07 快照曾报的 3 个 P0（09/12 缺 `exclude`、09/13 更新包 `.zip` 与 `tar.zst` 冲突、03 缺 channel 层）已在 14:07–14:09 的修改中修复；本报告按 14:10 重新核对，只列仍存在的问题。
- 结论：**P0 0 项、P1 1 项、P2 3 项**。edition × profile × channel、企业版 `(personal − personal-only) + 管控包`、更新通道、索引仓归属、包目录层级、命令面均已大体对齐；剩余问题只在 `03` 的组合单源措辞与 `09` 的交叉引用/登录地址细节。

## P0（0 项）

未发现仍存在的 P0。已核对：`AGENTS C1/C13/C14/C15` 对应的 `exclude`、`official` 渠道、`tar.zst` 更新链、索引仓自建在 03/09/10/12/13/PLAN 中已一致。

## P1（1 项：核心组合文档内部仍有“单源”旧表述）

| 文件:行号 | 问题 | 证据 | 修复建议 |
| --- | --- | --- | --- |
| docs/03-composition-and-ci.md:115（相关 50） | §3 已改为“edition × profile × channel 叠层”，但规则 1 仍写“`[packages]` 是唯一组合输入”，而同节又允许 edition `exclude`/`[packages]`、channel 注入品牌并“可带额外包”。实现者可能把包选择全部塞进 profile | `03:41-50`（三层图）；`03:64,115`；`03:86`；`09:30-32`；`10:98` | 把“唯一组合输入”限定为“同一层内的角色映射表”，并说明 edition/channel 的包增删如何进入同一解析结果 |

## P2（3 项：过期引用/行为细节）

| 文件:行号 | 问题 | 证据 | 修复建议 |
| --- | --- | --- | --- |
| docs/09-delivery-and-oem.md:41 | 引用“§1.4 的同一组常量”，但 09 没有 §1.4，只有 §1 与 §1.5 | `09:7`（§1）、`09:38`（§1.5）、`09:41` | 改为“§1 的同一组常量”或补 §1.4 标题 |
| docs/09-delivery-and-oem.md:308 | §7.3 登录地址只写“首次填写并保存”，未写“设置中可改、需重新登录”；而 09:127、01:114、05:31、AGENTS C14 均已定该行为 | `09:127`；`01:114`；`05:31`；`AGENTS C14` | §7.3 补“设置中可改，需重新登录”；OEM 非空仍保持不可编辑 |
| docs/09-delivery-and-oem.md:18 | OEM 行写“macOS 我方证书/客户自带另议（Q1）”，但 Q1 现在只覆盖 Windows 不签名与 macOS 公证，未定义证书主体归属 | `09:346`（Q1）；`13 ADR-037`；`17 D49` | 把“客户自带证书”移入 §9 新问题或删除 Q1 引用；明确 OEM macOS 签名主体 |

## 已核对一致（不作为问题）

- 版本模型：`personal` 基线；`enterprise = (personal − 个人专属包) + 企业管控包 + 角色覆盖`；03/01/10/13/PLAN/AGENTS 已一致（`base_edition` + `exclude`）。
- 渠道：官方企业版 = `enterprise × official`，`server_url` 可空；OEM 可预置非空；个人版仅 `official` 且无登录；03 已补 channel 层与 `--channel`。
- 更新：个人版 GitHub Releases；企业版服务端自建更新服务器 + 手动 `docker load`；企业版客户端只从登录服务端 1:1；09/13/12/PLAN 已统一 `tar.zst` + `CLIENT-RELEASE.json`。
- 索引仓：`zeokeel/channels` 仅作字段参考；索引仓自建，demo 为 `zeokeel/demo-oem-index`；09/15/17/AGENTS C15 一致。
- 仓库/构建：单一 workspace、三层 `packages/<group>/<name>`、`packages/*/*/web`、`default` 产物入库、`zk.lock` 与 `Cargo.lock` 分层；10/16/03/AGENTS 已一致。
- 严格检查：`configs/tsconfig.base.json`、`configs/eslint.config.js`、`configs/rust-workspace-lints.toml` 与 18 §1/§2/§4 一致；零 unsafe 的 `*_sys` 例外已在 18:135/10:158 说明。
- 实验 README：composition-poc 的 docs 路径、`external/acme-tool-fs`、resolve.py docstring 已修；wasm-poc 的尺寸口径已在 ADR-035 注明；`build.sh`/`golden/run.py` 路径一致。
- 路线图：M1 个人版 → M2 企业版，WASM 应用中心 M4 后置；M0–M6 人周合计 35；S0-P2 原型有 `experiments/wasm-poc` 证据。

## 必须修复清单（≤10）

1. **03:115（含 50）**：消除“唯一组合输入”与三层叠加/channel 额外包的矛盾。
2. **09:41**：修正不存在的“§1.4”引用。
3. **09:308**：补“设置中可改，需重新登录”，与 01/05/09:127/AGENTS C14 对齐。
4. **09:18**：删除或重指 Q1，明确 OEM macOS 签名主体。
