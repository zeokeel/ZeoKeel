//! Sandboxed WASM guest runtime shared by `srv.app-runtime` and the P2 prototype.
//! @capability wasm.runtime
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rusqlite::types::Value as SqlValue;
use rusqlite::{params_from_iter, Connection};
use serde_json::{json, Value};
use thiserror::Error;
use wasmtime::{Caller, Config, Engine, Extern, Linker, ResourceLimiter, Store};
use wasmtime_wasi::p1::{self, WasiP1Ctx};
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
use wasmtime_wasi::WasiCtxBuilder;

/// Runtime error surface (server maps to APP_* codes).
#[derive(Debug, Error)]
pub enum WasmError {
    #[error("bad wasm")] BadWasm,
    #[error("bad request")] BadRequest,
    #[error("bad frame")] BadFrame,
    #[error("fuel exhausted")] Fuel,
    #[error("timeout")] Timeout,
    #[error("memory limit")] Memory,
    #[error("db error: {0}")] Db(String),
    #[error("sql rejected: {0}")] Sql(String),
    #[error("internal: {0}")] Internal(String),
}

/// Run limits (docs/04 §9).
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub fuel: u64,
    pub timeout: Duration,
    pub memory_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self { Self { fuel: 10_000_000, timeout: Duration::from_secs(30), memory_bytes: 64 * 1024 * 1024 } }
}

/// One run input.
pub struct RunInput<'a> {
    pub wasm: &'a [u8],
    pub db_path: &'a Path,
    pub request: Value,
    pub limits: Limits,
    pub assets: &'a HashMap<String, Vec<u8>>,
    pub db_quota_bytes: u64,
}

/// One run output.
#[derive(Debug, Clone, PartialEq)]
pub struct RunOutput { pub response: Value, pub fuel_used: u64, pub elapsed: Duration, pub logs: Vec<String> }

struct HostState { wasi: WasiP1Ctx, last_error: String, limiter: StoreLimits, logs: Arc<Mutex<Vec<String>>> }

struct StoreLimits { max_memory: usize }

impl ResourceLimiter for StoreLimits {
    fn memory_growing(&mut self, _current: usize, desired: usize, _maximum: Option<usize>) -> Result<bool, wasmtime::Error> {
        Ok(desired <= self.max_memory)
    }
    fn table_growing(&mut self, _current: usize, desired: usize, _maximum: Option<usize>) -> Result<bool, wasmtime::Error> {
        Ok(desired <= 10_000)
    }
}

fn next_sql_deadline() -> Instant { Instant::now().checked_add(Duration::from_secs(5)).unwrap_or_else(Instant::now) }

fn read_mem(caller: &mut Caller<'_, HostState>, ptr: u32, len: u32) -> Result<Vec<u8>, WasmError> {
    let memory = match caller.get_export("memory") { Some(Extern::Memory(memory)) => memory, _ => return Err(WasmError::BadWasm) };
    let mut buffer = vec![0_u8; usize::try_from(len).map_err(|_| WasmError::BadRequest)?];
    memory.read(caller, usize::try_from(ptr).map_err(|_| WasmError::BadRequest)?, &mut buffer).map_err(|_| WasmError::BadWasm)?;
    Ok(buffer)
}

fn write_mem(caller: &mut Caller<'_, HostState>, ptr: u32, cap: u32, data: &[u8]) -> i32 {
    if data.len() > usize::try_from(cap).unwrap_or(0) { return -2; }
    let Some(Extern::Memory(memory)) = caller.get_export("memory") else { return -1 };
    let Ok(offset) = usize::try_from(ptr) else { return -1 };
    if memory.write(caller, offset, data).is_err() { -1 } else { i32::try_from(data.len()).unwrap_or(-1) }
}

fn sql_value(value: &Value) -> SqlValue {
    match value {
        Value::Null => SqlValue::Null,
        Value::Bool(value) => SqlValue::Integer(i64::from(*value)),
        Value::Number(number) => number.as_i64().map_or(SqlValue::Null, SqlValue::Integer),
        Value::String(value) => SqlValue::Text(value.clone()),
        other => SqlValue::Text(other.to_string()),
    }
}

/// Reject anything not a single DML statement. `db.exec/query` are the only entry points.
pub fn validate_sql(sql: &str) -> Result<(), WasmError> {
    let trimmed = sql.trim();
    if trimmed.is_empty() || trimmed.len() > 16_384 { return Err(WasmError::Sql("size".into())); }
    let upper = trimmed.to_ascii_uppercase();
    let first = upper.split_whitespace().next().unwrap_or("");
    if !matches!(first, "SELECT" | "INSERT" | "UPDATE" | "DELETE") { return Err(WasmError::Sql(format!("statement {first}"))); }
    for token in [";", "--", "/*", "*/", "ATTACH", "DETACH", "PRAGMA", "VACUUM", "LOAD_EXTENSION", "READFILE", "WRITEFILE", "CREATE", "DROP", "ALTER", "TRIGGER", "VIRTUAL"] {
        if upper.contains(token) { return Err(WasmError::Sql(format!("forbidden {token}"))); }
    }
    Ok(())
}

