//! Minimal local agent loop: prompt -> provider -> optional tool call.
//!
//! @capability agent.loop

#![forbid(unsafe_code)]

use std::sync::Arc;

use zk_contract_agent::{Agent, AgentError, AgentEvent, AgentReply, EventSink, ToolCallRecord};
use zk_contract_llm::LlmProvider;
use zk_contract_tool::{Tool, TurnContext};

/// Tool directives returned by the provider start with this prefix.
const TOOL_PREFIX: &str = "TOOL ";

/// The only tool the minimal demo understands.
const FS_READ: &str = "fs.read";

/// Build the single-turn prompt; the provider answers with text or a tool directive.
fn prompt_for(input: &str) -> String {
    format!(
        "你是 ZeoKeel 最小 demo。\nUSER_INPUT:{input}\n如需读取文件只输出一行: TOOL fs.read <relative-path>"
    )
}

struct NoopSink;
impl EventSink for NoopSink {
    fn emit(&self, _event: AgentEvent) {}
}

/// Minimal agent loop: one synchronous turn, at most one tool call.
pub struct AgentLoop {
    tools: Vec<Arc<dyn Tool>>,
    llm: Arc<dyn LlmProvider>,
}

impl AgentLoop {
    /// Create a loop from replaceable tools and a replaceable provider.
    #[must_use]
    pub fn new(tools: Vec<Arc<dyn Tool>>, llm: Arc<dyn LlmProvider>) -> Self {
        Self { tools, llm }
    }

    fn find_tool(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.iter().find(|tool| tool.spec().name == name)
    }

    fn invoke(
        &self,
        tool: &Arc<dyn Tool>,
        context: Option<&TurnContext>,
        args: &str,
    ) -> Result<String, zk_contract_tool::ToolError> {
        match context {
            Some(ctx) => tool.call_in(ctx, args),
            None => tool.call(args),
        }
    }

    fn run_emitting(
        &self,
        input: &str,
        sink: &dyn EventSink,
        context: Option<&TurnContext>,
    ) -> Result<AgentReply, AgentError> {
        let output = self
            .llm
            .complete(&prompt_for(input))
            .map_err(|error| AgentError::Llm(error.to_string()))?;
        self.run_output(&output, sink, context)
    }

    fn run_output(
        &self,
        output: &str,
        sink: &dyn EventSink,
        context: Option<&TurnContext>,
    ) -> Result<AgentReply, AgentError> {
        let Some(directive) = output.trim().strip_prefix(TOOL_PREFIX) else {
            return Ok(AgentReply {
                text: output.to_owned(),
                tool_calls: Vec::new(),
            });
        };

        let (name, args) = parse_directive(directive)?;
        if name != FS_READ {
            return Err(AgentError::Tool(format!("不支持的工具: {name}")));
        }
        let tool = self
            .find_tool(FS_READ)
            .ok_or_else(|| AgentError::Tool(format!("工具未注册: {FS_READ}")))?;

        sink.emit(AgentEvent::ToolCallStarted {
            name: FS_READ.to_owned(),
            args: args.to_owned(),
        });
        let (text, record) = match self.invoke(tool, context, args) {
            Ok(result) => {
                sink.emit(AgentEvent::ToolCallFinished {
                    name: FS_READ.to_owned(),
                    result: result.clone(),
                });
                let text = format!("{FS_READ} 返回:\n{result}");
                (
                    text,
                    ToolCallRecord {
                        name: FS_READ.to_owned(),
                        args: args.to_owned(),
                        result,
                    },
                )
            }
            Err(error) => {
                let result = error.to_string();
                sink.emit(AgentEvent::ToolCallFinished {
                    name: FS_READ.to_owned(),
                    result: result.clone(),
                });
                let text = format!("调用 {FS_READ} 失败: {result}");
                (
                    text,
                    ToolCallRecord {
                        name: FS_READ.to_owned(),
                        args: args.to_owned(),
                        result,
                    },
                )
            }
        };

        Ok(AgentReply {
            text,
            tool_calls: vec![record],
        })
    }

    fn execute(
        &self,
        input: &str,
        context: Option<&TurnContext>,
    ) -> Result<AgentReply, AgentError> {
        self.run_emitting(input, &NoopSink, context)
    }
}

/// Default factory used by the generated glue (role `agent.loop`).
#[must_use]
pub fn agent(tools: Vec<Arc<dyn Tool>>, llm: Arc<dyn LlmProvider>) -> AgentLoop {
    AgentLoop::new(tools, llm)
}

impl Agent for AgentLoop {
    fn run(&self, input: &str) -> Result<AgentReply, AgentError> {
        self.execute(input, None)
    }

    fn run_in(&self, context: &TurnContext, input: &str) -> Result<AgentReply, AgentError> {
        if context.workspace_root.trim().is_empty() {
            return Err(AgentError::Tool("workspace_root is empty".to_owned()));
        }
        self.execute(input, Some(context))
    }

    fn run_stream(
        &self,
        turn_id: &str,
        input: &str,
        sink: &dyn EventSink,
    ) -> Result<AgentReply, AgentError> {
        self.run_stream_ctx(turn_id, input, sink, None)
    }

    fn run_stream_in(
        &self,
        context: &TurnContext,
        turn_id: &str,
        input: &str,
        sink: &dyn EventSink,
    ) -> Result<AgentReply, AgentError> {
        if context.workspace_root.trim().is_empty() {
            return Err(AgentError::Tool("workspace_root is empty".to_owned()));
        }
        self.run_stream_ctx(turn_id, input, sink, Some(context))
    }
}

impl AgentLoop {
    fn run_stream_ctx(
        &self,
        turn_id: &str,
        input: &str,
        sink: &dyn EventSink,
        context: Option<&TurnContext>,
    ) -> Result<AgentReply, AgentError> {
        sink.emit(AgentEvent::TurnStarted {
            turn_id: turn_id.to_owned(),
        });
        let mut buffered = String::new();
        let result = self.llm.complete_stream(&prompt_for(input), &mut |delta| {
            buffered.push_str(delta);
            // Hold the first few chars so a `TOOL ...` directive is never shown as text.
            if buffered.trim_start().starts_with(TOOL_PREFIX) {
                return;
            }
            if buffered.chars().count() >= 32 {
                sink.emit(AgentEvent::TextDelta {
                    delta: std::mem::take(&mut buffered),
                });
            }
        });
        match result {
            Ok(output) => {
                if output.trim_start().starts_with(TOOL_PREFIX) {
                    // Tool directives are represented as tool events, not text.
                } else if !buffered.is_empty() {
                    sink.emit(AgentEvent::TextDelta { delta: buffered });
                }
                let reply = self.run_output(&output, sink, context)?;
                sink.emit(AgentEvent::TurnCompleted {
                    reply: reply.clone(),
                });
                Ok(reply)
            }
            Err(error) => {
                sink.emit(AgentEvent::Error {
                    message: error.to_string(),
                });
                Err(AgentError::Llm(error.to_string()))
            }
        }
    }
}

/// Split `TOOL <name> <args>`; both parts are required.
fn parse_directive(directive: &str) -> Result<(&str, &str), AgentError> {
    let mut parts = directive.splitn(2, char::is_whitespace);
    let name = parts.next().unwrap_or("").trim();
    let args = parts.next().unwrap_or("").trim();
    if name.is_empty() || args.is_empty() {
        return Err(AgentError::Tool(format!(
            "无法解析工具指令: {TOOL_PREFIX}{directive}"
        )));
    }
    Ok((name, args))
}
