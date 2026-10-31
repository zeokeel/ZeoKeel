import { describe, expect, it } from "vitest";
import { defaultSettings } from "@zeokeel/ui-kit";
import { EMPTY_SETTINGS_ERRORS, formToSettings, hasSettingsErrors, toForm, validateSettings } from "../src/settings-validation";

describe("settings validation", () => {
  it("accepts the defaults (provider + agent)", () => {
    const form = toForm(defaultSettings());
    expect(validateSettings(form)).toEqual(EMPTY_SETTINGS_ERRORS);
    expect(hasSettingsErrors(validateSettings(form))).toBe(false);
  });

  it("requires http(s) base_url and a model for non-mock providers", () => {
    const form = { ...toForm(defaultSettings()), providerKind: "open_ai_compatible" as const, baseUrl: "ftp://x", model: "" };
    expect(validateSettings(form).baseUrl).toBe("settingsInvalidUrl");
    expect(validateSettings(form).model).toBe("settingsInvalidModel");
  });

  it("validates agent numeric ranges and tool_choice=specific", () => {
    const base = toForm(defaultSettings());
    expect(validateSettings({ ...base, maxSteps: "0" }).maxSteps).toBe("settingsInvalidRange");
    expect(validateSettings({ ...base, maxSteps: "201" }).maxSteps).toBe("settingsInvalidRange");
    expect(validateSettings({ ...base, maxSteps: "abc" }).maxSteps).toBe("settingsInvalidNumber");
    expect(validateSettings({ ...base, temperature: "2.5" }).temperature).toBe("settingsInvalidRange");
    expect(validateSettings({ ...base, topP: "1.5" }).topP).toBe("settingsInvalidRange");
    expect(validateSettings({ ...base, maxTokens: "0" }).maxTokens).toBe("settingsInvalidRange");
    expect(validateSettings({ ...base, seed: "-1" }).seed).toBe("settingsInvalidRange");
    expect(validateSettings({ ...base, toolChoiceMode: "specific" as const, toolChoiceTools: "  " }).toolChoiceTools).toBe("settingsInvalidToolChoiceTools");
    expect(validateSettings({ ...base, toolChoiceMode: "specific" as const, toolChoiceTools: "fs.read, fs.write" }).toolChoiceTools).toBeNull();
    expect(validateSettings({ ...base, toolTimeoutMs: "999" }).toolTimeoutMs).toBe("settingsInvalidRange");
    expect(validateSettings({ ...base, toolTimeoutMs: "600001" }).toolTimeoutMs).toBe("settingsInvalidRange");
    expect(validateSettings({ ...base, approvalTimeoutMs: "999" }).approvalTimeoutMs).toBe("settingsInvalidRange");
    expect(validateSettings({ ...base, approvalTimeoutMs: "1800001" }).approvalTimeoutMs).toBe("settingsInvalidRange");
    expect(validateSettings({ ...base, toolTimeoutMs: "60000", approvalTimeoutMs: "450000" }).toolTimeoutMs).toBeNull();
  });

  it("round-trips all agent settings into the AppSettings contract shape", () => {
    const form = {
      ...toForm(defaultSettings()),
      maxSteps: "12",
      temperature: "0.7",
      maxTokens: "2048",
      topP: "0.9",
      seed: "42",
      stop: "STOP\n###",
      toolChoiceMode: "specific" as const,
      toolChoiceTools: "fs.read, fs.write",
      parallelToolCalls: "true" as const,
      reasoningEffort: "high" as const,
      promptCache: "long" as const,
      memoryEnabled: false,
      memoryDir: "/tmp/zk-memory",
      hooksEnabled: false,
      hooksLog: "/tmp/zk-hooks.jsonl",
      scheduleEnabled: true,
      scanSeconds: "30",
      toolTimeoutMs: "60000",
      approvalTimeoutMs: "450000",
    };
    expect(validateSettings(form)).toEqual(EMPTY_SETTINGS_ERRORS);
    const settings = formToSettings(form, 1);
    expect(settings.agent).toEqual({
      max_steps: 12,
      completion: {
        temperature: 0.7,
        max_tokens: 2048,
        top_p: 0.9,
        seed: 42,
        stop: ["STOP", "###"],
        tool_choice: { mode: "specific", tools: ["fs.read", "fs.write"] },
        parallel_tool_calls: true,
        reasoning_effort: "high",
        prompt_cache: "long",
      },
      memory: { enabled: false, dir: "/tmp/zk-memory" },
      hooks: { enabled: false, log: "/tmp/zk-hooks.jsonl" },
      schedule: { enabled: true, scan_seconds: 30 },
    });
    expect(settings.tools).toEqual({ tool_timeout_ms: 60000, approval_timeout_ms: 450000 });
    expect(toForm(settings).maxSteps).toBe("12");
    expect(toForm(settings).toolChoiceMode).toBe("specific");
  });

  it("serializes provider-default agent values as null/auto/empty", () => {
    const settings = formToSettings(toForm(defaultSettings()), 1);
    expect(settings.agent.completion.temperature).toBeNull();
    expect(settings.agent.completion.max_tokens).toBeNull();
    expect(settings.agent.completion.top_p).toBeNull();
    expect(settings.agent.completion.seed).toBeNull();
    expect(settings.agent.completion.parallel_tool_calls).toBeNull();
    expect(settings.agent.completion.tool_choice).toEqual({ mode: "auto" });
    expect(settings.agent.completion.stop).toEqual([]);
    expect(settings.agent.max_steps).toBe(50);
    expect(settings.agent.schedule).toEqual({ enabled: false, scan_seconds: 10 });
    expect(toForm(defaultSettings()).scanSeconds).toBe("10");
    expect(toForm(defaultSettings()).scheduleEnabled).toBe(false);
    expect(settings.tools).toEqual({ tool_timeout_ms: 120000, approval_timeout_ms: 300000 });
  });

  it("validates the schedule scan interval range (5..3600)", () => {
    const base = toForm(defaultSettings());
    expect(validateSettings({ ...base, scanSeconds: "4" }).scanSeconds).toBe("settingsInvalidRange");
    expect(validateSettings({ ...base, scanSeconds: "3601" }).scanSeconds).toBe("settingsInvalidRange");
    expect(validateSettings({ ...base, scanSeconds: "5" }).scanSeconds).toBeNull();
    expect(validateSettings({ ...base, scanSeconds: "3600" }).scanSeconds).toBeNull();
    expect(validateSettings({ ...base, scanSeconds: "1.5" }).scanSeconds).toBe("settingsInvalidInteger");
  });

  it("keeps a saved server URL (docs/19 §3.3)", () => {
    const form = { ...toForm(defaultSettings()), serverUrl: "https://zk.example" };
    expect(formToSettings(form, 1).server_url).toBe("https://zk.example");
  });
});