fn db_args(raw: &[u8]) -> Result<(String, Vec<SqlValue>), WasmError> {
    let value: Value = serde_json::from_slice(raw).map_err(|_| WasmError::BadRequest)?;
    let sql = value.get("sql").and_then(Value::as_str).ok_or(WasmError::BadRequest)?.to_owned();
    validate_sql(&sql)?;
    let params = value.get("params").and_then(Value::as_array).map_or_else(Vec::new, |items| items.iter().map(sql_value).collect());
    Ok((sql, params))
}

/// Execute one guest request. The caller owns the DB file and schema migration.
pub fn run(input: RunInput<'_>) -> Result<RunOutput, WasmError> {
    if input.wasm.get(..4) != Some(b"\0asm") { return Err(WasmError::BadWasm); }
    let started = Instant::now();
    let body = serde_json::to_vec(&input.request).map_err(|_| WasmError::BadRequest)?;
    let mut frame = format!("RS{}\n", body.len()).into_bytes();
    frame.extend_from_slice(&body);
    let out_pipe = MemoryOutputPipe::new(8 * 1024 * 1024);
    let wasi = WasiCtxBuilder::new()
        .stdin(MemoryInputPipe::new(frame))
        .stdout(out_pipe.clone())
        .stderr(MemoryOutputPipe::new(64 * 1024))
        .build_p1();
    let connection = Connection::open(input.db_path).map_err(|error| WasmError::Db(error.to_string()))?;
    connection.busy_timeout(Duration::from_secs(5)).map_err(|error| WasmError::Db(error.to_string()))?;
    if input.db_quota_bytes > 0 {
        let pages = i64::try_from(input.db_quota_bytes / 4096).unwrap_or(i64::MAX).max(1);
        connection.pragma_update(None, "max_page_count", pages).map_err(|error| WasmError::Db(error.to_string()))?;
    }
    let sql_deadline = Arc::new(Mutex::new(next_sql_deadline()));
    let progress_deadline = sql_deadline.clone();
    connection.progress_handler(1_000, Some(move || Instant::now() > *progress_deadline.lock().unwrap_or_else(|poison| poison.into_inner())));
    let database = Arc::new(Mutex::new(connection));
    let exec_deadline = sql_deadline.clone();
    let query_deadline = sql_deadline.clone();
    let assets = Arc::new(Mutex::new(input.assets.clone()));

    let mut config = Config::new();
    config.consume_fuel(true);
    config.epoch_interruption(true);
    let engine = Engine::new(&config).map_err(|error| WasmError::Internal(error.to_string()))?;
    let module = wasmtime::Module::new(&engine, input.wasm).map_err(|_| WasmError::BadWasm)?;
    let mut linker: Linker<HostState> = Linker::new(&engine);
    p1::add_to_linker_sync(&mut linker, |state: &mut HostState| &mut state.wasi).map_err(|error| WasmError::Internal(error.to_string()))?;

    let db = database.clone();
    linker.func_wrap("zeokeel", "db_exec", move |mut caller: Caller<'_, HostState>, ptr: u32, len: u32| -> i32 {
        if let Ok(mut deadline) = exec_deadline.lock() { *deadline = next_sql_deadline(); }
        let Ok(raw) = read_mem(&mut caller, ptr, len) else { return -1 };
        let Ok((sql, params)) = db_args(&raw) else { return -1 };
        let Ok(connection) = db.lock() else { return -1 };
        match connection.execute(&sql, params_from_iter(params.iter())) { Ok(_) => 0, Err(error) => { caller.data_mut().last_error = error.to_string(); -1 } }
    }).map_err(|error| WasmError::Internal(error.to_string()))?;

    let db = database.clone();
    linker.func_wrap("zeokeel", "db_query", move |mut caller: Caller<'_, HostState>, ptr: u32, len: u32, out: u32, cap: u32| -> i32 {
        if let Ok(mut deadline) = query_deadline.lock() { *deadline = next_sql_deadline(); }
        let Ok(raw) = read_mem(&mut caller, ptr, len) else { return -1 };
        let Ok((sql, params)) = db_args(&raw) else { return -1 };
        let Ok(connection) = db.lock() else { return -1 };
        let Ok(mut statement) = connection.prepare(&sql) else { return -1 };
        let names: Vec<String> = statement.column_names().iter().map(|name| (*name).to_string()).collect();
        let Ok(mut rows) = statement.query(params_from_iter(params.iter())) else { return -1 };
        let mut out_rows: Vec<Value> = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let mut object = serde_json::Map::new();
            for (index, name) in names.iter().enumerate() {
                let value = match row.get_ref(index) {
                    Ok(rusqlite::types::ValueRef::Null) => Value::Null,
                    Ok(rusqlite::types::ValueRef::Integer(number)) => json!(number),
                    Ok(rusqlite::types::ValueRef::Real(number)) => json!(number),
                    Ok(rusqlite::types::ValueRef::Text(text)) => json!(String::from_utf8_lossy(text).to_string()),
                    _ => Value::Null,
                };
                object.insert(name.clone(), value);
            }
            out_rows.push(Value::Object(object));
        }
        let data = serde_json::to_vec(&Value::Array(out_rows)).unwrap_or_default();
        write_mem(&mut caller, out, cap, &data)
    }).map_err(|error| WasmError::Internal(error.to_string()))?;

    linker.func_wrap("zeokeel", "log", move |mut caller: Caller<'_, HostState>, ptr: u32, len: u32| {
        if let Ok(raw) = read_mem(&mut caller, ptr, len) {
            let message = String::from_utf8_lossy(&raw).chars().take(4096).collect::<String>();
            if let Ok(mut lines) = caller.data().logs.lock() { if lines.len() < 1000 { lines.push(message); } }
        }
    }).map_err(|error| WasmError::Internal(error.to_string()))?;

    let assets = assets.clone();
    linker.func_wrap("zeokeel", "asset_read", move |mut caller: Caller<'_, HostState>, nptr: u32, nlen: u32, out: u32, cap: u32| -> i32 {
        let Ok(name) = read_mem(&mut caller, nptr, nlen) else { return -1 };
        let name = String::from_utf8_lossy(&name).to_string();
        let Ok(assets) = assets.lock() else { return -1 };
        assets.get(&name).map_or(-3, |bytes| write_mem(&mut caller, out, cap, bytes))
    }).map_err(|error| WasmError::Internal(error.to_string()))?;

    let logs = Arc::new(Mutex::new(Vec::new()));
    let log_sink = logs.clone();
    let state = HostState { wasi, last_error: String::new(), limiter: StoreLimits { max_memory: input.limits.memory_bytes }, logs };
    let mut store = Store::new(&engine, state);
    store.limiter(|state| &mut state.limiter);
    store.set_fuel(input.limits.fuel).map_err(|error| WasmError::Internal(error.to_string()))?;
    let ticker_stop = Arc::new(AtomicBool::new(false));
    let ticker = {
        let stop = ticker_stop.clone();
        let engine = engine.clone();
        let interval = input.limits.timeout.checked_div(50).unwrap_or(Duration::from_millis(1));
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(interval.max(Duration::from_millis(1)));
                engine.increment_epoch();
            }
        })
    };
    store.set_epoch_deadline(50);
    let instance = linker.instantiate(&mut store, &module).map_err(|_| WasmError::BadWasm)?;
    let start = instance.get_typed_func::<(), ()>(&mut store, "_start").map_err(|_| WasmError::BadWasm)?;
    let result = start.call(&mut store, ());
    ticker_stop.store(true, Ordering::Relaxed);
    let _ = ticker.join();
    let elapsed = started.elapsed();
    let fuel_used = store.get_fuel().map_or(0, |remaining| input.limits.fuel.saturating_sub(remaining));
    if let Err(error) = &result {
        let exit_ok = error.downcast_ref::<wasmtime_wasi::I32Exit>().is_some_and(|exit| exit.0 == 0);
        if !exit_ok {
            let text = error.to_string();
            if store.get_fuel().unwrap_or(1) == 0 { return Err(WasmError::Fuel); }
            if elapsed >= input.limits.timeout.saturating_sub(Duration::from_millis(10)) { return Err(WasmError::Timeout); }
            if input.limits.memory_bytes < 64 * 1024 * 1024 { return Err(WasmError::Memory); }
            return if text.contains("fuel") { Err(WasmError::Fuel) } else if text.contains("epoch") || text.contains("timeout") { Err(WasmError::Timeout) } else if text.contains("memory") { Err(WasmError::Memory) } else { Err(WasmError::BadWasm) };
        }
    }
    let output = out_pipe.contents();
    let text = String::from_utf8_lossy(&output);
    let Some(position) = text.find('\n') else { return Err(WasmError::BadFrame); };
    let header = text.get(..position).ok_or(WasmError::BadFrame)?;
    let length: usize = header.strip_prefix("RS").and_then(|value| value.trim().parse().ok()).ok_or(WasmError::BadFrame)?;
    let start = position.checked_add(1).ok_or(WasmError::BadFrame)?;
    let bytes = output.get(start..).ok_or(WasmError::BadFrame)?;
    let take = length.min(bytes.len());
    let body = bytes.get(..take).ok_or(WasmError::BadFrame)?;
    let response: Value = serde_json::from_slice(body).map_err(|_| WasmError::BadFrame)?;
    let logs = log_sink.lock().map(|lines| lines.clone()).unwrap_or_default();
    Ok(RunOutput { response, fuel_used, elapsed, logs })
}

/// Re-export for callers that only need the DB path convention.
pub fn app_db_path(root: &Path, app_id: &str) -> PathBuf { root.join(format!("{app_id}.sqlite")) }
