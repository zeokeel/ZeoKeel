//! Persistent shell sessions (bash for `bash_persistent`/`terminal`, pwsh for `pwsh_persistent`).
//!
//! One long-lived process per `(shell flavor, workspace, conversation, session id)` with piped
//! stdin/stdout/stderr. Sessions keep cwd/env across calls and are killed explicitly or at
//! [`close_all`] (process shutdown/tests). Session ids are bounded and validated here so tools
//! cannot grow the registry without limit.
#![forbid(unsafe_code)]
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

/// Session id used by the single-session legacy APIs when the caller does not name one.
pub const DEFAULT_SESSION: &str = "default";
/// Maximum persistent sessions per shell flavor per conversation.
pub const MAX_SESSIONS_PER_CONVERSATION: usize = 8;
/// Longest accepted session id.
const MAX_SESSION_ID_LEN: usize = 64;
/// How long a freshly spawned shell gets to complete its warm-up marker.
const WARMUP_TIMEOUT_MS: u64 = 10_000;

/// Persistent shell flavor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShellKind {
    /// bash (`--noprofile --norc`).
    Bash,
    /// PowerShell 7 (`pwsh -NoProfile -NonInteractive -Command -`).
    Pwsh,
}

impl ShellKind {
    /// Stable flavor name used in status output and diagnostics.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Pwsh => "pwsh",
        }
    }

    fn program(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Pwsh => "pwsh",
        }
    }

    fn args(self) -> &'static [&'static str] {
        match self {
            Self::Bash => &["--noprofile", "--norc"],
            Self::Pwsh => &["-NoProfile", "-NonInteractive", "-NoLogo", "-Command", "-"],
        }
    }

    /// Command that writes `marker` on its own line after the user command finished.
    fn marker_command(self, marker: &str) -> String {
        match self {
            Self::Bash => format!("printf '\\n{marker}\\n'"),
            Self::Pwsh => format!("Write-Output \"`n{marker}`n\""),
        }
    }
}

struct Session {
    stdin: Mutex<ChildStdin>,
    child: Mutex<Child>,
    output: Arc<Mutex<String>>,
    seq: Mutex<u64>,
    kind: ShellKind,
}

type SessionKey = (ShellKind, String, String, String);

static SHELLS: OnceLock<Mutex<BTreeMap<SessionKey, Arc<Session>>>> = OnceLock::new();

fn sessions() -> &'static Mutex<BTreeMap<SessionKey, Arc<Session>>> {
    SHELLS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn lock_error() -> ToolError {
    ToolError::Io("persistent shell lock poisoned".to_owned())
}

fn key(context: &TurnContext, kind: ShellKind, session: &str) -> SessionKey {
    (
        kind,
        context.workspace_root.clone(),
        context.conversation_id.clone(),
        session.to_owned(),
    )
}

fn validate_session_id(session: &str) -> Result<&str, ToolError> {
    let session = session.trim();
    if session.is_empty() {
        return Err(ToolError::InvalidArgs("session id is empty".to_owned()));
    }
    if session.len() > MAX_SESSION_ID_LEN {
        return Err(ToolError::InvalidArgs(format!(
            "session id is longer than {MAX_SESSION_ID_LEN} bytes"
        )));
    }
    if !session
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
    {
        return Err(ToolError::InvalidArgs(
            "session id may contain only ASCII letters, digits, '-', '_' and '.'".to_owned(),
        ));
    }
    Ok(session)
}

fn spawn_error(kind: ShellKind, error: &std::io::Error) -> ToolError {
    if kind == ShellKind::Pwsh
        && matches!(
            error.kind(),
            std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied
        )
    {
        return ToolError::Denied(format!(
            "TOOL_UNSUPPORTED_PLATFORM: pwsh is not available ({error})"
        ));
    }
    ToolError::Io(error.to_string())
}

fn pump<R: Read + Send + 'static>(mut pipe: R, target: Arc<Mutex<String>>) {
    std::thread::spawn(move || {
        let mut buf = [0_u8; 4096];
        loop {
            match pipe.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if let Ok(mut guard) = target.lock() {
                        guard.push_str(&String::from_utf8_lossy(buf.get(..n).unwrap_or_default()));
                    }
                }
            }
        }
    });
}

