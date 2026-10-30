#![forbid(unsafe_code)]
use zk_contract_tool::*;
use serde_json::{json, Value};

pub struct FsTool;
impl Tool for FsTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec { name: "fs".into(), description: "plain fs".into(), input_schema: json!({"type":"object"}) }
    }
    fn call(&self, _args: Value) -> Result<String, String> { Ok("plain-fs".into()) }
}

pub struct Pkg;
impl ToolPackage for Pkg {
    fn package_name(&self) -> &'static str { "zk-tool-fs" }
    fn tools(&self) -> Vec<Box<dyn Tool>> { vec![Box::new(FsTool)] }
}
pub fn package() -> impl ToolPackage { Pkg }
