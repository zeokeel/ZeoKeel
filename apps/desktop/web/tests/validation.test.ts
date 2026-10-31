import { afterEach, describe, expect, it, vi } from "vitest";
import { defaultSettings } from "../src/types";
import {
  parseAgentEvent,
  parseCommandResponse,
  parseConversation,
  parseFileEntries,
  parseMessages,
  parseSettings,
  parseUiEvent,
  parseMeta,
  parseWorkspace,
  parseWorkspaceChanges,
  parseWorkspaceFiles,
  parseWorkspaces,
} from "../src/protocol";
import {
  createBoundaryAjv,
  validateAgentEvent,
  validateAppSettings,
  validateChatSendResult,
  validateCommandResponse,
  validateConversation,
  validateMessage,
  validateMetaInfo,
  validateQuestionAsked,
  validateUiEvent,
  validateWorkspace,
  validateWorkspaceChangesResult,
  validateWorkspaceFilesResult,
} from "../src/validate";

const UI_EVENT = { stream_id: "stream-1", seq: 1, kind: "agent", payload: { type: "turn_started", turn_id: "t1" } };
const AGENT_EVENT = { type: "text_delta", delta: "hello" };
const WORKSPACE = {
  id: "ws-1",
  path: "/home/test",
  title: "Home",
  is_default: true,
  created_at_ms: 1,
  updated_at_ms: 2,
};
const CONVERSATION = {
  id: "c-1",
  workspace_id: "ws-1",
  title: "Conversation",
  created_at_ms: 1,
  updated_at_ms: 2,
};
const MESSAGE = {
  id: "m-1",
  conversation_id: "c-1",
  role: "user",
  content: "hi",
  created_at_ms: 1,
};

afterEach(() => {
  vi.restoreAllMocks();
});

describe("Ajv boundary schemas (T22/C10)", () => {
  it("accepts valid IPC/event/settings/DTO payloads", () => {
    expect(validateCommandResponse({ ok: true, data: null, error: null })).toBe(true);
    expect(validateUiEvent(UI_EVENT)).toBe(true);
    expect(validateAgentEvent(AGENT_EVENT)).toBe(true);
    expect(validateAgentEvent({ type: "turn_cancelled" })).toBe(true);
    expect(validateAppSettings(defaultSettings())).toBe(true);
    expect(validateWorkspace(WORKSPACE)).toBe(true);
    expect(validateConversation(CONVERSATION)).toBe(true);
    expect(validateMessage(MESSAGE)).toBe(true);
  });

  it("rejects missing fields, wrong types and unknown agent events", () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    expect(validateCommandResponse({ data: null })).toBe(false);
    expect(validateCommandResponse({ ok: "yes" })).toBe(false);
    expect(validateUiEvent({ ...UI_EVENT, stream_id: undefined })).toBe(false);
    expect(validateUiEvent({ ...UI_EVENT, seq: "1" })).toBe(false);
    expect(validateAgentEvent({ type: "not_a_known_event" })).toBe(false);
    expect(validateAgentEvent({ type: "text_delta" })).toBe(false);
    expect(validateAppSettings({ ...defaultSettings(), schema: "1" })).toBe(false);
    expect(validateAppSettings({ ...defaultSettings(), provider: { kind: "nope" } })).toBe(false);
    expect(validateWorkspace({ ...WORKSPACE, path: 42 })).toBe(false);
    expect(validateConversation({ ...CONVERSATION, title: undefined })).toBe(false);
    expect(validateMessage({ ...MESSAGE, content: null })).toBe(false);
  });
});

