use zk_contract_tool::*;
use serde_json::{json, Value};

pub struct FsTool;
impl Tool for FsTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec { name: "fs".into(), description: "acme fs (third-party)".into(), input_schema: json!({"type":"object"}) }
    }
    fn call(&self, _args: Value) -> Result<String, String> { Ok("acme-fs".into()) }
}
pub struct Pkg;
impl ToolPackage for Pkg {
    fn package_name(&self) -> &'static str { "acme-tool-fs" }
    fn tools(&self) -> Vec<Box<dyn Tool>> { vec![Box::new(FsTool)] }
}
pub fn package() -> impl ToolPackage { Pkg }
