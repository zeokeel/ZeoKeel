import { describe, expect, it } from "vitest";
import { defaultSettings } from "@zeokeel/ui-kit";
import { formToSettings, toForm, validateSettings } from "../src/settings-validation";

describe("settings validation", () => {
  it("accepts the mock provider defaults", () => {
    const form = toForm(defaultSettings());
    expect(validateSettings(form)).toEqual({ baseUrl: null, model: null });
  });

  it("requires http(s) base_url and a model for non-mock providers", () => {
    const form = { ...toForm(defaultSettings()), providerKind: "open_ai_compatible" as const, baseUrl: "ftp://x", model: "" };
    expect(validateSettings(form).baseUrl).toBe("settingsInvalidUrl");
    expect(validateSettings(form).model).toBe("settingsInvalidModel");
  });

  it("round-trips a saved server URL (docs/19 §3.3)", () => {
    const form = { ...toForm(defaultSettings()), serverUrl: "https://zk.example" };
    expect(formToSettings(form, 1).server_url).toBe("https://zk.example");
  });
});
