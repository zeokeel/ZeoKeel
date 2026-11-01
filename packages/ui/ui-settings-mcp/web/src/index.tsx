/**
 * Personal-only MCP pane (`ui.settings.pane`, order 70).
 *
 * Rendered by the settings shell with the shared `SettingsPaneProps`: the pane only
 * edits the draft (`mcpServers`); the single bottom save persists through
 * `settings.set`, where Rust validates with the same local caps (two-level).
 * Test connection goes through the app-injected `testMcpServer` (mcp.test +
 * mcp.test.status polling; no-cache probe), never direct IPC.
 */
import { useState, type ReactNode } from "react";
import {
  FieldError,
  inputClass,
  PaneSection,
  type McpServerConfig,
  type McpTransport,
  type SettingsPaneProps,
} from "@zeokeel/ui-kit";

function emptyServer(): McpServerConfig {
  return {
    id: "",
    name: "",
    transport: "stdio",
    command: "",
    args: [],
    env: {},
    url: "",
    timeout_ms: 0,
    oauth_required: false,
  };
}

function transportOf(value: string): McpTransport {
  return value === "http" || value === "sse" ? value : "stdio";
}

function parseLines(raw: string): string[] {
  return raw
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "");
}

function parseEnv(raw: string): Record<string, string> {
  const env: Record<string, string> = {};
  for (const line of raw.split("\n")) {
    const index = line.indexOf("=");
    if (index <= 0) continue;
    const key = line.slice(0, index).trim();
    const value = line.slice(index + 1).trim();
    if (key !== "") env[key] = value;
  }
  return env;
}

function envText(env: Record<string, string>): string {
  return Object.entries(env)
    .map(([key, value]) => `${key}=${value}`)
    .join("\n");
}

function rowValid(server: McpServerConfig): boolean {
  if (server.id.trim() === "" || server.name.trim() === "") return false;
  if (server.transport === "stdio") return server.command.trim() !== "";
  return server.url.trim() !== "";
}

interface Editing {
  index: number | null;
  server: McpServerConfig;
}

interface ProbeView {
  index: number;
  state: "pending" | "ok" | "error";
  text: string;
}

