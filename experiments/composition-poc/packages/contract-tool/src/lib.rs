#![forbid(unsafe_code)]
//! Contract seam for the `tool.*` roles. Packages implement this; the composed
//! glue crate only ever sees this trait, never a concrete implementation.
use serde::{Deserialize, Serialize};

pub const CONTRACT_ID: &str = "zk.contract.tool";
pub const CONTRACT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

pub trait Tool: Send + Sync + 'static {
    fn spec(&self) -> ToolSpec;
    fn call(&self, args: serde_json::Value) -> Result<String, String>;
}

pub trait ToolPackage: Send + Sync + 'static {
    fn package_name(&self) -> &'static str;
    fn tools(&self) -> Vec<Box<dyn Tool>>;
}
