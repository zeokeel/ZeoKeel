//! Contract conformance: a tool implementation is object-safe and errors round-trip.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use std::sync::Arc;
use zk_contract_tool::{Tool, ToolError, ToolSpec};

struct Echo;

impl Tool for Echo {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "demo.echo".to_owned(),
            description: "echo".to_owned(),
            parameters: serde_json::json!({"type":"object"}),
            mutating: false,
            requires_approval: false,
        }
    }

    fn call(&self, args: &str) -> Result<String, ToolError> {
        if args.is_empty() {
            return Err(ToolError::InvalidArgs("empty".to_owned()));
        }
        Ok(args.to_owned())
    }
}

#[test]
fn tool_is_object_safe_and_errors_serialize() {
    let tool: Arc<dyn Tool> = Arc::new(Echo);
    assert_eq!(tool.call("hi").unwrap(), "hi");
    assert!(tool.call("").is_err());
    let err = ToolError::Denied("..".to_owned());
    let json = serde_json::to_string(&err).unwrap();
    assert!(json.to_lowercase().contains("denied"));
}
