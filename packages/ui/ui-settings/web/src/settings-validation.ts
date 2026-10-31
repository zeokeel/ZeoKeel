/**
 * Settings form validation — pure, shared by the dialog and tests.
 * Errors are i18n keys (the dialog translates them via useUi().t).
 */
import type { AgentSettings, AppSettings, Locale, ProviderKind, Theme, ToolChoiceSetting, WebSearchSettings } from "@zeokeel/ui-kit";

export type ToolChoiceMode = "auto" | "none" | "required" | "specific";
export type ParallelMode = "default" | "true" | "false";
export type ReasoningEffort = "" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max";
export type PromptCache = "" | "short" | "long";

export interface SettingsForm {
  providerKind: ProviderKind;
  baseUrl: string;
  model: string;
  apiKey: string;
  theme: Theme;
  locale: Locale;
  autoApproveReadOnly: boolean;
  serverUrl: string;
  // Agent config (strings so the form can represent "provider default" as empty).
  maxSteps: string;
  temperature: string;
  maxTokens: string;
  topP: string;
  seed: string;
  stop: string;
  toolChoiceMode: ToolChoiceMode;
  toolChoiceTools: string;
  parallelToolCalls: ParallelMode;
  reasoningEffort: ReasoningEffort;
  promptCache: PromptCache;
  memoryEnabled: boolean;
  memoryDir: string;
  hooksEnabled: boolean;
  hooksLog: string;
  scheduleEnabled: boolean;
  scanSeconds: string;
  // Tool/approval timeouts (ms, strings for empty/invalid UI states).
  toolTimeoutMs: string;
  approvalTimeoutMs: string;
  // Embedding / vector index (empty dims means "provider default").
  embeddingBaseUrl: string;
  embeddingApiKey: string;
  embeddingModel: string;
  embeddingDims: string;
  embeddingDir: string;
  // Media capabilities (personal only; empty fields fall back to env).
  mediaBaseUrl: string;
  mediaApiKey: string;
  mediaImageModel: string;
  mediaAudioModel: string;
  mediaTranscriptionModel: string;
  mediaRerankModel: string;
  // Web-search tool endpoint (empty base URL falls back to env/default).
  webSearchBaseUrl: string;
  webSearchTimeoutMs: string;
}

export type SettingsFieldError = string | null;

export interface SettingsErrors {
  baseUrl: SettingsFieldError;
  model: SettingsFieldError;
  maxSteps: SettingsFieldError;
  temperature: SettingsFieldError;
  maxTokens: SettingsFieldError;
  topP: SettingsFieldError;
  seed: SettingsFieldError;
  toolChoiceTools: SettingsFieldError;
  scanSeconds: SettingsFieldError;
  toolTimeoutMs: SettingsFieldError;
  approvalTimeoutMs: SettingsFieldError;
  embeddingBaseUrl: SettingsFieldError;
  embeddingDims: SettingsFieldError;
  mediaBaseUrl: SettingsFieldError;
  webSearchBaseUrl: SettingsFieldError;
  webSearchTimeoutMs: SettingsFieldError;
}

export const EMPTY_SETTINGS_ERRORS: SettingsErrors = {
  baseUrl: null,
  model: null,
  maxSteps: null,
  temperature: null,
  maxTokens: null,
  topP: null,
  seed: null,
  toolChoiceTools: null,
  scanSeconds: null,
  toolTimeoutMs: null,
  approvalTimeoutMs: null,
  embeddingBaseUrl: null,
  embeddingDims: null,
  mediaBaseUrl: null,
  webSearchBaseUrl: null,
  webSearchTimeoutMs: null,
};

function isHttpUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}

/** `tool-web-search` only speaks https; plain http is rejected before saving. */
function isHttpsUrl(value: string): boolean {
  try {
    return new URL(value).protocol === "https:";
  } catch {
    return false;
  }
}

function numberField(
  raw: string,
  options: { min: number; max: number; integer: boolean },
): { value: number | null; error: SettingsFieldError } {
  const trimmed = raw.trim();
  if (trimmed === "") {
    return { value: null, error: null };
  }
  const parsed = Number(trimmed);
  if (!Number.isFinite(parsed)) {
    return { value: null, error: "settingsInvalidNumber" };
  }
  if (options.integer && !Number.isSafeInteger(parsed)) {
    return { value: null, error: "settingsInvalidInteger" };
  }
  if (parsed < options.min || parsed > options.max) {
    return { value: null, error: "settingsInvalidRange" };
  }
  return { value: parsed, error: null };
}

function toolChoiceOf(form: SettingsForm): ToolChoiceSetting {
  if (form.toolChoiceMode === "specific") {
    const tools = form.toolChoiceTools
      .split(",")
      .map((tool) => tool.trim())
      .filter((tool) => tool !== "");
    return tools.length === 0 ? { mode: "auto" } : { mode: "specific", tools };
  }
  return { mode: form.toolChoiceMode };
}

