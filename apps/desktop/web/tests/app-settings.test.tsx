import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "../src/App";
import { parseSettings } from "../src/protocol";
import { META, readyTransport } from "./fixtures";

afterEach(() => {
  localStorage.removeItem("zk.enterprise.session");
});

describe("settings integration", () => {
  it("applies locale and theme after saving and shows a toast", async () => {
    const transport = readyTransport([]);
    render(<App transport={transport} />);

    await userEvent.click(await screen.findByRole("button", { name: "打开设置" }));
    expect(await screen.findByRole("dialog")).toBeTruthy();

    await userEvent.selectOptions(screen.getByLabelText("语言"), "en-US");
    await userEvent.selectOptions(screen.getByLabelText("主题"), "dark");
    const maxSteps = screen.getByLabelText("最大步数（1-200）");
    await userEvent.clear(maxSteps);
    await userEvent.type(maxSteps, "12");
    const temperature = screen.getByLabelText("temperature（0-2）");
    await userEvent.clear(temperature);
    await userEvent.type(temperature, "0.7");
    await userEvent.click(screen.getByLabelText("启用会话记忆"));
    await userEvent.click(screen.getByLabelText("启用定时调度"));
    const scan = screen.getByLabelText("扫描间隔（秒，5-3600）");
    await userEvent.clear(scan);
    await userEvent.type(scan, "30");
    const embedUrl = screen.getByLabelText("嵌入服务地址");
    await userEvent.type(embedUrl, "https://embed.example/v1");
    const embedModel = screen.getByLabelText("嵌入模型");
    await userEvent.type(embedModel, "text-embedding-3-small");
    const embedDims = screen.getByLabelText("向量维度（1-65536，可选）");
    await userEvent.type(embedDims, "1024");
    const toolTimeout = screen.getByLabelText("工具超时（ms，1000-600000）");
    await userEvent.clear(toolTimeout);
    await userEvent.type(toolTimeout, "60000");
    const approvalTimeout = screen.getByLabelText("审批超时（ms，1000-1800000）");
    await userEvent.clear(approvalTimeout);
    await userEvent.type(approvalTimeout, "450000");
    await userEvent.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() => {
      expect(transport.callsFor("settings.set")).toHaveLength(1);
    });
    const sentSettings = parseSettings(
      transport.callsFor("settings.set")[0]?.["settings"],
    );
    expect(sentSettings.locale).toBe("en-US");
    expect(sentSettings.theme).toBe("dark");
    expect(sentSettings.agent.max_steps).toBe(12);
    expect(sentSettings.agent.completion.temperature).toBe(0.7);
    expect(sentSettings.agent.memory.enabled).toBe(false);
    expect(sentSettings.tools).toEqual({ tool_timeout_ms: 60000, approval_timeout_ms: 450000 });
    expect(sentSettings.agent.schedule).toEqual({ enabled: true, scan_seconds: 30 });
    expect(sentSettings.embedding).toEqual({
      base_url: "https://embed.example/v1",
      api_key: "",
      model: "text-embedding-3-small",
      dims: 1024,
      dir: "",
    });

    expect(document.documentElement.lang).toBe("en-US");
    expect(document.documentElement.dataset["theme"]).toBe("dark");
    expect(await screen.findByText("Settings saved")).toBeTruthy();
    expect(screen.getAllByRole("button", { name: "New conversation" }).length).toBeGreaterThan(0);
    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });
});

describe("update check", () => {
  it("shows the available version from the update feed", async () => {
    const transport = readyTransport([]);
    transport.handle("update.check", () => ({ release: { version: "9.9.9", notes: "" } }));
    render(<App transport={transport} />);
    await userEvent.click(await screen.findByRole("button", { name: "打开设置" }));
    await userEvent.click(await screen.findByRole("button", { name: "检查更新" }));
    expect(await screen.findByText("发现新版本 9.9.9")).toBeTruthy();
  });
});


describe("enterprise settings (C7)", () => {
  it("does not render embedding/vector index fields or their key input", async () => {
    localStorage.setItem("zk.enterprise.session", JSON.stringify({
      access_token: "enterprise-token",
      refresh_token: "enterprise-refresh",
      expires_in: 900,
      user_id: "u1",
      role: "employee",
      username: "user",
      device_name: "device",
    }));
    const transport = readyTransport([]);
    transport.handle("meta.get", () => ({ ...META, edition: "enterprise" }));
    render(<App transport={transport} />);

    await userEvent.click(await screen.findByRole("button", { name: "打开设置" }));
    expect(await screen.findByRole("dialog")).toBeTruthy();
    expect(screen.queryByText("嵌入/向量索引")).toBeNull();
    expect(screen.queryByLabelText("嵌入服务地址")).toBeNull();
    expect(screen.queryByLabelText("嵌入 API Key")).toBeNull();
    // The personal-only provider key UI is also not rendered in an enterprise composition;
    // this personal-composition test only guarantees the embedding/local credential gate.
    expect(screen.getByText("调度/自动化")).toBeTruthy();
  });
});
