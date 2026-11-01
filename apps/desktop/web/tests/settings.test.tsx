import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import {
  defaultSettings,
  UiProvider,
  type AppSettings,
  type SettingsPaneAppProps,
  type SettingsPaneContribution,
  type SettingsSaveOutcome,
} from "@zeokeel/ui-kit";
import { ProviderSettingsPane } from "@zeokeel/ui-settings-provider";
import {
  SettingsDialog,
  components,
  toForm,
  validateSettings,
  type SettingsForm,
} from "@zeokeel/ui-settings";
import { describe, expect, it, vi } from "vitest";
import { testUi } from "./ui-fixtures";

/** Mirrors the personal `SettingsPanes` list slot (generated registry). */
const PANES: readonly SettingsPaneContribution[] = [
  { id: "general", order: 10, Component: components["settings.pane.general"] },
  { id: "models", order: 20, Component: components["settings.pane.models"] },
  { id: "provider", order: 21, Component: ProviderSettingsPane },
  { id: "agent", order: 30, Component: components["settings.pane.agent"] },
  { id: "tools", order: 40, Component: components["settings.pane.tools"] },
  { id: "search", order: 50, Component: components["settings.pane.search"] },
  { id: "embedding", order: 60, Component: components["settings.pane.embedding"] },
  { id: "media", order: 65, Component: components["settings.pane.media"] },
  { id: "about", order: 90, Component: components["settings.pane.about"] },
];

const APP: SettingsPaneAppProps = {
  serverUrlLocked: false,
  gatewayModels: [],
  gatewayModel: "",
  updateText: null,
  updateBusy: false,
};

function nonMockForm(): SettingsForm {
  return { ...toForm(defaultSettings()), providerKind: "open_ai_compatible", baseUrl: "" };
}

function selectValue(label: string): string {
  const element = screen.getByLabelText(label);
  return element instanceof HTMLSelectElement ? element.value : "";
}

interface HarnessProps {
  onSaved: (settings: AppSettings) => void;
  panes?: readonly SettingsPaneContribution[];
  app?: SettingsPaneAppProps;
  outcome?: SettingsSaveOutcome;
  onReload?: () => Promise<AppSettings>;
}