function parallelOf(mode: ParallelMode): boolean | null {
  return mode === "default" ? null : mode === "true";
}

function stopOf(raw: string): string[] {
  return raw
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "");
}

function webSearchOf(form: SettingsForm): WebSearchSettings {
  return {
    base_url: form.webSearchBaseUrl.trim(),
    timeout_ms:
      numberField(form.webSearchTimeoutMs, { min: 1000, max: 120000, integer: true }).value ??
      15000,
  };
}

export function toForm(settings: AppSettings): SettingsForm {
  const agent = settings.agent;
  const toolChoice = agent.completion.tool_choice;
  const parallel: ParallelMode =
    agent.completion.parallel_tool_calls === null
      ? "default"
      : agent.completion.parallel_tool_calls
        ? "true"
        : "false";
  const effort: ReasoningEffort =
    agent.completion.reasoning_effort === "minimal" ||
    agent.completion.reasoning_effort === "low" ||
    agent.completion.reasoning_effort === "medium" ||
    agent.completion.reasoning_effort === "high" ||
    agent.completion.reasoning_effort === "xhigh" ||
    agent.completion.reasoning_effort === "max"
      ? agent.completion.reasoning_effort
      : "";
  const cache: PromptCache =
    agent.completion.prompt_cache === "short" || agent.completion.prompt_cache === "long"
      ? agent.completion.prompt_cache
      : "";
  return {
    providerKind: settings.provider.kind,
    baseUrl: settings.provider.base_url,
    model: settings.provider.model,
    apiKey: settings.provider.api_key,
    theme: settings.theme,
    locale: settings.locale,
    autoApproveReadOnly: settings.auto_approve_read_only,
    serverUrl: settings.server_url,
    maxSteps: String(agent.max_steps),
    temperature: agent.completion.temperature === null ? "" : String(agent.completion.temperature),
    maxTokens: agent.completion.max_tokens === null ? "" : String(agent.completion.max_tokens),
    topP: agent.completion.top_p === null ? "" : String(agent.completion.top_p),
    seed: agent.completion.seed === null ? "" : String(agent.completion.seed),
    stop: agent.completion.stop.join("\n"),
    toolChoiceMode: toolChoice.mode,
    toolChoiceTools: toolChoice.mode === "specific" ? toolChoice.tools.join(", ") : "",
    parallelToolCalls: parallel,
    reasoningEffort: effort,
    promptCache: cache,
    memoryEnabled: agent.memory.enabled,
    memoryDir: agent.memory.dir,
    hooksEnabled: agent.hooks.enabled,
    hooksLog: agent.hooks.log,
    scheduleEnabled: agent.schedule.enabled,
    scanSeconds: String(agent.schedule.scan_seconds),
    toolTimeoutMs: String(settings.tools.tool_timeout_ms),
    approvalTimeoutMs: String(settings.tools.approval_timeout_ms),
    embeddingBaseUrl: settings.embedding.base_url,
    embeddingApiKey: settings.embedding.api_key,
    embeddingModel: settings.embedding.model,
    embeddingDims: settings.embedding.dims === null ? "" : String(settings.embedding.dims),
    embeddingDir: settings.embedding.dir,
    mediaBaseUrl: settings.media.base_url,
    mediaApiKey: settings.media.api_key,
    mediaImageModel: settings.media.image_model,
    mediaAudioModel: settings.media.audio_model,
    mediaTranscriptionModel: settings.media.transcription_model,
    mediaRerankModel: settings.media.rerank_model,
    webSearchBaseUrl: settings.web_search.base_url,
    webSearchTimeoutMs: String(settings.web_search.timeout_ms),
  };
}