describe("protocol boundary rejects invalid data after schema validation", () => {
  it("maps malformed command envelopes to a safe error", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    expect(parseCommandResponse({ data: {} })).toEqual({ ok: false, data: null, error: "invalid response" });
    expect(warn).toHaveBeenCalled();
  });

  it("drops malformed UiEvent and AgentEvent payloads", () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    expect(parseUiEvent({ stream_id: "s", seq: 1, kind: "agent" })).toBeNull();
    expect(parseUiEvent({ ...UI_EVENT, payload: undefined })).toBeNull();
    expect(parseAgentEvent({ type: "unknown_event", foo: 1 })).toBeNull();
  });

  it("falls back to defaults for invalid settings and rejects malformed DTOs", () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    expect(parseSettings({ schema: 1 }).schema).toBe(defaultSettings().schema);
    expect(parseSettings({ ...defaultSettings(), provider: { kind: "bogus" } })).toEqual(defaultSettings());
    expect(parseWorkspace({ ...WORKSPACE, created_at_ms: "1" })).toBeNull();
    expect(parseConversation({ ...CONVERSATION, title: 5 })).toBeNull();
    expect(parseMessages({ messages: [{ ...MESSAGE, content: 5 }] })).toBeNull();
    expect(parseWorkspaces({ workspaces: [WORKSPACE, { id: "bad" }] })).toBeNull();
  });

  it("accepts valid settings/DTO payloads (no regression)", () => {
    expect(parseSettings(defaultSettings())).toEqual(defaultSettings());
    expect(parseUiEvent(UI_EVENT)).toEqual(UI_EVENT);
    expect(parseAgentEvent(AGENT_EVENT)).toEqual(AGENT_EVENT);
    expect(parseWorkspace(WORKSPACE)).toEqual(WORKSPACE);
    expect(parseConversation(CONVERSATION)).toEqual(CONVERSATION);
  });

  it("parses AppSettings.agent and AgentReply.usage from the contract shape", () => {
    const settings = parseSettings({
      ...defaultSettings(),
      agent: {
        max_steps: 12,
        completion: {
          temperature: 0.7,
          max_tokens: 2048,
          top_p: 0.9,
          seed: 42,
          stop: ["STOP"],
          tool_choice: { mode: "specific", tools: ["fs.read"] },
          parallel_tool_calls: true,
          reasoning_effort: "high",
          prompt_cache: "long",
        },
        memory: { enabled: false, dir: "/tmp/mem" },
        hooks: { enabled: false, log: "/tmp/hooks.jsonl" },
      },
    });
    expect(settings.agent.max_steps).toBe(12);
    expect(settings.agent.completion.tool_choice).toEqual({ mode: "specific", tools: ["fs.read"] });
    expect(settings.agent.memory.dir).toBe("/tmp/mem");

    const completed = parseAgentEvent({
      type: "turn_completed",
      reply: {
        text: "done",
        tool_calls: [],
        usage: { input_tokens: 5, output_tokens: 2, total_tokens: 7, cached_tokens: 1, reasoning_tokens: 0, cost_micros: 10 },
      },
    });
    expect(completed).not.toBeNull();
    expect(completed?.type).toBe("turn_completed");
    if (completed?.type === "turn_completed") {
      expect(completed.reply.usage?.total_tokens).toBe(7);
    }

    const resolved = parseAgentEvent({ type: "approval_resolved", approval_id: "a1", decision: "reject" });
    expect(resolved).toEqual({ type: "approval_resolved", approval_id: "a1", decision: "reject" });
  });
  it("rejects a turn_completed reply whose usage is missing required counters", () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    expect(validateAgentEvent({
      type: "turn_completed",
      reply: { text: "x", tool_calls: [], usage: { input_tokens: 1, output_tokens: 2, total_tokens: 3 } },
    })).toBe(false);
    expect(parseAgentEvent({
      type: "turn_completed",
      reply: { text: "x", tool_calls: [], usage: { input_tokens: 1, output_tokens: 2, total_tokens: 3 } },
    })).toBeNull();
    expect(validateAgentEvent({
      type: "turn_completed",
      reply: { text: "x", tool_calls: [], usage: { input_tokens: 0, output_tokens: 0, total_tokens: 0, cached_tokens: 0, reasoning_tokens: 0 } },
    })).toBe(true);
  });

  it("validates schedule settings via the regenerated app_settings schema", () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const current = defaultSettings();
    expect(validateAppSettings({ ...current, agent: { ...current.agent, schedule: { enabled: true, scan_seconds: 30 } } })).toBe(true);
    expect(validateAppSettings({ ...current, agent: { ...current.agent, schedule: { enabled: false, scan_seconds: 10 } } })).toBe(true);
    expect(validateAppSettings({ ...current, agent: { ...current.agent, schedule: { enabled: "yes", scan_seconds: 10 } } })).toBe(false);
    expect(validateAppSettings({ ...current, agent: { ...current.agent, schedule: { enabled: true, scan_seconds: "30" } } })).toBe(false);
  });

  it("validates tools timeouts via the generated app_settings schema", () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    expect(validateAppSettings({ ...defaultSettings(), tools: { tool_timeout_ms: 60000, approval_timeout_ms: 450000 } })).toBe(true);
    expect(validateAppSettings({ ...defaultSettings(), tools: { tool_timeout_ms: "60000", approval_timeout_ms: 450000 } })).toBe(false);
    expect(validateAppSettings({ ...defaultSettings(), tools: { tool_timeout_ms: -1, approval_timeout_ms: 450000 } })).toBe(false);
  });

  it("parses schedule settings with serde defaults for old settings", () => {
    const current = defaultSettings();
    expect(parseSettings({ ...current, agent: { ...current.agent, schedule: undefined } }).agent.schedule).toEqual({ enabled: false, scan_seconds: 10 });
    expect(parseSettings({ ...current, agent: { ...current.agent, schedule: { enabled: true, scan_seconds: 30 } } }).agent.schedule).toEqual({ enabled: true, scan_seconds: 30 });
    expect(parseSettings({ ...current, agent: { ...current.agent, schedule: { enabled: true } } }).agent.schedule).toEqual({ enabled: true, scan_seconds: 10 });
  });

  it("parses tools timeouts with serde defaults for old settings", () => {
    expect(parseSettings({ ...defaultSettings(), tools: undefined }).tools).toEqual({ tool_timeout_ms: 120000, approval_timeout_ms: 300000 });
    expect(parseSettings({ ...defaultSettings(), tools: { tool_timeout_ms: 60000 } }).tools).toEqual({ tool_timeout_ms: 60000, approval_timeout_ms: 300000 });
  });

  it("parses meta.get.agent_config_error for the boot warning", () => {
    expect(parseMeta({ edition: "personal", version: "1", packages: [], agent_config_error: "agent.max_steps must be within 1..=200" }).agentConfigError).toBe("agent.max_steps must be within 1..=200");
    expect(parseMeta({ edition: "personal", version: "1", packages: [] }).agentConfigError).toBeNull();
  });

  it("defaults missing AppSettings.agent fields (old settings files)", () => {
    const settings = parseSettings({ ...defaultSettings(), agent: undefined });
    expect(settings.agent.max_steps).toBe(50);
    expect(settings.agent.completion.tool_choice).toEqual({ mode: "auto" });
    expect(settings.agent.memory.enabled).toBe(true);
  });
});


