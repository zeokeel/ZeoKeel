//! Minimal persistent shell session (for bash-persistent/terminal).
#![forbid(unsafe_code)]
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use zk_contract_tool::{ToolError, TurnContext};

struct Shell {
    stdin: Mutex<ChildStdin>,
    child: Mutex<Child>,
    output: Arc<Mutex<String>>,
    seq: Mutex<u64>,
}

static SHELLS: OnceLock<Mutex<BTreeMap<String, Arc<Shell>>>> = OnceLock::new();
fn shells() -> &'static Mutex<BTreeMap<String, Arc<Shell>>> { SHELLS.get_or_init(|| Mutex::new(BTreeMap::new())) }
fn key(context: &TurnContext) -> String { format!("{}::{}", context.workspace_root, context.conversation_id) }

fn spawn(context: &TurnContext) -> Result<Arc<Shell>, ToolError> {
    let root = crate::workspace_root(context)?;
    let mut child = Command::new("bash").arg("--noprofile").arg("--norc").current_dir(root).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|error| ToolError::Io(error.to_string()))?;
    let stdin = child.stdin.take().ok_or_else(|| ToolError::Io("shell stdin unavailable".to_owned()))?;
    let output = Arc::new(Mutex::new(String::new()));
    if let Some(mut pipe) = child.stdout.take() { let target = Arc::clone(&output); std::thread::spawn(move || { let mut buf = [0_u8; 4096]; loop { match pipe.read(&mut buf) { Ok(0) | Err(_) => break, Ok(n) => { if let Ok(mut guard) = target.lock() { guard.push_str(&String::from_utf8_lossy(buf.get(..n).unwrap_or_default())); } } } } }); }
    if let Some(mut pipe) = child.stderr.take() { let target = Arc::clone(&output); std::thread::spawn(move || { let mut buf = [0_u8; 4096]; loop { match pipe.read(&mut buf) { Ok(0) | Err(_) => break, Ok(n) => { if let Ok(mut guard) = target.lock() { guard.push_str(&String::from_utf8_lossy(buf.get(..n).unwrap_or_default())); } } } } }); }
    Ok(Arc::new(Shell { stdin: Mutex::new(stdin), child: Mutex::new(child), output, seq: Mutex::new(0) }))
}

/// Run a command in a persistent shell for this workspace/conversation.
pub fn run(context: &TurnContext, command: &str, timeout_ms: u64) -> Result<String, ToolError> {
    if command.trim().is_empty() { return Err(ToolError::InvalidArgs("command is empty".to_owned())); }
    let shell = {
        let mut guard = shells().lock().map_err(|_| ToolError::Io("shell lock poisoned".to_owned()))?;
        if let Some(shell) = guard.get(&key(context)) { Arc::clone(shell) } else { let shell = spawn(context)?; guard.insert(key(context), Arc::clone(&shell)); shell }
    };
    let seq = { let mut guard = shell.seq.lock().map_err(|_| ToolError::Io("shell seq poisoned".to_owned()))?; *guard = (*guard).saturating_add(1); *guard };
    let marker = format!("__ZK_DONE_{seq}__");
    if let Ok(mut guard) = shell.output.lock() { guard.clear(); }
    {
        let mut stdin = shell.stdin.lock().map_err(|_| ToolError::Io("shell stdin poisoned".to_owned()))?;
        writeln!(stdin, "{command}\nprintf '\\n{marker}\\n'").map_err(|error| ToolError::Io(error.to_string()))?;
        stdin.flush().map_err(|error| ToolError::Io(error.to_string()))?;
    }
    let deadline = Instant::now().checked_add(Duration::from_millis(timeout_ms.clamp(1, 300_000))).unwrap_or_else(Instant::now);
    loop {
        let text = shell.output.lock().map(|guard| guard.clone()).unwrap_or_default();
        if let Some(index) = text.find(&marker) { return Ok(crate::truncate(text.get(..index).unwrap_or_default().trim_end().to_owned())); }
        if Instant::now() > deadline { return Err(ToolError::Io(format!("persistent shell timed out after {timeout_ms}ms"))); }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Close all persistent shells (tests/shutdown).
pub fn close_all() {
    if let Ok(mut guard) = shells().lock() {
        for shell in guard.values() { if let Ok(mut child) = shell.child.lock() { let _ = child.kill(); let _ = child.wait(); } }
        guard.clear();
    }
}