export function validateSettings(form: SettingsForm): SettingsErrors {
  const maxSteps = numberField(form.maxSteps, { min: 1, max: 200, integer: true });
  const temperature = numberField(form.temperature, { min: 0, max: 2, integer: false });
  const maxTokens = numberField(form.maxTokens, { min: 1, max: Number.MAX_SAFE_INTEGER, integer: true });
  const topP = numberField(form.topP, { min: 0, max: 1, integer: false });
  const seed = numberField(form.seed, { min: 0, max: Number.MAX_SAFE_INTEGER, integer: true });
  const toolChoiceTools =
    form.toolChoiceMode === "specific" && form.toolChoiceTools.trim() === ""
      ? "settingsInvalidToolChoiceTools"
      : null;
  const toolTimeout = numberField(form.toolTimeoutMs, { min: 1000, max: 600000, integer: true });
  const approvalTimeout = numberField(form.approvalTimeoutMs, { min: 1000, max: 1800000, integer: true });
  const scan = numberField(form.scanSeconds, { min: 5, max: 3600, integer: true });
  const embeddingDims = form.embeddingDims.trim() === ""
    ? { value: null, error: null }
    : numberField(form.embeddingDims, { min: 1, max: 65536, integer: true });
  const searchTimeout = numberField(form.webSearchTimeoutMs, { min: 1000, max: 120000, integer: true });
  return {
    baseUrl:
      form.providerKind === "mock" || isHttpUrl(form.baseUrl.trim()) ? null : "settingsInvalidUrl",
    model: form.model.trim() === "" ? "settingsInvalidModel" : null,
    maxSteps: maxSteps.error ?? (maxSteps.value === null ? "settingsInvalidInteger" : null),
    temperature: temperature.error,
    maxTokens: maxTokens.error,
    topP: topP.error,
    seed: seed.error,
    toolChoiceTools,
    scanSeconds: scan.error ?? (scan.value === null ? "settingsInvalidInteger" : null),
    toolTimeoutMs: toolTimeout.error ?? (toolTimeout.value === null ? "settingsInvalidInteger" : null),
    approvalTimeoutMs: approvalTimeout.error ?? (approvalTimeout.value === null ? "settingsInvalidInteger" : null),
    embeddingBaseUrl:
      form.embeddingBaseUrl.trim() === "" || isHttpUrl(form.embeddingBaseUrl.trim())
        ? null
        : "settingsInvalidUrl",
    embeddingDims: embeddingDims.error ?? (embeddingDims.value === null && form.embeddingDims.trim() !== "" ? "settingsInvalidInteger" : null),
    mediaBaseUrl:
      form.mediaBaseUrl.trim() === "" || isHttpUrl(form.mediaBaseUrl.trim())
        ? null
        : "settingsInvalidUrl",
    webSearchBaseUrl:
      form.webSearchBaseUrl.trim() === "" || isHttpsUrl(form.webSearchBaseUrl.trim())
        ? null
        : "settingsInvalidSecureUrl",
    webSearchTimeoutMs:
      searchTimeout.error ?? (searchTimeout.value === null ? "settingsInvalidInteger" : null),
  };
}

export function hasSettingsErrors(errors: SettingsErrors): boolean {
  return Object.values(errors).some((error) => error !== null);
}

function agentOf(form: SettingsForm): AgentSettings {
  const maxSteps = numberField(form.maxSteps, { min: 1, max: 200, integer: true }).value ?? 50;
  return {
    max_steps: maxSteps,
    completion: {
      temperature: numberField(form.temperature, { min: 0, max: 2, integer: false }).value,
      max_tokens: numberField(form.maxTokens, { min: 1, max: Number.MAX_SAFE_INTEGER, integer: true }).value,
      top_p: numberField(form.topP, { min: 0, max: 1, integer: false }).value,
      seed: numberField(form.seed, { min: 0, max: Number.MAX_SAFE_INTEGER, integer: true }).value,
      stop: stopOf(form.stop),
      tool_choice: toolChoiceOf(form),
      parallel_tool_calls: parallelOf(form.parallelToolCalls),
      reasoning_effort: form.reasoningEffort === "" ? null : form.reasoningEffort,
      prompt_cache: form.promptCache === "" ? null : form.promptCache,
    },
    memory: { enabled: form.memoryEnabled, dir: form.memoryDir.trim() },
    hooks: { enabled: form.hooksEnabled, log: form.hooksLog.trim() },
    schedule: {
      enabled: form.scheduleEnabled,
      scan_seconds: numberField(form.scanSeconds, { min: 5, max: 3600, integer: true }).value ?? 10,
    },
  };
}

export function formToSettings(form: SettingsForm, schema: number): AppSettings {
  return {
    schema,
    provider: {
      kind: form.providerKind,
      base_url: form.baseUrl.trim(),
      model: form.model.trim(),
      api_key: form.apiKey,
    },
    theme: form.theme,
    locale: form.locale,
    auto_approve_read_only: form.autoApproveReadOnly,
    server_url: form.serverUrl.trim(),
    agent: agentOf(form),
    tools: {
      tool_timeout_ms: numberField(form.toolTimeoutMs, { min: 1000, max: 600000, integer: true }).value ?? 120000,
      approval_timeout_ms: numberField(form.approvalTimeoutMs, { min: 1000, max: 1800000, integer: true }).value ?? 300000,
    },
    embedding: {
      base_url: form.embeddingBaseUrl.trim(),
      api_key: form.embeddingApiKey,
      model: form.embeddingModel.trim(),
      dims: form.embeddingDims.trim() === ""
        ? null
        : numberField(form.embeddingDims, { min: 1, max: 65536, integer: true }).value,
      dir: form.embeddingDir.trim(),
    },
    media: {
      base_url: form.mediaBaseUrl.trim(),
      api_key: form.mediaApiKey,
      image_model: form.mediaImageModel.trim(),
      audio_model: form.mediaAudioModel.trim(),
      transcription_model: form.mediaTranscriptionModel.trim(),
      rerank_model: form.mediaRerankModel.trim(),
    },
    web_search: webSearchOf(form),
  };
}
