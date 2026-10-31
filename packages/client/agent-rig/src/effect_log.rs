//! Append-only JSONL effect log (B9 subset).
//!
//! Wraps rig's [`RecordBuffer`] and writes one redacted `EffectRecord` JSON object
//! per resolved effect. This records completions/tool effects for diagnostics and
//! audit-style replay; it does **not** implement live run checkpoint/resume (that rig
//! API is not host-reachable — see docs/40 known limitations).
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::Mutex;

use rig_core::effect::{Answered, EffectId, EffectKind, HandlerDescriptor, HandlerKey};
use rig_core::error::ErrorReport;
use rig_core::message::Origin;
use rig_core::serve::{DispatchContext, RecordBuffer, Recorder};
use rig_core::streaming::{Item, StreamEvent};
use rig_core::tool::ToolResultContext;

/// JSONL writer recorder; errors while writing are dropped after a warning so a
/// diagnostics log can never fail a model turn.
pub(crate) struct JsonlEffectRecorder {
    buffer: RecordBuffer,
    writer: Mutex<Option<BufWriter<File>>>,
}

impl JsonlEffectRecorder {
    /// Open (append) the JSONL effect log at `path`; creates parent directories.
    pub(crate) fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("effect log dir {}: {error}", parent.display()))?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|error| format!("effect log {}: {error}", path.display()))?;
        Ok(Self {
            buffer: RecordBuffer::new().timed(),
            writer: Mutex::new(Some(BufWriter::new(file))),
        })
    }
}

impl Recorder for JsonlEffectRecorder {
    fn handlers(&self, handlers: Vec<HandlerDescriptor>) {
        self.buffer.handlers(handlers);
    }

    fn begin(&self, id: EffectId, key: HandlerKey, kind: EffectKind, origin: DispatchContext) {
        self.buffer.begin(id, key, kind, origin);
    }

    fn discard(&self, id: EffectId) {
        self.buffer.discard(id);
    }

    fn patch(&self, id: EffectId, kind: EffectKind) {
        self.buffer.patch(id, kind);
    }

    fn keep_events(&self) -> bool {
        self.buffer.keep_events()
    }

    fn event(&self, id: EffectId, item: &Item<StreamEvent>) {
        self.buffer.event(id, item);
    }

    fn stream_error(&self, id: EffectId, error: &ErrorReport) {
        self.buffer.stream_error(id, error);
    }

    fn origin(&self, id: EffectId, origin: &Origin) {
        self.buffer.origin(id, origin);
    }

    fn tool_output(&self, id: EffectId, output: ToolResultContext) {
        self.buffer.tool_output(id, output);
    }

    fn resolve(&self, id: EffectId, outcome: Result<Answered, ErrorReport>) {
        self.buffer.resolve(id, outcome);
        let Some(record) = self.buffer.take(id) else {
            return;
        };
        let Ok(json) = serde_json::to_string(&record) else {
            return;
        };
        let line = rig_core::redact::json_text(&json);
        let Ok(mut guard) = self.writer.lock() else {
            return;
        };
        let Some(writer) = guard.as_mut() else {
            return;
        };
        if writer.write_all(line.as_bytes()).is_err() || writer.write_all(b"\n").is_err() {
            eprintln!("agent-rig: effect log write failed; disabling further effect-log writes");
            *guard = None;
            return;
        }
        let _ = writer.flush();
    }
}
