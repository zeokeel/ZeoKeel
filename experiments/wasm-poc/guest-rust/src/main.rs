//! 原型例外：wasip1 的 host import 在 Rust 中必须用 unsafe extern 调用；产品侧将改为生成绑定或纯 WASI 方案。
//! 产品手写代码仍要求零 unsafe（见 docs/18 §4、ADR-040）。
use serde_json::{json, Value};
use std::io::{Read, Write};

#[link(wasm_import_module = "zeokeel")]
extern "C" {
    fn db_exec(ptr: *const u8, len: usize) -> i32;
    fn db_query(ptr: *const u8, len: usize, out: *mut u8, cap: usize) -> i32;
    fn log(ptr: *const u8, len: usize);
    fn asset_read(name: *const u8, nlen: usize, out: *mut u8, cap: usize) -> i32;
}

fn host_call1(f: unsafe extern "C" fn(*const u8, usize) -> i32, s: &str) -> i32 {
    unsafe { f(s.as_ptr(), s.len()) }
}

fn db_exec_json(s: &str) -> i32 {
    unsafe { db_exec(s.as_ptr(), s.len()) }
}

fn db_query_json(s: &str) -> Result<Value, String> {
    let mut buf = vec![0u8; 256 * 1024];
    let n = unsafe { db_query(s.as_ptr(), s.len(), buf.as_mut_ptr(), buf.len()) };
    if n < 0 {
        return Err(format!("db_query error {n}"));
    }
    let text = String::from_utf8_lossy(&buf[..n as usize]).to_string();
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn log_line(s: &str) {
    unsafe { log(s.as_ptr(), s.len()) }
}

fn read_frame() -> Result<Value, String> {
    let mut raw = Vec::new();
    std::io::stdin().read_to_end(&mut raw).map_err(|e| e.to_string())?;
    let pos = raw.iter().position(|b| *b == b'\n').ok_or("no frame header")?;
    let header = String::from_utf8_lossy(&raw[..pos]).to_string();
    let len: usize = header.trim_start_matches("RS").trim().parse().map_err(|_| "bad frame header")?;
    let body = &raw[pos + 1..];
    if body.len() < len {
        return Err("short frame".into());
    }
    serde_json::from_slice(&body[..len]).map_err(|e| e.to_string())
}

fn write_frame(v: &Value) {
    let body = serde_json::to_vec(v).unwrap();
    let mut out = std::io::stdout();
    let _ = write!(out, "RS{}\n", body.len());
    let _ = out.write_all(&body);
    let _ = out.flush();
}

fn main() {
    let req = match read_frame() {
        Ok(v) => v,
        Err(e) => {
            write_frame(&json!({"status":400,"body":{"error":e}}));
            return;
        }
    };
    let op = req["body"]["op"].as_str().unwrap_or("list");
    let resp = match op {
        "add" => {
            let name = req["body"]["name"].as_str().unwrap_or("anonymous");
            let payload = json!({
                "sql": "INSERT INTO items(name) VALUES(?)",
                "params": [name]
            });
            let rc = db_exec_json(&payload.to_string());
            if rc == 0 {
                log_line("inserted");
                json!({"status":200,"body":{"ok":true,"name":name}})
            } else {
                json!({"status":500,"body":{"error":format!("db_exec rc={rc}")}})
            }
        }
        "asset" => {
            let mut buf = vec![0u8; 64 * 1024];
            let name = "hello.txt";
            let n = unsafe { asset_read(name.as_ptr(), name.len(), buf.as_mut_ptr(), buf.len()) };
            json!({"status":200,"body":{"asset_len":n.max(0)}})
        }
        _ => match db_query_json(r#"{"sql":"SELECT id,name FROM items ORDER BY id","params":[]}"#) {
            Ok(rows) => json!({"status":200,"body":{"rows":rows}}),
            Err(e) => json!({"status":500,"body":{"error":e}}),
        },
    };
    write_frame(&resp);
}
