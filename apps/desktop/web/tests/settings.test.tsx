import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { defaultSettings, UiProvider, type AppSettings } from "@zeokeel/ui-kit";
import { ProviderSettings } from "@zeokeel/ui-settings-provider";
import {
  SettingsDialog,
  toForm,
  validateSettings,
  type SettingsForm,
} from "@zeokeel/ui-settings";
import { describe, expect, it, vi } from "vitest";
import { testUi } from "./ui-fixtures";

function nonMockForm(): SettingsForm {
  return { ...toForm(defaultSettings()), providerKind: "open_ai_compatible", baseUrl: "" };
}

describe("settings validation", () => {
  it("accepts the mock provider defaults", () => {
    const errors = validateSettings(toForm(defaultSettings()));
    expect(errors.baseUrl).toBeNull();
    expect(errors.model).toBeNull();
  });

  it("requires an http(s) base_url for non-mock providers", () => {
    const form = nonMockForm();
    expect(validateSettings(form).baseUrl).toBe("settingsInvalidUrl");
    const withUrl: SettingsForm = { ...form, baseUrl: "https://api.example.com/v1" };
    expect(validateSettings(withUrl).baseUrl).toBeNull();
  });

  it("requires a non-empty model", () => {
    const form: SettingsForm = {
      ...nonMockForm(),
      baseUrl: "https://api.example.com/v1",
      model: " ",
    };
    expect(validateSettings(form).model).toBe("settingsInvalidModel");
    const fixed: SettingsForm = { ...form, model: "gpt-4o-mini" };
    expect(validateSettings(fixed).model).toBeNull();
  });
});

function Harness(props: { onSaved: (settings: AppSettings) => void }) {
  const [open, setOpen] = useState(true);
  const [settings, setSettings] = useState<AppSettings>(defaultSettings());
  const ui = testUi({
    settings,
    settingsOpen: open,
    actions: {
      saveSettings: (next: AppSettings): Promise<void> => {
        setSettings(next);
        props.onSaved(next);
        setOpen(false);
        return Promise.resolve();
      },
      closeSettings: () => {
        setOpen(false);
      },
    },
  });
  return (
    <UiProvider value={ui}>
      <SettingsDialog renderProvider={(props) => <ProviderSettings {...props} />} />
    </UiProvider>
  );
}

describe("settings dialog", () => {
  it("shows field errors and does not save invalid values", async () => {
    const captured: { settings: AppSettings | null } = { settings: null };
    const onSaved = vi.fn((settings: AppSettings) => {
      captured.settings = settings;
    });
    render(<Harness onSaved={onSaved} />);
    expect(screen.getByRole("dialog")).toBeTruthy();

    await userEvent.selectOptions(screen.getByLabelText("Provider 类型"), "open_ai_compatible");
    await userEvent.click(screen.getByRole("button", { name: "保存" }));
    expect(screen.getByText("请输入以 http:// 或 https:// 开头的合法地址")).toBeTruthy();
    expect(onSaved).not.toHaveBeenCalled();

    const baseUrl = screen.getByLabelText("Base URL");
    await userEvent.type(baseUrl, "not-a-url");
    await userEvent.click(screen.getByRole("button", { name: "保存" }));
    expect(onSaved).not.toHaveBeenCalled();

    await userEvent.clear(baseUrl);
    await userEvent.type(baseUrl, "https://api.example.com/v1");
    const model = screen.getByLabelText("模型");
    await userEvent.clear(model);
    await userEvent.click(screen.getByRole("button", { name: "保存" }));
    expect(screen.getByText("模型名不能为空")).toBeTruthy();
    expect(onSaved).not.toHaveBeenCalled();

    await userEvent.type(model, "gpt-4o-mini");
    await userEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledTimes(1);
    });
    expect(captured.settings).not.toBeNull();
    expect(captured.settings?.provider.kind).toBe("open_ai_compatible");
    expect(captured.settings?.provider.base_url).toBe("https://api.example.com/v1");
    expect(captured.settings?.provider.model).toBe("gpt-4o-mini");
  });

  it("rejects invalid agent settings before saving", async () => {
    const onSaved = vi.fn();
    render(<Harness onSaved={onSaved} />);
    const maxSteps = screen.getByLabelText("最大步数（1-200）");
    await userEvent.clear(maxSteps);
    await userEvent.type(maxSteps, "0");
    await userEvent.click(screen.getByRole("button", { name: "保存" }));
    expect(screen.getByText("数值超出允许范围")).toBeTruthy();
    expect(onSaved).not.toHaveBeenCalled();
  });

  it("closes on Escape", async () => {
    const onSaved = vi.fn();
    render(<Harness onSaved={onSaved} />);
    expect(screen.getByRole("dialog")).toBeTruthy();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });
});
