#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::io::Write;
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn reads_epub_and_pdf() {
    let dir = std::env::temp_dir().join(format!("zk-doc-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = std::fs::File::create(dir.join("a.epub")).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("chapter1.xhtml", zip::write::SimpleFileOptions::default()).unwrap();
    zip.write_all(b"<html><body>Hello EPUB</body></html>").unwrap();
    zip.finish().unwrap();
    {
        use lopdf::{dictionary, content::{Content, Operation}, Document, Object, Stream};
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary!{"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica"});
        let resources = dictionary!{"Font"=>dictionary!{"F1"=>font_id}};
        let content = Content { operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 24.into()]),
            Operation::new("Td", vec![72.into(), 720.into()]),
            Operation::new("Tj", vec![Object::string_literal("Hello PDF")]),
            Operation::new("ET", vec![]),
        ]};
        let content_id = doc.add_object(Stream::new(dictionary!{}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary!{"Type"=>"Page","Parent"=>pages_id,"MediaBox"=>vec![0.into(),0.into(),612.into(),792.into()],"Contents"=>content_id,"Resources"=>resources});
        doc.objects.insert(pages_id, Object::Dictionary(dictionary!{"Type"=>"Pages","Kids"=>vec![page_id.into()],"Count"=>1}));
        let catalog_id = doc.add_object(dictionary!{"Type"=>"Catalog","Pages"=>pages_id});
        doc.trailer.set("Root", catalog_id);
        doc.compress();
        doc.save(dir.join("a.pdf")).unwrap();
    }
    let ctx = TurnContext { conversation_id: "c".to_owned(), workspace_id: "w".to_owned(), workspace_root: dir.display().to_string() };
    let tool = zk_tool_read_document::tool();
    assert!(tool.call_json(&ctx, serde_json::json!({"path":"a.epub"})).unwrap().contains("Hello EPUB"));
    let pdf = tool.call_json(&ctx, serde_json::json!({"path":"a.pdf"})).unwrap();
    assert!(pdf.contains("Hello PDF"));
}
