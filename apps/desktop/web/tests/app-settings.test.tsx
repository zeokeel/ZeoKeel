import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { defaultSettings } from "@zeokeel/ui-kit";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "../src/App";
import { parseSettings } from "../src/protocol";
import { readyTransport } from "./fixtures";

afterEach(() => {
  localStorage.removeItem("zk.enterprise.session");
});

async function openSettings(): Promise<void> {
  await userEvent.click(await screen.findByRole("button", { name: "打开设置" }));
  expect(await screen.findByRole("dialog")).toBeTruthy();
}

async function pane(name: string): Promise<void> {
  await userEvent.click(screen.getByRole("button", { name }));
}

describe("settings integration", () => {
  it("applies locale and theme after saving and shows a toast", async () => {
    const transport = readyTransport([]);
    render(<App transport={transport} />);

    await openSettings();

    // Pane 10 · general: language + theme.
    await userEvent.selectOptions(screen.getByLabelText("语言"), "en-US");
    await userEvent.selectOptions(screen.getByLabelText("主题"), "dark");

    // Pane 20 · models: completion parameters.
    await pane("模型");
    const temperature = screen.getByLabelText("temperature（0-2）");
    await userEvent.clear(temperature);
    await userEvent.type(temperature, "0.7");

    // Pane 30 · agent: steps, memory, scheduler.
    await pane("Agent");
    const maxSteps = screen.getByLabelText("最大步数（1-200）");
    await userEvent.clear(maxSteps);
    await userEvent.type(maxSteps, "12");
    await userEvent.click(screen.getByLabelText("启用会话记忆"));
    await userEvent.click(screen.getByLabelText("启用定时调度"));
    const scan = screen.getByLabelText("扫描间隔（秒，5-3600）");
    await userEvent.clear(scan);
    await userEvent.type(scan, "30");

    // Pane 40 · tools & approval timeouts.
    await pane("工具与审批");
    const toolTimeout = screen.getByLabelText("工具超时（ms，1000-600000）");
    await userEvent.clear(toolTimeout);
    await userEvent.type(toolTimeout, "60000");
    const approvalTimeout = screen.getByLabelText("审批超时（ms，1000-1800000）");
    await userEvent.clear(approvalTimeout);
    await userEvent.type(approvalTimeout, "450000");

    // Pane 60 · embedding (personal-only contribution).
    await pane("嵌入与向量索引");
    await userEvent.type(screen.getByLabelText("嵌入服务地址"), "https://embed.example/v1");
    await userEvent.type(screen.getByLabelText("嵌入模型"), "text-embedding-3-small");
    await userEvent.type(screen.getByLabelText("向量维度（1-65536，可选）"), "1024");

    // Pane 65 · media (personal-only contribution).
    await pane("媒体能力");
    await userEvent.type(screen.getByLabelText("媒体服务地址"), "https://media.example/v1");
    await userEvent.type(screen.getByLabelText("图像模型"), "dall-e-3");
    await userEvent.type(screen.getByLabelText("音频模型"), "tts-1");
    await userEvent.type(screen.getByLabelText("转写模型"), "whisper-1");
    await userEvent.type(screen.getByLabelText("重排模型"), "rerank-1");

    // Single bottom save for the whole document.
    await userEvent.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() => {
      expect(transport.callsFor("settings.set")).toHaveLength(1);
    });
    const saveCall = transport.callsFor("settings.set")[0];
    expect(saveCall?.["expected_revision"]).toBe(0);
    const sentSettings = parseSettings(saveCall?.["settings"]);
    expect(sentSettings.locale).toBe("en-US");
    expect(sentSettings.theme).toBe("dark");
    expect(sentSettings.agent.max_steps).toBe(12);
    expect(sentSettings.agent.completion.temperature).toBe(0.7);
    expect(sentSettings.agent.memory.enabled).toBe(false);
    expect(sentSettings.tools).toEqual({ tool_timeout_ms: 60000, approval_timeout_ms: 450000, approval_policy: null });
    expect(sentSettings.agent.schedule).toEqual({ enabled: true, scan_seconds: 30 });
    expect(sentSettings.embedding).toEqual({
      base_url: "https://embed.example/v1",
      api_key: "",
      model: "text-embedding-3-small",
      dims: 1024,
      dir: "",
    });
    expect(sentSettings.media).toEqual({
      base_url: "https://media.example/v1",
      api_key: "",
      image_model: "dall-e-3",
      audio_model: "tts-1",
      transcription_model: "whisper-1",
      rerank_model: "rerank-1",
    });

    expect(document.documentElement.lang).toBe("en-US");
    expect(document.documentElement.dataset["theme"]).toBe("dark");
    expect(await screen.findByText("Settings saved")).toBeTruthy();
    expect(screen.getAllByRole("button", { name: "New conversation" }).length).toBeGreaterThan(0);
    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });

  it("keeps the draft on a revision conflict and reloads the latest settings", async () => {
    const transport = readyTransport([]);
    render(<App transport={transport} />);

    await openSettings();
    await userEvent.selectOptions(screen.getByLabelText("主题"), "dark");
    // The host rejects the stale revision (another window saved first).
    transport.fail("settings.set", "settings/revision-conflict: expected 0, current 1");
    await userEvent.click(screen.getByRole("button", { name: "保存" }));

    expect(await screen.findByText("设置已在别处更新，请重新加载后再保存。")).toBeTruthy();
    // Draft preserved, dialog open, nothing committed to the app state.
    expect(screen.getByRole("dialog")).toBeTruthy();
    expect(screen.getByLabelText("主题")).toBeTruthy();

    // Reload re-reads settings.get and clears the conflict banner.
    transport.handle("settings.get", () => ({
      settings: { ...defaultSettings(), theme: "light" },
      revision: 1,
    }));
    await userEvent.click(screen.getByRole("button", { name: "重新加载" }));
    await waitFor(() => {
      expect(screen.queryByText("设置已在别处更新，请重新加载后再保存。")).toBeNull();
    });
    expect(screen.getByRole("dialog")).toBeTruthy();
  });
});