fn kill(session: &Session) {
    if let Ok(mut child) = session.child.lock() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn is_alive(session: &Session) -> bool {
    match session.child.lock() {
        Ok(mut child) => matches!(child.try_wait(), Ok(None)),
        Err(_) => false,
    }
}

fn forget(session: &Arc<Session>) {
    if let Ok(mut guard) = sessions().lock() {
        guard.retain(|_, value| !Arc::ptr_eq(value, session));
    }
}

fn spawn(context: &TurnContext, kind: ShellKind) -> Result<Arc<Session>, ToolError> {
    let root = crate::workspace_root(context)?;
    let mut command = Command::new(kind.program());
    for argument in kind.args() {
        command.arg(argument);
    }
    let mut child = command
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| spawn_error(kind, &error))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| ToolError::Io("shell stdin unavailable".to_owned()))?;
    let output = Arc::new(Mutex::new(String::new()));
    if let Some(pipe) = child.stdout.take() {
        pump(pipe, Arc::clone(&output));
    }
    if let Some(pipe) = child.stderr.take() {
        pump(pipe, Arc::clone(&output));
    }
    let session = Arc::new(Session {
        stdin: Mutex::new(stdin),
        child: Mutex::new(child),
        output,
        seq: Mutex::new(0),
        kind,
    });
    // Warm up: consume startup output and prove the shell reads commands before the first
    // user command is sent. A shell that fails to start is killed, never registered.
    if let Err(error) = send_marker(&session, None, WARMUP_TIMEOUT_MS) {
        kill(&session);
        return Err(error);
    }
    Ok(session)
}

