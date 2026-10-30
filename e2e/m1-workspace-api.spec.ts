import { test, expect } from "@playwright/test";
import { copyFileSync, mkdirSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const SERVE = "http://127.0.0.1:8788";

async function call(request: import("@playwright/test").APIRequestContext, name: string, payload: unknown) {
  const response = await request.post(`${SERVE}/ipc`, { data: { name, payload } });
  expect(response.ok()).toBeTruthy();
  const body = (await response.json()) as { ok: boolean; data?: unknown; error?: string };
  return body;
}

test.describe("workspace IPC (serve carrier)", () => {
  test("create workspace, scoped conversation, cross-workspace mismatch, reassign delete", async ({ request }) => {
    const list = await call(request, "workspace.list", {});
    expect(list.ok).toBeTruthy();
    const workspaces = list.data as Array<{ id: string; path: string; is_default: boolean }>;
    const defaultWs = workspaces.find((w) => w.is_default);
    expect(defaultWs).toBeTruthy();
    expect(defaultWs?.path).toContain("home");

    const dir = mkdtempSync(join(tmpdir(), "zeokeel-e2e-ws-"));
    mkdirSync(join(dir, "demo-data"));
    copyFileSync(join(process.cwd(), "demo-data/hello.txt"), join(dir, "demo-data/hello.txt"));
    const created = await call(request, "workspace.create", { path: dir, title: "e2e workspace" });
    expect(created.ok, created.error).toBeTruthy();
    const ws = created.data as { id: string; path: string; is_default: boolean };
    expect(ws.path).toBe(dir);

    const conv = await call(request, "session.create", { workspace_id: ws.id, title: "scoped" });
    expect(conv.ok, conv.error).toBeTruthy();
    const conversation = conv.data as { id: string; workspace_id: string };
    expect(conversation.workspace_id).toBe(ws.id);

    const missing = await call(request, "chat.send", { workspace_id: ws.id, text: "读取 demo-data/hello.txt" });
    expect(missing.ok).toBeFalsy();
    expect(missing.error).toContain("missing conversation_id");

    const sent = await call(request, "chat.send", { workspace_id: ws.id, conversation_id: conversation.id, text: "读取 demo-data/hello.txt" });
    expect(sent.ok, sent.error).toBeTruthy();
    await new Promise((resolve) => setTimeout(resolve, 1500));
    const messages = await call(request, "session.messages", { workspace_id: ws.id, conversation_id: conversation.id });
    expect(messages.ok).toBeTruthy();
    expect(JSON.stringify(messages.data)).toContain("Hello ZeoKeel");

    const mismatch = await call(request, "session.messages", { workspace_id: defaultWs?.id, conversation_id: conversation.id });
    expect(mismatch.ok).toBeFalsy();
    expect(mismatch.error).toContain("workspace-mismatch");

    const refused = await call(request, "workspace.remove", { workspace_id: ws.id, confirm: true });
    expect(refused.ok).toBeFalsy();
    const removed = await call(request, "workspace.remove", { workspace_id: ws.id, reassign_to: defaultWs?.id, confirm: true });
    expect(removed.ok, removed.error).toBeTruthy();
    expect((removed.data as { moved_sessions: number }).moved_sessions).toBe(1);
  });
});
