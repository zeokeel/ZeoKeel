//! Subagent delegation tools (synchronous + asynchronous start).
#![forbid(unsafe_code)]
use std::sync::Arc;

use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

/// Synchronous `subagent` tool.
pub struct SubagentTool;
#[must_use]
pub fn tool() -> SubagentTool {
    SubagentTool
}

/// Asynchronous `subagent_start` tool.
pub struct SubagentStartTool;
#[must_use]
pub fn start_tool() -> SubagentStartTool {
    SubagentStartTool
}

/// Extra tools contributed by this package (composed after the base tool).
///
/// `zk compose` appends these when a tool package exports this function
/// (generated glue convention); single-role packages can therefore contribute
/// more than one tool without new roles.
#[must_use]
pub fn extra_tools() -> Vec<Arc<dyn Tool>> {
    vec![Arc::new(SubagentStartTool)]
}
impl Tool for SubagentTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "subagent".to_owned(),
            description: "Delegate a focused task to a child agent and return its final answer."
                .to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"prompt":{"type":"string"}},"required":["prompt"],"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs("subagent requires JSON".to_owned()))
    }
    fn call_json(
        &self,
        context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let prompt = args
            .get("prompt")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("prompt".to_owned()))?;
        let runner = zk_kit_subagent::runner()
            .ok_or_else(|| ToolError::Denied("subagent runner is not installed".to_owned()))?;
        let depth = zk_kit_subagent::current_depth();
        if depth >= zk_kit_subagent::MAX_DEPTH {
            return Err(ToolError::Denied("subagent depth limit reached".to_owned()));
        }
        zk_kit_subagent::with_depth(depth.saturating_add(1), || {
            runner.run(prompt, context, depth)
        })
        .map_err(ToolError::Io)
    }
}

impl Tool for SubagentStartTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "subagent_start".to_owned(),
            description: "Start a background child agent and return its id; poll with subagent_status and cancel with subagent_interrupt.".to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"prompt":{"type":"string"}},"required":["prompt"],"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "subagent_start requires JSON".to_owned(),
        ))
    }
    fn call_json(
        &self,
        context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let prompt = args
            .get("prompt")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("prompt".to_owned()))?;
        let runner = zk_kit_subagent::runner()
            .ok_or_else(|| ToolError::Denied("subagent runner is not installed".to_owned()))?;
        let depth = zk_kit_subagent::current_depth();
        if depth >= zk_kit_subagent::MAX_DEPTH {
            return Err(ToolError::Denied("subagent depth limit reached".to_owned()));
        }
        let id = runner
            .start(prompt, context, depth)
            .map_err(ToolError::Io)?;
        Ok(serde_json::json!({ "id": id, "status": "running" }).to_string())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only

    use super::*;
    use std::sync::Mutex;

    use zk_kit_subagent::{AsyncAgentStatus, SubagentRunner};

    struct FakeRunner {
        started: Mutex<Vec<String>>,
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
            prompt: &str,
            _context: &TurnContext,
            _depth: usize,
        ) -> Result<String, String> {
            if let Ok(mut started) = self.started.lock() {
                started.push(prompt.to_owned());
            }
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
    fn async_tools_use_the_installed_runner() {
        let runner = Arc::new(FakeRunner {
            started: Mutex::new(Vec::new()),
            interrupted: Mutex::new(Vec::new()),
        });
        zk_kit_subagent::set_runner(runner.clone());
        let started = start_tool()
            .call_json(&context(), serde_json::json!({"prompt": "inspect"}))
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&started).unwrap()["id"],
            "async-1"
        );
        assert_eq!(runner.started.lock().unwrap().as_slice(), ["inspect"]);
        assert_eq!(extra_tools().len(), 1);
    }
}