export function McpPane(props: SettingsPaneProps): ReactNode {
  const { t, draft, update, errors, saving, app } = props;
  const servers = draft.mcpServers;
  const [editing, setEditing] = useState<Editing | null>(null);
  const [rowError, setRowError] = useState<string | null>(null);
  const [probe, setProbe] = useState<ProbeView | null>(null);

  const commit = (next: McpServerConfig[]): void => {
    update("mcpServers", next);
    setEditing(null);
    setRowError(null);
  };

  const saveRow = (): void => {
    if (editing === null) return;
    const server = editing.server;
    if (!rowValid(server)) {
      setRowError(t("settingsMcpInvalid"));
      return;
    }
    const next =
      editing.index === null
        ? [...servers, server]
        : servers.map((item, index) => (index === editing.index ? server : item));
    const ids = new Set<string>();
    for (const item of next) {
      if (ids.has(item.id.trim())) {
        setRowError(t("settingsMcpDuplicateId"));
        return;
      }
      ids.add(item.id.trim());
    }
    commit(next);
  };

  const remove = (index: number): void => {
    commit(servers.filter((_item, current) => current !== index));
    if (probe !== null && probe.index === index) setProbe(null);
  };

  const test = (index: number): void => {
    const server = servers[index];
    if (server === undefined) return;
    if (app.testMcpServer === undefined) {
      setProbe({ index, state: "error", text: t("settingsMcpTestUnavailable") });
      return;
    }
    setProbe({ index, state: "pending", text: t("settingsMcpProbing") });
    void app.testMcpServer(server).then(
      (result) => {
        if (result.state === "ok") {
          setProbe({
            index,
            state: "ok",
            text: t("settingsMcpTestOk", { count: result.tools.length }),
          });
        } else if (result.state === "pending") {
          setProbe({ index, state: "pending", text: t("settingsMcpProbing") });
        } else {
          setProbe({
            index,
            state: "error",
            text: result.error ?? t("settingsMcpTestFailed"),
          });
        }
      },
      (error: unknown) => {
        setProbe({
          index,
          state: "error",
          text: error instanceof Error ? error.message : t("settingsMcpTestFailed"),
        });
      },
    );
  };

  const field = (label: string, node: ReactNode): ReactNode => (
    <label className="flex flex-col gap-1">
      <span className="text-[12px] text-muted-foreground">{label}</span>
      {node}
    </label>
  );

  const editingServer = editing?.server;
  const editingIndex = editing?.index ?? null;
  return (
    <PaneSection title={t("settingsSectionMcp")}>
      <p className="text-[12px] text-muted-foreground sm:col-span-2">{t("settingsMcpHint")}</p>
      {servers.length === 0 ? (
        <p className="text-[12px] text-muted-foreground">{t("settingsMcpEmpty")}</p>
      ) : (
        <ul className="grid gap-2 sm:col-span-2">
          {servers.map((server, index) => (
            <li
              key={`${server.id}-${String(index)}`}
              className="flex flex-wrap items-center gap-2 rounded-document bg-muted px-3 py-2"
            >
              <span className="min-w-0 flex-1 truncate text-[13px] text-foreground">
                {server.name} <span className="text-muted-foreground">({server.transport})</span>
              </span>
              {probe !== null && probe.index === index ? (
                <span
                  className={
                    probe.state === "error"
                      ? "text-[11px] text-destructive"
                      : "text-[11px] text-muted-foreground"
                  }
                >
                  {probe.text}
                </span>
              ) : null}
              <button
                type="button"
                disabled={saving || (probe !== null && probe.state === "pending")}
                onClick={() => {
                  test(index);
                }}
                className="rounded-control border border-border px-2 py-1 text-[11px] text-foreground disabled:opacity-40"
              >
                {t("settingsMcpTest")}
              </button>
              <button
                type="button"
                disabled={saving}
                onClick={() => {
                  setEditing({ index, server: { ...server } });
                  setRowError(null);
                }}
                className="rounded-control border border-border px-2 py-1 text-[11px] text-foreground disabled:opacity-40"
              >
                {t("settingsMcpEdit")}
              </button>
              <button
                type="button"
                disabled={saving}
                onClick={() => {
                  remove(index);
                }}
                className="rounded-control border border-border px-2 py-1 text-[11px] text-destructive disabled:opacity-40"
              >
                {t("settingsMcpDelete")}
              </button>
            </li>
          ))}
        </ul>
      )}
      <FieldError error={errors.mcpServers} t={t} />
      {editingServer === undefined ? null : (
        <div className="grid gap-2 rounded-document border border-border p-2 sm:col-span-2 sm:grid-cols-2">
          {field(
            t("settingsMcpId"),
            <input
              className={inputClass}
              value={editingServer.id}
              onChange={(event) => {
                setEditing({ index: editingIndex, server: { ...editingServer, id: event.target.value } });
              }}
            />,
          )}
          {field(
            t("settingsMcpName"),
            <input
              className={inputClass}
              value={editingServer.name}
              onChange={(event) => {
                setEditing({ index: editingIndex, server: { ...editingServer, name: event.target.value } });
              }}
            />,
          )}
          {field(
            t("settingsMcpTransport"),
            <select
              className={inputClass}
              value={editingServer.transport}
              onChange={(event) => {
                setEditing({
                  index: editingIndex,
                  server: { ...editingServer, transport: transportOf(event.target.value) },
                });
              }}
            >
              <option value="stdio">{t("settingsMcpStdio")}</option>
              <option value="http">HTTP</option>
              <option value="sse">SSE</option>
            </select>,
          )}
          {editingServer.transport === "stdio"
            ? field(
                t("settingsMcpCommand"),
                <input
                  className={inputClass}
                  value={editingServer.command}
                  onChange={(event) => {
                    setEditing({
                      index: editingIndex,
                      server: { ...editingServer, command: event.target.value },
                    });
                  }}
                />,
              )
            : field(
                t("settingsMcpUrl"),
                <input
                  className={inputClass}
                  value={editingServer.url}
                  onChange={(event) => {
                    setEditing({ index: editingIndex, server: { ...editingServer, url: event.target.value } });
                  }}
                />,
              )}
          {editingServer.transport === "stdio"
            ? field(
                t("settingsMcpArgs"),
                <textarea
                  className={inputClass}
                  rows={2}
                  value={editingServer.args.join("\n")}
                  onChange={(event) => {
                    setEditing({
                      index: editingIndex,
                      server: { ...editingServer, args: parseLines(event.target.value) },
                    });
                  }}
                />,
              )
            : field(
                t("settingsMcpTimeout"),
                <input
                  className={inputClass}
                  type="number"
                  min={0}
                  max={600000}
                  value={editingServer.timeout_ms}
                  onChange={(event) => {
                    setEditing({
                      index: editingIndex,
                      server: {
                        ...editingServer,
                        timeout_ms: Number(event.target.value) || 0,
                      },
                    });
                  }}
                />,
              )}
          {field(
            t("settingsMcpEnv"),
            <textarea
              className={inputClass}
              rows={2}
              placeholder="KEY=value"
              value={envText(editingServer.env)}
              onChange={(event) => {
                setEditing({
                  index: editingIndex,
                  server: { ...editingServer, env: parseEnv(event.target.value) },
                });
              }}
            />,
          )}
          <div className="flex items-center gap-2 sm:col-span-2">
            <button
              type="button"
              disabled={saving}
              onClick={saveRow}
              className="rounded-control bg-foreground px-3 py-1.5 text-[12px] text-background disabled:opacity-40"
            >
              {t("settingsMcpSaveRow")}
            </button>
            <button
              type="button"
              disabled={saving}
              onClick={() => {
                setEditing(null);
                setRowError(null);
              }}
              className="rounded-control border border-border px-3 py-1.5 text-[12px] text-foreground disabled:opacity-40"
            >
              {t("settingsMcpCancel")}
            </button>
            {rowError === null ? null : (
              <span role="alert" className="text-[12px] text-destructive">
                {rowError}
              </span>
            )}
          </div>
        </div>
      )}
      <button
        type="button"
        disabled={saving}
        onClick={() => {
          setEditing({ index: null, server: emptyServer() });
          setRowError(null);
        }}
        className="rounded-control border border-border px-3 py-1.5 text-[12px] text-foreground disabled:opacity-40 sm:col-span-2"
      >
        {t("settingsMcpAdd")}
      </button>
    </PaneSection>
  );
}

export const components = { "settings.pane.mcp": McpPane } as const;
