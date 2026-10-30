#![forbid(unsafe_code)]
fn main() {
    for t in zk_composed::tools() {
        let s = t.spec();
        println!("tool={} impl={} call={:?}", s.name, s.description, t.call(serde_json::json!({})));
    }
}
