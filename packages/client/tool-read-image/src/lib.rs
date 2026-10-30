//! Read image as data URL.
#![forbid(unsafe_code)]
use base64::Engine;
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct ReadImageTool;
#[must_use]
pub fn tool() -> ReadImageTool {
    ReadImageTool
}
impl Tool for ReadImageTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read_image".to_owned(),
            description: "Read a PNG/JPEG/WebP/GIF file and return a data URL (max 2MB)."
                .to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"],"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "read_image requires JSON".to_owned(),
        ))
    }
    fn call_json(
        &self,
        context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let path = args
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("path".to_owned()))?;
        let target = zk_kit_tools::safe_path(context, path)?;
        let metadata =
            std::fs::metadata(&target).map_err(|error| ToolError::Io(error.to_string()))?;
        if metadata.len() > 2 * 1024 * 1024 {
            return Err(ToolError::Denied("image too large (max 2MB)".to_owned()));
        }
        let bytes = std::fs::read(&target).map_err(|error| ToolError::Io(error.to_string()))?;
        let mime = match target
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            "gif" => "image/gif",
            _ => return Err(ToolError::InvalidArgs("unsupported image type".to_owned())),
        };
        Ok(format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ))
    }
}
