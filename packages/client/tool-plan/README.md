# zk-tool-plan

Plan mode (`/plan`) submission tool and workspace plan store.

- Role `tool.plan` (contract `zk-contract-tool`, singleton, personal+enterprise).
- `exit_plan_mode` is the only submission entry: validates a non-empty Markdown plan
  (≤64 KiB), requires plan mode active for the conversation, writes
  `<workspace>/.zeokeel/plans.json` entry `status=pending_review` + `through_turn`,
  and returns `{"status":"pending_review","plan_path":...}`. It never executes the
  plan and never leaves plan mode.
- `plans` module owns the file format (atomic write, 64 KiB bound); the desktop host
  uses it through the generated `zk_composed::zk_tool_plan` facade for
  `plan.get`/`plan.update`, so there is one implementation (C17).
- Tool availability in plan mode is enforced by agent-rig (read-only + `exit_plan_mode`;
  mutating/delegation/schedule/MCP blocked), not by this package or by prompts.
