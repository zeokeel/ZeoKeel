# examples/wasm（M4 示例应用）

M4 的 Tier-1 Go 示例 guest 源码在 `experiments/wasm-poc/guest-go`（帧 ABI + `db.exec/db.query` + `log`），作为“数据应用”示例由 `scripts/apps-e2e.sh` 上传、审批、打开、写库、重启持久；静态 UI 示例由 e2e 的 `index.html` 注入 `zkAppRequest` 验证。

- 表单/数据两个示例共用同一帧 ABI；M4 先交付数据应用，表单 UI 为同一静态 UI 目录的扩展（M4.1 增加独立 forms guest）。
- 构建/验证：`experiments/wasm-poc/build.sh` → `python3 golden/run.py` → `bash scripts/apps-e2e.sh`。
