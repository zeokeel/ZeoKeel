# `specs/changes/` — 进行中的变更

- 每个变更一个目录：`YYYY-MM-DD-<kebab-slug>/`。
- 新建：`bash scripts/new-change.sh <slug>`。
- 完成后：`git mv specs/changes/<id> specs/archive/<id>`，并在 `proposal.md` 标 `done`。
- 已归档目录只读；后续变更另建新 id。

不要在这里放临时草稿；草稿也应先建 change 目录，避免"聊天里的决定"丢失。