fn send_marker(
    session: &Session,
    command: Option<&str>,
    timeout_ms: u64,
) -> Result<String, ToolError> {
    let seq = {
        let mut guard = session
            .seq
            .lock()
            .map_err(|_| ToolError::Io("shell seq poisoned".to_owned()))?;
        *guard = (*guard).saturating_add(1);
        *guard
    };
    let marker = format!("__ZK_DONE_{seq}__");
    if let Ok(mut guard) = session.output.lock() {
        guard.clear();
    }
    {
        let mut stdin = session
            .stdin
            .lock()
            .map_err(|_| ToolError::Io("shell stdin poisoned".to_owned()))?;
        if let Some(command) = command {
            writeln!(stdin, "{command}").map_err(|error| ToolError::Io(error.to_string()))?;
        }
        writeln!(stdin, "{}", session.kind.marker_command(&marker))
            .map_err(|error| ToolError::Io(error.to_string()))?;
        stdin
            .flush()
            .map_err(|error| ToolError::Io(error.to_string()))?;
    }
    let deadline = Instant::now()
        .checked_add(Duration::from_millis(timeout_ms.clamp(1, 300_000)))
        .unwrap_or_else(Instant::now);
    loop {
        let text = session
            .output
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default();
        if let Some(index) = text.find(&marker) {
            let cut = text.get(..index).unwrap_or_default();
            return Ok(crate::truncate(cut.trim_end().to_owned()));
        }
        if Instant::now() > deadline {
            return Err(ToolError::Io(format!(
                "persistent shell timed out after {timeout_ms}ms"
            )));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn ensure_session(
    context: &TurnContext,
    kind: ShellKind,
    session: &str,
) -> Result<Arc<Session>, ToolError> {
    let session = validate_session_id(session)?;
    let key = key(context, kind, session);
    let existing = {
        let guard = sessions().lock().map_err(|_| lock_error())?;
        guard.get(&key).map(Arc::clone)
    };
    if let Some(existing) = existing {
        if is_alive(&existing) {
            return Ok(existing);
        }
        forget(&existing);
    }
    let _ = open_session(context, kind, session)?;
    let guard = sessions().lock().map_err(|_| lock_error())?;
    guard
        .get(&key)
        .map(Arc::clone)
        .ok_or_else(|| ToolError::Io("persistent session disappeared".to_owned()))
}

/// Ensure a persistent session exists, spawning it when absent.
///
/// Returns `true` when the session was already open and alive.
///
/// # Errors
///
/// Invalid session ids, the per-conversation cap, workspace resolution and shell startup
/// failures (including `TOOL_UNSUPPORTED_PLATFORM` for a missing `pwsh`) are reported.
pub fn open_session(
    context: &TurnContext,
    kind: ShellKind,
    session: &str,
) -> Result<bool, ToolError> {
    let session = validate_session_id(session)?;
    let key = key(context, kind, session);
    let existing = {
        let guard = sessions().lock().map_err(|_| lock_error())?;
        guard.get(&key).map(Arc::clone)
    };
    if let Some(existing) = existing {
        if is_alive(&existing) {
            return Ok(true);
        }
        forget(&existing);
    }
    {
        let guard = sessions().lock().map_err(|_| lock_error())?;
        let count = guard
            .keys()
            .filter(|(flavor, workspace, conversation, _)| {
                *flavor == kind
                    && *workspace == context.workspace_root
                    && *conversation == context.conversation_id
            })
            .count();
        if count >= MAX_SESSIONS_PER_CONVERSATION {
            return Err(ToolError::Denied(format!(
                "session limit reached: at most {MAX_SESSIONS_PER_CONVERSATION} {} sessions per conversation",
                kind.name()
            )));
        }
    }
    let spawned = spawn(context, kind)?;
    let mut guard = sessions().lock().map_err(|_| lock_error())?;
    if let Some(existing) = guard.get(&key) {
        kill(&spawned);
        return Ok(is_alive(existing));
    }
    guard.insert(key, spawned);
    Ok(false)
}

/// Run a command in a persistent session, spawning the session when absent.
///
/// # Errors
///
/// Reports empty commands, invalid session ids, spawn failures and command timeouts.
pub fn run_in_session(
    context: &TurnContext,
    kind: ShellKind,
    session: &str,
    command: &str,
    timeout_ms: u64,
) -> Result<String, ToolError> {
    if command.trim().is_empty() {
        return Err(ToolError::InvalidArgs("command is empty".to_owned()));
    }
    let shell = ensure_session(context, kind, session)?;
    match send_marker(&shell, Some(command), timeout_ms) {
        Ok(output) => Ok(output),
        Err(error) => {
            // A failed write/timeout leaves the session unusable; drop it so the next call
            // opens a fresh shell instead of reusing a dead pipe.
            forget(&shell);
            kill(&shell);
            Err(error)
        }
    }
}

/// Close one persistent session; returns whether it was open.
///
/// # Errors
///
/// Reports invalid session ids and a poisoned session lock.
pub fn close_session(
    context: &TurnContext,
    kind: ShellKind,
    session: &str,
) -> Result<bool, ToolError> {
    let session = validate_session_id(session)?;
    let key = key(context, kind, session);
    let removed = sessions().lock().map_err(|_| lock_error())?.remove(&key);
    if let Some(shell) = removed {
        kill(&shell);
        return Ok(true);
    }
    Ok(false)
}

/// Whether a persistent session is currently open.
#[must_use]
pub fn is_session_open(context: &TurnContext, kind: ShellKind, session: &str) -> bool {
    let Ok(session) = validate_session_id(session) else {
        return false;
    };
    let key = key(context, kind, session);
    sessions()
        .lock()
        .map(|guard| guard.get(&key).is_some_and(|shell| is_alive(shell)))
        .unwrap_or(false)
}

/// Session ids open for this conversation and shell flavor, sorted.
#[must_use]
pub fn list_sessions(context: &TurnContext, kind: ShellKind) -> Vec<String> {
    sessions()
        .lock()
        .map(|guard| {
            guard
                .keys()
                .filter(|(flavor, workspace, conversation, _)| {
                    *flavor == kind
                        && *workspace == context.workspace_root
                        && *conversation == context.conversation_id
                })
                .map(|(_, _, _, session)| session.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Run a command in the legacy default bash session.
///
/// # Errors
///
/// See [`run_in_session`].
pub fn run(context: &TurnContext, command: &str, timeout_ms: u64) -> Result<String, ToolError> {
    run_in_session(
        context,
        ShellKind::Bash,
        DEFAULT_SESSION,
        command,
        timeout_ms,
    )
}

/// Close the legacy default bash session.
///
/// # Errors
///
/// See [`close_session`].
pub fn close(context: &TurnContext) -> Result<bool, ToolError> {
    close_session(context, ShellKind::Bash, DEFAULT_SESSION)
}

/// Whether the legacy default bash session is open.
#[must_use]
pub fn is_open(context: &TurnContext) -> bool {
    is_session_open(context, ShellKind::Bash, DEFAULT_SESSION)
}

/// Close all persistent sessions (process shutdown/tests).
pub fn close_all() {
    if let Ok(mut guard) = sessions().lock() {
        for shell in guard.values() {
            kill(shell);
        }
        guard.clear();
    }
}

/// Generic persistent-session tool: `terminal` for bash, `pwsh_persistent` for pwsh.
///
/// Both shipped tool packages are thin constructors over this type so the action schema, session
/// semantics and error mapping have exactly one implementation (C17).
pub struct SessionTool {
    kind: ShellKind,
}

impl SessionTool {
    /// Create the tool for one shell flavor.
    #[must_use]
    pub fn new(kind: ShellKind) -> Self {
        Self { kind }
    }

    fn name(&self) -> &'static str {
        match self.kind {
            ShellKind::Bash => "terminal",
            ShellKind::Pwsh => "pwsh_persistent",
        }
    }

    fn description(&self) -> &'static str {
        match self.kind {
            ShellKind::Bash => {
                "Open/send/close/status named persistent terminal sessions for this conversation; \
                 cwd/env persist per session. Use `open` before `send`; `status`/`list` enumerate \
                 open sessions."
            }
            ShellKind::Pwsh => {
                "Open/send/close/status named persistent PowerShell 7 sessions for this \
                 conversation; cwd/env persist per session. Use `open` before `send`. Returns \
                 TOOL_UNSUPPORTED_PLATFORM when pwsh is not installed (e.g. non-Windows hosts \
                 without PowerShell 7)."
            }
        }
    }

    fn parameters(&self) -> serde_json::Value {
        match self.kind {
            ShellKind::Bash => {
                serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["open","status","send","close","list"]},"session":{"type":"string"},"command":{"type":"string"}},"required":["action"],"additionalProperties":false})
            }
            ShellKind::Pwsh => {
                serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["open","status","send","close","list"]},"session":{"type":"string"},"command":{"type":"string"},"timeout_ms":{"type":"integer"}},"required":["action"],"additionalProperties":false})
            }
        }
    }

    fn open_command(&self) -> &'static str {
        match self.kind {
            ShellKind::Bash => "pwd",
            ShellKind::Pwsh => "Get-Location | Select-Object -ExpandProperty Path",
        }
    }

    fn open_timeout_ms(&self) -> u64 {
        match self.kind {
            ShellKind::Bash => 5_000,
            ShellKind::Pwsh => 10_000,
        }
    }

    fn command_timeout_ms(&self) -> u64 {
        30_000
    }

    fn session_arg<'a>(&self, args: &'a serde_json::Value) -> &'a str {
        args.get("session")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|session| !session.is_empty())
            .unwrap_or(DEFAULT_SESSION)
    }

    fn optional_session<'a>(&self, args: &'a serde_json::Value) -> Option<&'a str> {
        args.get("session")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|session| !session.is_empty())
    }

    fn handle(&self, context: &TurnContext, args: &serde_json::Value) -> Result<String, ToolError> {
        let action = args
            .get("action")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("action".to_owned()))?;
        match action {
            "open" => {
                let session = self.session_arg(args);
                let already_open = open_session(context, self.kind, session)?;
                let cwd = run_in_session(
                    context,
                    self.kind,
                    session,
                    self.open_command(),
                    self.open_timeout_ms(),
                )?;
                Ok(format!(
                    "{} open: session={session} already_open={already_open} cwd={cwd}",
                    self.name()
                ))
            }
            "status" => match self.optional_session(args) {
                Some(session) => Ok(format!(
                    "session={session} open={}",
                    is_session_open(context, self.kind, session)
                )),
                None => Ok(format!("sessions={:?}", list_sessions(context, self.kind))),
            },
            "list" => Ok(format!("sessions={:?}", list_sessions(context, self.kind))),
            "send" => {
                let session = self.session_arg(args);
                if !is_session_open(context, self.kind, session) {
                    return Err(ToolError::InvalidArgs(format!(
                        "unknown session: {session}; call open first"
                    )));
                }
                let command = args
                    .get("command")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| ToolError::InvalidArgs("command".to_owned()))?;
                let timeout_ms = if self.kind == ShellKind::Pwsh {
                    args.get("timeout_ms")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or_else(|| self.command_timeout_ms())
                } else {
                    self.command_timeout_ms()
                };
                run_in_session(context, self.kind, session, command, timeout_ms)
            }
            "close" => {
                let session = self.session_arg(args);
                Ok(format!(
                    "closed={}",
                    close_session(context, self.kind, session)?
                ))
            }
            other => Err(ToolError::InvalidArgs(format!("unknown action {other}"))),
        }
    }
}

impl Tool for SessionTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_owned(),
            description: self.description().to_owned(),
            parameters: self.parameters(),
            mutating: true,
            requires_approval: true,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(format!(
            "{} requires JSON",
            self.name()
        )))
    }
    fn call_json(
        &self,
        context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        self.handle(context, &args)
    }
}
