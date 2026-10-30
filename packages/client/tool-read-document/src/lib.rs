//! EPUB reader (zip + xhtml); PDF explicitly unsupported.
#![forbid(unsafe_code)]
use std::io::Read;
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

pub struct ReadDocumentTool;
#[must_use] pub fn tool() -> ReadDocumentTool { ReadDocumentTool }

fn strip_markup(html: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for ch in html.chars() {
        match ch { '<' => tag = true, '>' => tag = false, _ if !tag => out.push(ch), _ => {} }
    }
    out
}

impl Tool for ReadDocumentTool {
    fn spec(&self) -> ToolSpec { ToolSpec { name: "read_document".to_owned(), description: "Read text from an EPUB file inside the workspace; PDF is unsupported in this build.".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"],"additionalProperties":false}), mutating: false, requires_approval: false } }
    fn call(&self, _args: &str) -> Result<String, ToolError> { Err(ToolError::InvalidArgs("read_document requires JSON".to_owned())) }
    fn call_json(&self, context: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let relative = args.get("path").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("path".to_owned()))?;
        let path = zk_kit_tools::safe_path(context, relative)?;
        let extension = path.extension().and_then(std::ffi::OsStr::to_str).unwrap_or("").to_ascii_lowercase();
        if extension == "pdf" { return Err(ToolError::Denied("DOCUMENT_FORMAT_UNSUPPORTED: pdf loader is not available offline".to_owned())); }
        if extension != "epub" { return Err(ToolError::InvalidArgs("only .epub is supported".to_owned())); }
        let metadata = std::fs::metadata(&path).map_err(|error| ToolError::Io(error.to_string()))?;
        if metadata.len() > 1024 * 1024 { return Err(ToolError::Denied("document too large (max 1MB)".to_owned())); }
        let file = std::fs::File::open(&path).map_err(|error| ToolError::Io(error.to_string()))?;
        let mut archive = zip::ZipArchive::new(file).map_err(|error| ToolError::InvalidArgs(error.to_string()))?;
        let mut text = String::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|error| ToolError::Io(error.to_string()))?;
            let name = entry.name().to_ascii_lowercase();
            if !(name.ends_with(".xhtml") || name.ends_with(".html") || name.ends_with(".htm")) { continue; }
            let mut content = String::new();
            entry.read_to_string(&mut content).map_err(|error| ToolError::Io(error.to_string()))?;
            text.push_str(&strip_markup(&content));
            text.push('\n');
            if text.len() > 1024 * 1024 { break; }
        }
        Ok(zk_kit_tools::truncate(text))
    }
}
