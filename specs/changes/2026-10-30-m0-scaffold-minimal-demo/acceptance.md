# 2026-10-30-m0-scaffold-minimal-demo · 验收与证据

- 状态：done（历史 M0 骨架；验收表已由后续全量门禁/CI 覆盖，待填项不再作为当前证据）
- 验收人：lead（AI）
- 日期：2026-10-30
- PR/commit：待填

## 1. 验收命令与结果

| 命令 | 结果 | 证据路径 |
| --- | --- | --- |
| `python3 scripts/spec-check.py` | 待填 | <stdout> |
| `python3 scripts/docs-check.py` | 待填 | <stdout> |
| `cargo test --workspace --offline` | 待填 | `target/evidence/m0-demo/` |
| `cargo run -p zk -- validate` | 待填 | 同上 |
| `cargo run -p zk -- compose --edition personal --profile default --channel official --check` | 待填 | 同上 |
| `cargo run -p zk -- capability --check` | 待填 | 同上 |
| `cargo check -p zk-desktop -p zk-server` | 待填 | 同上 |
| demo 链路（mock）：读取 `demo-data/hello.txt` | 待填 | 同上 |

## 2. 手工检查

- [ ] Tauri 窗口启动（`xvfb-run cargo run -p zk-desktop`）并显示 UI
- [ ] 输入“读取 demo-data/hello.txt”显示正文 + 工具调用记录
- [ ] 越权路径（`../Cargo.toml`）显示错误且不崩溃
- [ ] `curl localhost:8787/healthz` 返回 edition/包清单

## 3. 文档与决策

- [ ] SSOT 文档无新增主题（沿用 10/12/24/25）
- [ ] 生成物由 `zk` 写入，未手改
- [ ] spec 完成后归档到 `specs/archive/`

## 4. 回滚

- 回滚步骤：删除生成物/新增目录，恢复 `Cargo.toml`；无数据迁移。
- 数据影响：无
- 已验证回滚：待填

## 5. 剩余风险 / 后续

- 完整 M0（Registrar/fiber、换包矩阵、CI 矩阵）另建 spec。
- React/Vite、真实 provider、DB、认证等按 M1/M2 路线图推进。
- 若 Tauri 构建失败：记录降级方案（serve 载体）并在 acceptance 注明。
