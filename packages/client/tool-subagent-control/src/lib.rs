//! Subagent control tools (list/status/interrupt async children).
#![forbid(unsafe_code)]
use std::sync::Arc;

use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

/// `list_agents` tool (sync + async registry).
pub struct SubagentControlTool;
#[must_use]
pub fn tool() -> SubagentControlTool {
    SubagentControlTool
}

/// `subagent_status` tool.
pub struct SubagentStatusTool;
#[must_use]
pub fn status_tool() -> SubagentStatusTool {
    SubagentStatusTool
}

/// `subagent_interrupt` tool.
pub struct SubagentInterruptTool;
#[must_use]
pub fn interrupt_tool() -> SubagentInterruptTool {
    SubagentInterruptTool
}

/// Extra tools contributed by this package (composed after `list_agents`).
#[must_use]
pub fn extra_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(SubagentStatusTool),
        Arc::new(SubagentInterruptTool),
    ]
}

impl Tool for SubagentControlTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "list_agents".to_owned(),
            description:
                "List current subagent runs (sync + async children) and delegation limits."
                    .to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{},"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "list_agents takes no arguments".to_owned(),
        ))
    }
    fn call_json(&self, _c: &TurnContext, _a: serde_json::Value) -> Result<String, ToolError> {
        let agents = zk_kit_subagent::list();
        let runner = zk_kit_subagent::runner();
        let async_ids: Vec<zk_kit_subagent::AsyncAgentStatus> = agents
            .iter()
            .filter_map(|agent| runner.as_ref().and_then(|runner| runner.status(&agent.id)))
            .collect();
        Ok(serde_json::json!({
            "agents": agents,
            "async": async_ids,
            "max_depth": zk_kit_subagent::MAX_DEPTH,
            "max_async": zk_kit_subagent::MAX_ASYNC_AGENTS,
            "note": "subagent_status/subagent_interrupt available for async children"
        })
        .to_string())
    }
}

impl Tool for SubagentStatusTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "subagent_status".to_owned(),
            description: "Show status and final result of one background subagent.".to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"id":{"type":"string"}},"required":["id"],"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "subagent_status requires JSON".to_owned(),
        ))
    }
    fn call_json(&self, _c: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let id = args
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("id".to_owned()))?;
        let runner = zk_kit_subagent::runner()
            .ok_or_else(|| ToolError::Denied("subagent runner is not installed".to_owned()))?;
        let status = runner
            .status(id)
            .ok_or_else(|| ToolError::NotFound(format!("subagent: {id}")))?;
        let result = runner.result(id);
        Ok(serde_json::json!({
            "agent": status,
            "result": result.map(|outcome| match outcome {
                Ok(text) => serde_json::json!({"ok": true, "text": text}),
                Err(error) => serde_json::json!({"ok": false, "error": error}),
            })
        })
        .to_string())
    }
}

impl Tool for SubagentInterruptTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "subagent_interrupt".to_owned(),
            description: "Cancel one running background subagent.".to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"id":{"type":"string"}},"required":["id"],"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "subagent_interrupt requires JSON".to_owned(),
        ))
    }
    fn call_json(&self, _c: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let id = args
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("id".to_owned()))?;
        let runner = zk_kit_subagent::runner()
            .ok_or_else(|| ToolError::Denied("subagent runner is not installed".to_owned()))?;
        runner
            .interrupt(id)
            .map_err(|error| ToolError::Io(error.to_string()))?;
        Ok(serde_json::json!({"interrupted": true, "id": id}).to_string())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only

    use std::sync::Mutex;

    use zk_kit_subagent::{AsyncAgentStatus, SubagentRunner};

    use super::*;

    struct FakeRunner {
        interrupted: Mutex<Vec<String>>,
    }

    impl SubagentRunner for FakeRunner {
        fn run(
            &self,
            _prompt: &str,
            _context: &TurnContext,
            _depth: usize,
        ) -> Result<String, String> {
            Ok("sync".to_owned())
        }
        fn start(
            &self,
            _prompt: &str,
            _context: &TurnContext,
            _depth: usize,
        ) -> Result<String, String> {
            Ok("async-1".to_owned())
        }
        fn status(&self, id: &str) -> Option<AsyncAgentStatus> {
            (id == "async-1").then(|| AsyncAgentStatus {
                id: id.to_owned(),
                depth: 1,
                status: "running".to_owned(),
                prompt: "inspect".to_owned(),
            })
        }
        fn result(&self, id: &str) -> Option<Result<String, String>> {
            (id == "async-1").then(|| Ok("done".to_owned()))
        }
        fn interrupt(&self, id: &str) -> Result<(), String> {
            if id != "async-1" {
                return Err(format!("unknown subagent: {id}"));
            }
            if let Ok(mut interrupted) = self.interrupted.lock() {
                interrupted.push(id.to_owned());
            }
            Ok(())
        }
    }

    fn context() -> TurnContext {
        TurnContext {
            conversation_id: "c".to_owned(),
            workspace_id: "w".to_owned(),
            workspace_root: ".".to_owned(),
        }
    }

    #[test]
    fn status_and_interrupt_tools_delegate_to_the_runner() {
        let runner = Arc::new(FakeRunner {
            interrupted: Mutex::new(Vec::new()),
        });
        zk_kit_subagent::set_runner(runner.clone());
        let value = status_tool()
            .call_json(&context(), serde_json::json!({"id": "async-1"}))
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&value).unwrap();
        assert_eq!(parsed["agent"]["status"], "running");
        assert_eq!(parsed["result"]["ok"], true);
        let unknown = status_tool().call_json(&context(), serde_json::json!({"id": "missing"}));
        assert!(unknown.is_err());
        assert!(interrupt_tool()
            .call_json(&context(), serde_json::json!({"id": "async-1"}))
            .is_ok());
        assert_eq!(runner.interrupted.lock().unwrap().as_slice(), ["async-1"]);
        assert_eq!(extra_tools().len(), 2);
    }

    #[test]
    fn list_agents_includes_async_children() {
        zk_kit_subagent::register("list-async-1", "c", 1, "inspect");
        let runner = Arc::new(FakeRunner {
            interrupted: Mutex::new(Vec::new()),
        });
        zk_kit_subagent::set_runner(runner);
        let value = tool().call_json(&context(), serde_json::json!({})).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&value).unwrap();
        assert!(parsed["max_async"].as_u64().unwrap() >= 1);
        zk_kit_subagent::finish("list-async-1", true);
    }
}
