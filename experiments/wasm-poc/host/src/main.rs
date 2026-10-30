#![forbid(unsafe_code)]
use anyhow::{anyhow, Context, Result};
use rusqlite::{params_from_iter, types::Value as SqlValue, Connection};
use serde_json::{json, Value};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Config, Engine, Extern, Linker, Module, Store};
use wasmtime_wasi::I32Exit;
use wasmtime_wasi::p1::{self, WasiP1Ctx};
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
use wasmtime_wasi::WasiCtxBuilder;

struct HostState {
    wasi: WasiP1Ctx,
    last_error: String,
}

fn read_guest_mem(caller: &mut Caller<'_, HostState>, ptr: u32, len: u32) -> Result<Vec<u8>> {
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(m)) => m,
        _ => return Err(anyhow!("guest has no memory export")),
    };
    let mut buf = vec![0u8; len as usize];
    mem.read(caller, ptr as usize, &mut buf)?;
    Ok(buf)
}

fn write_guest_mem(caller: &mut Caller<'_, HostState>, ptr: u32, cap: u32, data: &[u8]) -> i32 {
    if data.len() > cap as usize {
        return -2;
    }
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(m)) => m,
        _ => return -1,
    };
    if mem.write(caller, ptr as usize, data).is_err() {
        return -1;
    }
    data.len() as i32
}