describe("Ajv strictness (C19 P2 regression)", () => {
  it("rejects unknown keywords/formats at compile time instead of silently ignoring them", () => {
    const ajv = createBoundaryAjv();
    expect(() => ajv.compile({ type: "string", format: "not-a-registered-format" })).toThrow(/unknown format/);
    expect(() => ajv.compile({ type: "string", futureKeyword: true })).toThrow(/unknown keyword/);
    // Every production boundary schema must compile under strict mode (module load would fail otherwise).
    expect(() => ajv.compile({ type: "integer", format: "uint64" })).not.toThrow();
  });

  it("enforces explicit numeric format semantics (uint/int bounds, finite doubles)", () => {
    const ajv = createBoundaryAjv();
    const uint64 = ajv.compile({ type: "integer", format: "uint64" });
    expect(uint64(5)).toBe(true);
    expect(uint64(-1)).toBe(false);
    expect(uint64(1.5)).toBe(false);
    expect(uint64(Number.MAX_SAFE_INTEGER + 1)).toBe(false);
    const int32 = ajv.compile({ type: "integer", format: "int32" });
    expect(int32(-2_147_483_648)).toBe(true);
    expect(int32(2_147_483_648)).toBe(false);
    const double = ajv.compile({ type: "number", format: "double" });
    expect(double(0.5)).toBe(true);
    expect(double(Number.POSITIVE_INFINITY)).toBe(false);
    expect(double(Number.NaN)).toBe(false);
  });

  it("keeps unknown settings fields forward-compatible (docs/04:257, docs/18 §5)", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const future = { ...defaultSettings(), future_setting: { nested: true } };
    expect(validateAppSettings(future)).toBe(true);
    expect(parseSettings(future)).toEqual(defaultSettings());
    expect(warn).not.toHaveBeenCalled();
  });
});