function Harness(props: HarnessProps) {
  const [open, setOpen] = useState(true);
  const [settings, setSettings] = useState<AppSettings>(defaultSettings());
  const ui = testUi({
    settings,
    settingsOpen: open,
    actions: {
      saveSettings: (next: AppSettings): Promise<SettingsSaveOutcome> => {
        const outcome = props.outcome ?? { status: "saved", settings: next, revision: 1 };
        if (outcome.status === "saved") {
          setSettings(outcome.settings);
          props.onSaved(outcome.settings);
          setOpen(false);
        }
        return Promise.resolve(outcome);
      },
      reloadSettings: props.onReload ?? ((): Promise<AppSettings> => Promise.resolve(defaultSettings())),
      closeSettings: () => {
        setOpen(false);
      },
    },
  });
  return (
    <UiProvider value={ui}>
      <SettingsDialog panes={props.panes ?? PANES} app={props.app ?? APP} />
    </UiProvider>
  );
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

describe("settings dialog (paginated shell)", () => {
  it("renders ordered section navigation and the first pane by default", () => {
    render(<Harness onSaved={vi.fn()} />);
    expect(screen.getByRole("dialog")).toBeTruthy();
    const nav = screen.getByRole("navigation", { name: "设置分区" });
    expect(nav).toBeTruthy();
    // Default = first pane (general): appearance fields are visible, provider is not.
    expect(screen.getByLabelText("主题")).toBeTruthy();
    expect(screen.queryByLabelText("Provider 类型")).toBeNull();
    expect(screen.getByRole("button", { name: "通用" }).getAttribute("aria-current")).toBe("page");
  });

  it("keeps the draft across pane switches (no per-pane save)", async () => {
    render(<Harness onSaved={vi.fn()} />);
    await userEvent.selectOptions(screen.getByLabelText("语言"), "en-US");
    await userEvent.click(screen.getByRole("button", { name: "模型" }));
    const temperature = screen.getByLabelText("temperature（0-2）");
    await userEvent.clear(temperature);
    await userEvent.type(temperature, "0.4");
    // Back to general: the language edit is still in the draft.
    await userEvent.click(screen.getByRole("button", { name: "通用" }));
    expect(selectValue("语言")).toBe("en-US");
    expect(screen.queryByRole("button", { name: "保存" })).toBeTruthy();
  });

  it("jumps to the pane with the first invalid field and shows field errors", async () => {
    const captured: { settings: AppSettings | null } = { settings: null };
    const onSaved = vi.fn((settings: AppSettings) => {
      captured.settings = settings;
    });
    render(<Harness onSaved={onSaved} />);
    await userEvent.click(screen.getByRole("button", { name: "本地 Provider" }));
    await userEvent.selectOptions(screen.getByLabelText("Provider 类型"), "open_ai_compatible");
    await userEvent.click(screen.getByRole("button", { name: "保存" }));
    // Invalid base URL lives in the provider pane, which is now active.
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
    expect(captured.settings?.provider.kind).toBe("open_ai_compatible");
    expect(captured.settings?.provider.base_url).toBe("https://api.example.com/v1");
    expect(captured.settings?.provider.model).toBe("gpt-4o-mini");
  });

  it("shows inline range errors on the agent pane", async () => {
    const onSaved = vi.fn();
    render(<Harness onSaved={onSaved} />);
    await userEvent.click(screen.getByRole("button", { name: "Agent" }));
    const maxSteps = screen.getByLabelText("最大步数（1-200）");
    await userEvent.clear(maxSteps);
    await userEvent.type(maxSteps, "0");
    await userEvent.click(screen.getByRole("button", { name: "保存" }));
    expect(screen.getByText("数值超出允许范围")).toBeTruthy();
    expect(onSaved).not.toHaveBeenCalled();
  });

  it("round-trips the web-search endpoint settings (B11)", async () => {
    const captured: { settings: AppSettings | null } = { settings: null };
    const onSaved = vi.fn((settings: AppSettings) => {
      captured.settings = settings;
    });
    render(<Harness onSaved={onSaved} />);

    await userEvent.click(screen.getByRole("button", { name: "网络与检索" }));
    const baseUrl = screen.getByLabelText("搜索服务地址（仅 https）");
    await userEvent.type(baseUrl, "http://search.example");
    await userEvent.click(screen.getByRole("button", { name: "保存" }));
    expect(screen.getByText("请输入以 https:// 开头的合法地址")).toBeTruthy();
    expect(onSaved).not.toHaveBeenCalled();

    await userEvent.clear(baseUrl);
    await userEvent.type(baseUrl, "https://search.example/api");
    const timeout = screen.getByLabelText("请求超时（ms，1000-120000）");
    await userEvent.clear(timeout);
    await userEvent.type(timeout, "30000");
    await userEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledTimes(1);
    });
    expect(captured.settings?.web_search).toEqual({
      base_url: "https://search.example/api",
      timeout_ms: 30000,
    });
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

  it("keeps the draft and offers reload on a revision conflict", async () => {
    const onSaved = vi.fn();
    const onReload = vi.fn((): Promise<AppSettings> => Promise.resolve(defaultSettings()));
    render(
      <Harness onSaved={onSaved} outcome={{ status: "conflict" }} onReload={onReload} />,
    );
    await userEvent.selectOptions(screen.getByLabelText("主题"), "dark");
    await userEvent.click(screen.getByRole("button", { name: "保存" }));
    expect(await screen.findByText("设置已在别处更新，请重新加载后再保存。")).toBeTruthy();
    // Draft is preserved (theme select still dark) and nothing was saved.
    expect(selectValue("主题")).toBe("dark");
    expect(onSaved).not.toHaveBeenCalled();
    // Reload re-reads the document and clears the banner.
    await userEvent.click(screen.getByRole("button", { name: "重新加载" }));
    await waitFor(() => {
      expect(onReload).toHaveBeenCalledTimes(1);
    });
    expect(screen.queryByText("设置已在别处更新，请重新加载后再保存。")).toBeNull();
    expect(selectValue("主题")).toBe("auto");
  });

  it("enterprise pane locks the channel address and exposes logout + gateway model (C14)", async () => {
    const enterprisePanes: readonly SettingsPaneContribution[] = [
      { id: "enterprise", order: 80, Component: components["settings.pane.enterprise"] },
    ];
    const onLogout = vi.fn();
    render(
      <Harness
        onSaved={vi.fn()}
        panes={enterprisePanes}
        app={{
          ...APP,
          serverUrlLocked: true,
          gatewayModels: ["gw-model-a"],
          gatewayModel: "gw-model-a",
          onLogout,
          onSelectGatewayModel: vi.fn(),
        }}
      />,
    );
    const address = screen.getByLabelText("登录服务地址");
    expect(address instanceof HTMLInputElement && address.disabled).toBe(true);
    expect(screen.getByText("该地址由渠道锁定，不可修改。")).toBeTruthy();
    expect(screen.getByLabelText("企业模型")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "退出登录" }));
    expect(onLogout).toHaveBeenCalledTimes(1);
  });

  it("renders an empty state (no white screen) when no pane is contributed", () => {
    render(<Harness onSaved={vi.fn()} panes={[]} />);
    expect(screen.getByText("当前构建没有可用的设置分区。")).toBeTruthy();
    expect(screen.getByRole("button", { name: "恢复默认" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "保存" })).toBeTruthy();
  });
});
