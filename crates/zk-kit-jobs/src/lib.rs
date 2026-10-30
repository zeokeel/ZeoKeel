//! In-process background job registry shared by bash/jobs tools.
//!
//! @capability kit.jobs
#![forbid(unsafe_code)]
use std::collections::BTreeMap;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use zk_contract_tool::{ToolError, TurnContext};

struct Job {
    id: String,
    command: String,
    cwd: String,
    started_ms: u64,
    stdout: Arc<Mutex<String>>,
    stderr: Arc<Mutex<String>>,
    status: Arc<Mutex<String>>,
    child: Mutex<Child>,
}

static JOBS: OnceLock<Mutex<BTreeMap<String, Arc<Job>>>> = OnceLock::new();
static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn jobs() -> &'static Mutex<BTreeMap<String, Arc<Job>>> {
    JOBS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// Start a background bash command in the workspace.
pub fn start(context: &TurnContext, command: &str, cwd: Option<&str>) -> Result<String, ToolError> {
    if command.trim().is_empty() {
        return Err(ToolError::InvalidArgs("command is empty".to_owned()));
    }
    let root = zk_kit_tools::workspace_root(context)?;
    let workdir = match cwd {
        Some(cwd) if !cwd.trim().is_empty() => zk_kit_tools::safe_path(context, cwd)?,
        _ => root,
    };
    if !workdir.is_dir() {
        return Err(ToolError::InvalidArgs("cwd is not a directory".to_owned()));
    }
    let mut child = Command::new("bash")
        .arg("-lc")
        .arg(command)
        .current_dir(&workdir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| ToolError::Io(error.to_string()))?;
    let stdout = Arc::new(Mutex::new(String::new()));
    let stderr = Arc::new(Mutex::new(String::new()));
    if let Some(mut pipe) = child.stdout.take() {
        let target = Arc::clone(&stdout);
        std::thread::spawn(move || {
            let mut buffer = [0_u8; 4096];
            loop {
                match pipe.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if let Ok(mut guard) = target.lock() {
                            guard.push_str(&String::from_utf8_lossy(
                                buffer.get(..n).unwrap_or_default(),
                            ));
                        }
                    }
                }
            }
        });
    }
    if let Some(mut pipe) = child.stderr.take() {
        let target = Arc::clone(&stderr);
        std::thread::spawn(move || {
            let mut buffer = [0_u8; 4096];
            loop {
                match pipe.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if let Ok(mut guard) = target.lock() {
                            guard.push_str(&String::from_utf8_lossy(
                                buffer.get(..n).unwrap_or_default(),
                            ));
                        }
                    }
                }
            }
        });
    }
    let id = format!(
        "job-{}",
        SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    );
    let job = Arc::new(Job {
        id: id.clone(),
        command: command.to_owned(),
        cwd: workdir.display().to_string(),
        started_ms: now_ms(),
        stdout,
        stderr,
        status: Arc::new(Mutex::new("running".to_owned())),
        child: Mutex::new(child),
    });
    jobs()
        .lock()
        .map_err(|_| ToolError::Io("job lock poisoned".to_owned()))?
        .insert(id.clone(), job);
    Ok(id)
}

fn snapshot(job: &Arc<Job>) -> serde_json::Value {
    let status = job
        .child
        .lock()
        .ok()
        .and_then(|mut child| child.try_wait().ok().flatten())
        .map(|status| {
            if status.success() {
                "completed".to_owned()
            } else {
                format!("failed:{}", status.code().unwrap_or(-1))
            }
        })
        .unwrap_or_else(|| "running".to_owned());
    if let Ok(mut guard) = job.status.lock() {
        *guard = status.clone();
    }
    let stdout = job.stdout.lock().map(|g| g.clone()).unwrap_or_default();
    let stderr = job.stderr.lock().map(|g| g.clone()).unwrap_or_default();
    serde_json::json!({"id":job.id,"command":job.command,"cwd":job.cwd,"started_ms":job.started_ms,"status":status,"stdout":zk_kit_tools::truncate(stdout),"stderr":zk_kit_tools::truncate(stderr)})
}

/// Read current output for one job.
pub fn output(id: &str) -> Result<serde_json::Value, ToolError> {
    let job = jobs()
        .lock()
        .map_err(|_| ToolError::Io("job lock poisoned".to_owned()))?
        .get(id)
        .cloned()
        .ok_or_else(|| ToolError::InvalidArgs(format!("job not found: {id}")))?;
    Ok(snapshot(&job))
}

/// List all jobs.
pub fn list() -> Result<serde_json::Value, ToolError> {
    let jobs = jobs()
        .lock()
        .map_err(|_| ToolError::Io("job lock poisoned".to_owned()))?;
    Ok(serde_json::json!(jobs
        .values()
        .map(snapshot)
        .collect::<Vec<_>>()))
}

/// Kill one job.
pub fn kill(id: &str) -> Result<serde_json::Value, ToolError> {
    let job = jobs()
        .lock()
        .map_err(|_| ToolError::Io("job lock poisoned".to_owned()))?
        .get(id)
        .cloned()
        .ok_or_else(|| ToolError::InvalidArgs(format!("job not found: {id}")))?;
    job.child
        .lock()
        .map_err(|_| ToolError::Io("job lock poisoned".to_owned()))?
        .kill()
        .map_err(|error| ToolError::Io(error.to_string()))?;
    Ok(serde_json::json!({"id":id,"killed":true}))
}
