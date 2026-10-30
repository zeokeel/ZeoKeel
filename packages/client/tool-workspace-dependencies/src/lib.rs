//! Workspace dependency inventory.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct WorkspaceDependenciesTool;
#[must_use]
pub fn tool() -> WorkspaceDependenciesTool {
    WorkspaceDependenciesTool
}
impl Tool for WorkspaceDependenciesTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "load_workspace_dependencies".to_owned(),
            description: "Load Cargo.toml/package.json dependency names from the workspace."
                .to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{},"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "load_workspace_dependencies takes no arguments".to_owned(),
        ))
    }
    fn call_json(
        &self,
        context: &TurnContext,
        _args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let root = zk_kit_tools::workspace_root(context)?;
        let mut out = serde_json::Map::new();
        for name in ["Cargo.toml", "package.json"] {
            let path = root.join(name);
            if !path.is_file() {
                continue;
            }
            let text =
                std::fs::read_to_string(&path).map_err(|error| ToolError::Io(error.to_string()))?;
            if name == "Cargo.toml" {
                let parsed: toml::Value = toml::from_str(&text)
                    .map_err(|error| ToolError::InvalidArgs(error.to_string()))?;
                let mut deps = Vec::new();
                for table in ["dependencies", "dev-dependencies", "build-dependencies"] {
                    if let Some(map) = parsed.get(table).and_then(toml::Value::as_table) {
                        deps.extend(map.keys().cloned());
                    }
                }
                deps.sort();
                deps.dedup();
                out.insert(name.to_owned(), serde_json::json!(deps));
            } else {
                let parsed: serde_json::Value = serde_json::from_str(&text)
                    .map_err(|error| ToolError::InvalidArgs(error.to_string()))?;
                let mut deps = Vec::new();
                for table in ["dependencies", "devDependencies", "peerDependencies"] {
                    if let Some(map) = parsed.get(table).and_then(serde_json::Value::as_object) {
                        deps.extend(map.keys().cloned());
                    }
                }
                deps.sort();
                deps.dedup();
                out.insert(name.to_owned(), serde_json::json!(deps));
            }
        }
        Ok(serde_json::Value::Object(out).to_string())
    }
}