fn parse_db_args(raw: &[u8]) -> Result<(String, Vec<SqlValue>)> {
    let v: Value = serde_json::from_slice(raw)?;
    let sql = v["sql"].as_str().ok_or_else(|| anyhow!("missing sql"))?.to_string();
    let mut params = Vec::new();
    if let Some(arr) = v["params"].as_array() {
        for p in arr {
            params.push(match p {
                Value::Null => SqlValue::Null,
                Value::Bool(b) => SqlValue::Integer(*b as i64),
                Value::Number(n) => SqlValue::Integer(n.as_i64().unwrap_or(0)),
                Value::String(s) => SqlValue::Text(s.clone()),
                other => SqlValue::Text(other.to_string()),
            });
        }
    }
    Ok((sql, params))
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut wasm_path: Option<PathBuf> = None;
    let mut db_path = String::from(":memory:");
    let mut fuel: u64 = 50_000_000;
    let mut request = String::from(r#"{"abi":"zeokeel-app/1","body":{"op":"list"}}"#);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--wasm" => wasm_path = args.next().map(PathBuf::from),
            "--db" => db_path = args.next().unwrap_or(db_path),
            "--fuel" => fuel = args.next().and_then(|v| v.parse().ok()).unwrap_or(fuel),
            "--request" => request = args.next().unwrap_or(request),
            _ => {}
        }
    }
    let wasm_path = wasm_path.ok_or_else(|| anyhow!("usage: host --wasm <guest.wasm> [--db <sqlite>] [--fuel N] [--request <json>]"))?;

    let mut input = String::new();
    if !matches!(std::io::stdin().read_to_string(&mut input), Ok(0) | Err(_)) {
        if input.trim_start().starts_with('{') {
            request = input;
        }
    }

    let mut body = request.into_bytes();
    let mut frame = format!("RS{}\n", body.len()).into_bytes();
    frame.append(&mut body);

    let out_pipe = MemoryOutputPipe::new(4 * 1024 * 1024);
    let wasi = WasiCtxBuilder::new()
        .stdin(MemoryInputPipe::new(frame))
        .stdout(out_pipe.clone())
        .stderr(MemoryOutputPipe::new(64 * 1024))
        .build_p1();

    let conn = Connection::open(&db_path).with_context(|| format!("open db {db_path}"))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS items(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL);",
    )?;
    let db = Arc::new(Mutex::new(conn));
    let mut assets = std::collections::HashMap::new();
    assets.insert("hello.txt".to_string(), b"hello from host asset".to_vec());

    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let module = Module::from_file(&engine, &wasm_path)?;
    let mut linker: Linker<HostState> = Linker::new(&engine);
    p1::add_to_linker_sync(&mut linker, |s: &mut HostState| &mut s.wasi)?;

    let db_handle = db.clone();
    linker.func_wrap(
        "zeokeel",
        "db_exec",
        move |mut caller: Caller<'_, HostState>, ptr: u32, len: u32| -> i32 {
            let raw = match read_guest_mem(&mut caller, ptr, len) {
                Ok(v) => v,
                Err(_) => return -1,
            };
            let (sql, params) = match parse_db_args(&raw) {
                Ok(v) => v,
                Err(e) => {
                    caller.data_mut().last_error = e.to_string();
                    return -1;
                }
            };
            let conn = db_handle.lock().unwrap();
            match conn.execute(&sql, params_from_iter(params.iter())) {
                Ok(_) => 0,
                Err(e) => {
                    caller.data_mut().last_error = e.to_string();
                    -1
                }
            }
        },
    )?;

    let db_handle = db.clone();
    linker.func_wrap(
        "zeokeel",
        "db_query",
        move |mut caller: Caller<'_, HostState>, ptr: u32, len: u32, out: u32, cap: u32| -> i32 {
            let raw = match read_guest_mem(&mut caller, ptr, len) {
                Ok(v) => v,
                Err(_) => return -1,
            };
            let (sql, params) = match parse_db_args(&raw) {
                Ok(v) => v,
                Err(e) => {
                    caller.data_mut().last_error = e.to_string();
                    return -1;
                }
            };
            let conn = db_handle.lock().unwrap();
            let mut stmt = match conn.prepare(&sql) {
                Ok(s) => s,
                Err(e) => {
                    caller.data_mut().last_error = e.to_string();
                    return -1;
                }
            };
            let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
            let mut rows = match stmt.query(params_from_iter(params.iter())) {
                Ok(r) => r,
                Err(e) => {
                    caller.data_mut().last_error = e.to_string();
                    return -1;
                }
            };
            let mut out_rows: Vec<Value> = Vec::new();
            while let Ok(Some(row)) = rows.next() {
                let mut obj = serde_json::Map::new();
                for (i, name) in col_names.iter().enumerate() {
                    let v: Value = match row.get_ref(i) {
                        Ok(rusqlite::types::ValueRef::Null) => Value::Null,
                        Ok(rusqlite::types::ValueRef::Integer(n)) => json!(n),
                        Ok(rusqlite::types::ValueRef::Real(f)) => json!(f),
                        Ok(rusqlite::types::ValueRef::Text(t)) => {
                            json!(String::from_utf8_lossy(t).to_string())
                        }
                        _ => Value::Null,
                    };
                    obj.insert(name.clone(), v);
                }
                out_rows.push(Value::Object(obj));
            }
            let data = serde_json::to_vec(&Value::Array(out_rows)).unwrap();
            write_guest_mem(&mut caller, out, cap, &data)
        },
    )?;

    linker.func_wrap(
        "zeokeel",
        "log",
        |mut caller: Caller<'_, HostState>, ptr: u32, len: u32| {
            if let Ok(raw) = read_guest_mem(&mut caller, ptr, len) {
                eprintln!("[guest] {}", String::from_utf8_lossy(&raw));
            }
        },
    )?;

    let assets_handle = Arc::new(Mutex::new(assets));
    linker.func_wrap(
        "zeokeel",
        "asset_read",
        move |mut caller: Caller<'_, HostState>, nptr: u32, nlen: u32, out: u32, cap: u32| -> i32 {
            let name = match read_guest_mem(&mut caller, nptr, nlen) {
                Ok(v) => String::from_utf8_lossy(&v).to_string(),
                Err(_) => return -1,
            };
            let assets = assets_handle.lock().unwrap();
            match assets.get(&name) {
                Some(bytes) => write_guest_mem(&mut caller, out, cap, bytes),
                None => -3,
            }
        },
    )?;

    let state = HostState { wasi, last_error: String::new() };
    let mut store = Store::new(&engine, state);
    store.set_fuel(fuel)?;
    let instance = linker.instantiate(&mut store, &module)?;
    let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
    let run = start.call(&mut store, ());

    let output = out_pipe.contents();
    let out_text = String::from_utf8_lossy(&output).to_string();
    // Go(wasip1) 通过 proc_exit 结束，wasmtime 会返回 I32Exit(0)；这是成功退出，不是错误。
    let exit_ok = match run.as_ref().err() {
        None => true,
        Some(e) => e
            .downcast_ref::<I32Exit>()
            .map(|t| t.0 == 0)
            .unwrap_or(false),
    };
    if !exit_ok {
        let msg = run.err().map(|e| e.to_string()).unwrap_or_default();
        println!("{}", json!({"host_error": msg, "guest_stdout": out_text}));
        std::process::exit(2);
    }
    // 解析 guest 帧：RS<len>\n<json>
    if let Some(pos) = out_text.find('\n') {
        let header = &out_text[..pos];
        if let Some(len) = header.strip_prefix("RS").and_then(|v| v.trim().parse::<usize>().ok()) {
            let body = &out_text.as_bytes()[pos + 1..];
            let take = len.min(body.len());
            match serde_json::from_slice::<Value>(&body[..take]) {
                Ok(v) => {
                    println!("{}", serde_json::to_string(&v)?);
                    if v["status"].as_i64().unwrap_or(500) >= 400 {
                        std::process::exit(3);
                    }
                    return Ok(());
                }
                Err(e) => {
                    println!("{}", json!({"host_error": format!("bad frame json: {e}"), "raw": out_text}));
                    std::process::exit(2);
                }
            }
        }
    }
    println!("{}", json!({"host_error": "no frame in guest stdout", "raw": out_text}));
    std::process::exit(2);
}