describe("round-3 T22 boundary DTOs", () => {
  const FILE = { path: "demo-data/hello.txt", depth: 1, kind: "file" };

  it("validates workspace.files results and rejects the wrong container/shape", () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    expect(validateWorkspaceFilesResult({ items: [FILE] })).toBe(true);
    expect(validateWorkspaceFilesResult({ items: [] })).toBe(true);
    expect(parseFileEntries({ items: [FILE] })).toEqual([FILE]);
    expect(validateWorkspaceFilesResult({ entries: [FILE] })).toBe(false);
    expect(validateWorkspaceFilesResult({ items: [{ path: "x", kind: "file" }] })).toBe(false);
    expect(validateWorkspaceFilesResult({ items: [{ path: "x", depth: 0, kind: "link" }] })).toBe(false);
    expect(parseFileEntries({ entries: [FILE] })).toBeNull();
    // Generated WorkspaceFilesResult/FileEntry types are used (badge is part of the wire schema).
    const withBadge = { ...FILE, badge: "M" };
    const parsed = parseWorkspaceFiles({ items: [FILE, withBadge] });
    expect(parsed?.items).toEqual([FILE, withBadge]);
    expect(parseFileEntries({ items: [FILE] })?.[0]?.kind).toBe("file");
  });

  it("validates meta.get and falls back to safe defaults on bad payloads", () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    expect(validateMetaInfo({ edition: "personal", version: "1.0.0", packages: ["a"] })).toBe(true);
    expect(validateMetaInfo({ edition: 1, version: "1.0.0", packages: [] })).toBe(false);
    expect(parseMeta({ edition: 1, version: "1.0.0", packages: [] })).toEqual({ edition: "personal", version: "0.0.0", packages: [], agentConfigError: null });
  });

  it("validates chat.send results", () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    expect(validateChatSendResult({ stream_id: "s1", conversation_id: "c1", workspace_id: "w1" })).toBe(true);
    expect(validateChatSendResult({ stream_id: "s1", workspace_id: "w1" })).toBe(false);
  });

  it("validates ask-user question payloads (and via agent_event question_asked)", () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const valid = { question_id: "q1", questions: [{ id: "name", question: "Name?" }] };
    expect(validateQuestionAsked(valid)).toBe(true);
    expect(validateQuestionAsked({ question_id: "q1", questions: [{ question: "Name?" }] })).toBe(false);
    expect(validateQuestionAsked({ question_id: "q1", questions: [{ id: "n", question: "Name?", options: [{ label: "A" }] }] })).toBe(true);
    expect(validateAgentEvent({ type: "question_asked", ...valid })).toBe(true);
    expect(validateAgentEvent({ type: "question_asked", question_id: "q1", questions: [{ question: "Name?" }] })).toBe(false);

    // parseAgentEvent walks the generated agent_question_asked/QuestionDef path (no dead validator).
    const parsed = parseAgentEvent({ type: "question_asked", ...valid });
    expect(parsed).toEqual({ type: "question_asked", question_id: "q1", questions: [{ id: "name", question: "Name?" }] });
    expect(parseAgentEvent({ type: "question_asked", question_id: "q1", questions: [{ question: "Name?" }] })).toBeNull();
    expect(parseAgentEvent({
      type: "question_asked",
      question_id: "q1",
      questions: [{ id: "n", question: "Name?", header: "H", multi_select: true, options: [{ label: "A" }, { label: "B", description: "desc" }] }],
    })).toEqual({
      type: "question_asked",
      question_id: "q1",
      questions: [{ id: "n", question: "Name?", header: "H", multi_select: true, options: [{ label: "A" }, { label: "B", description: "desc" }] }],
    });
  });
});


describe("workspace.changes boundary (round-4)", () => {
  it("validates WorkspaceChangesResult and maps items+warning", () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const value = {
      items: [{ path: "src/main.rs", add: 1, del: 0 }],
      warning: "not a git repository or git status failed: exit status: 128",
    };
    expect(validateWorkspaceChangesResult(value)).toBe(true);
    expect(parseWorkspaceChanges(value)).toEqual({
      items: [{ path: "src/main.rs", add: 1, del: 0 }],
      warning: "not a git repository or git status failed: exit status: 128",
    });
    expect(parseWorkspaceChanges({ items: [], warning: null })).toEqual({ items: [], warning: null });
  });

  it("rejects malformed changes payloads", () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    expect(validateWorkspaceChangesResult({ changes: [] })).toBe(false);
    expect(validateWorkspaceChangesResult({ items: [{ path: "x", add: "1", del: 0 }] })).toBe(false);
    expect(validateWorkspaceChangesResult({ items: [{ path: "x", add: 1 }] })).toBe(false);
    expect(parseWorkspaceChanges({ entries: [] })).toBeNull();
  });
});
