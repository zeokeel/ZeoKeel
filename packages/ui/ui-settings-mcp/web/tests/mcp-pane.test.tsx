/** @vitest-environment jsdom */
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { McpPane } from "../src/index";
import { defaultSettings, type McpServerConfig, type McpTestStatusResult, type SettingsErrors, type SettingsForm, type SettingsPaneProps } from "@zeokeel/ui-kit";
import { toForm } from "@zeokeel/ui-settings";

function t(key: string, vars?: Record<string, string | number>): string {
  const table: Record<string, string> = {
    settingsSectionMcp: "MCP 本地服务",
    settingsMcpEmpty: "尚未配置本地 MCP server。",
    settingsMcpAdd: "添加 MCP server",
    settingsMcpTest: "测试连接",
    settingsMcpEdit: "编辑",
    settingsMcpDelete: "删除",
    settingsMcpProbing: "探测中…",
    settingsMcpTestOk: "连接成功（{count} 个工具）",
    settingsMcpHint: "hint",
    settingsMcpInvalid: "invalid",
    settingsMcpDuplicateId: "dup",
    settingsMcpId: "ID",
    settingsMcpName: "名称",
    settingsMcpTransport: "传输",
    settingsMcpCommand: "命令",
    settingsMcpStdio: "stdio",
    settingsMcpArgs: "参数",
    settingsMcpEnv: "环境变量",
    settingsMcpSaveRow: "应用",
    settingsMcpCancel: "取消",
  };
  const template = table[key] ?? key;
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    vars !== undefined && name in vars ? String(vars[name]) : match,
  );
}

const errors: SettingsErrors = {
  baseUrl: null, model: null, maxSteps: null, temperature: null, maxTokens: null, topP: null,
  seed: null, toolChoiceTools: null, scanSeconds: null, toolTimeoutMs: null,
  approvalTimeoutMs: null, embeddingBaseUrl: null, embeddingDims: null, mediaBaseUrl: null,
  webSearchBaseUrl: null, webSearchTimeoutMs: null, mcpServers: null,
};

function form(servers: McpServerConfig[]): SettingsForm {
  const base = defaultSettings();
  base.mcp.servers = servers;
  return toForm(base);
}

function server(): McpServerConfig {
  return {
    id: "fake",
    name: "Fake MCP",
    transport: "stdio",
    command: "python3",
    args: [],
    env: {},
    url: "",
    timeout_ms: 0,
    oauth_required: false,
  };
}

describe("MCP pane", () => {
  it("shows the empty state and adds/edits draft rows", async () => {
    let draft = form([]);
    const update: SettingsPaneProps["update"] = (key, value) => {
      if (key === "mcpServers" && Array.isArray(value)) draft = form(value);
    };
    const view = render(
      <McpPane
        t={t}
        draft={draft}
        update={update}
        errors={errors}
        saving={false}
        app={{ serverUrlLocked: false, gatewayModels: [], gatewayModel: "", updateText: null, updateBusy: false }}
      />,
    );
    expect(screen.getByText("尚未配置本地 MCP server。")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "添加 MCP server" }));
    await userEvent.type(screen.getByLabelText("ID"), "fake");
    await userEvent.type(screen.getByLabelText("名称"), "Fake");
    await userEvent.type(screen.getByLabelText("命令"), "python3");
    await userEvent.click(screen.getByRole("button", { name: "应用" }));
    expect(draft.mcpServers).toHaveLength(1);
    expect(draft.mcpServers[0]?.id).toBe("fake");
    view.unmount();
  });

  it("tests a connection through the app-injected probe", async () => {
    const draft = form([server()]);
    render(
      <McpPane
        t={t}
        draft={draft}
        update={() => undefined}
        errors={errors}
        saving={false}
        app={{
          serverUrlLocked: false,
          gatewayModels: [],
          gatewayModel: "",
          updateText: null,
          updateBusy: false,
          testMcpServer: (): Promise<McpTestStatusResult> =>
            Promise.resolve({ state: "ok", tools: ["echo"], error: null }),
        }}
      />,
    );
    await userEvent.click(screen.getByRole("button", { name: "测试连接" }));
    await waitFor(() => {
      expect(screen.getByText("连接成功（1 个工具）")).toBeTruthy();
    });
  });
});
