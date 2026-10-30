#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::io::Write;
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn reads_epub_and_rejects_pdf() {
    let dir = std::env::temp_dir().join(format!("zk-doc-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = std::fs::File::create(dir.join("a.epub")).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("chapter1.xhtml", zip::write::SimpleFileOptions::default()).unwrap();
    zip.write_all(b"<html><body>Hello EPUB</body></html>").unwrap();
    zip.finish().unwrap();
    std::fs::write(dir.join("a.pdf"), b"%PDF").unwrap();
    let ctx = TurnContext { conversation_id: "c".to_owned(), workspace_id: "w".to_owned(), workspace_root: dir.display().to_string() };
    let tool = zk_tool_read_document::tool();
    assert!(tool.call_json(&ctx, serde_json::json!({"path":"a.epub"})).unwrap().contains("Hello EPUB"));
    assert!(tool.call_json(&ctx, serde_json::json!({"path":"a.pdf"})).is_err());
}
