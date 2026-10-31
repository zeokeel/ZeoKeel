import { test, expect, type Page } from "@playwright/test";

const UI_URL = process.env["ZEOKEEL_SCHEDULE_URL"];

function dataOf(value: unknown): unknown {
  if (typeof value !== "object" || value === null) return null;
  return Reflect.get(value, "data");
}

function records(value: unknown): Record<string, unknown>[] {
  if (!Array.isArray(value)) return [];
  return value.filter((item): item is Record<string, unknown> => typeof item === "object" && item !== null);
}

function stringField(record: Record<string, unknown>, key: string): string | null {
  const value = record[key];
  return typeof value === "string" ? value : null;
}

async function ipc(page: Page, name: string, payload: unknown): Promise<unknown> {
  const response = await page.request.post(`${UI_URL ?? ""}/ipc`, { data: { name, payload } });
  expect(response.ok()).toBeTruthy();
  const body: unknown = await response.json();
  expect(body).toMatchObject({ ok: true });
  return body;
}

test.describe("schedule executor auto-run (real serve + SQLite)", () => {
  test.skip(UI_URL === undefined, "run via bash e2e/serve-schedule.sh");
  test.setTimeout(120_000);

  test("a due tool-created schedule starts an agent turn and persists the assistant message", async ({ page }) => {
    await page.goto(`${UI_URL}/`, { waitUntil: "networkidle" });

    // The serve carrier sets the auth cookie when the SPA index loads; reuse it for IPC.
    const workspaces = records(dataOf(await ipc(page, "workspace.list", {})));
    const workspace = workspaces.find((item) => item["is_default"] === true) ?? workspaces[0];
    expect(workspace, "serve must expose a workspace").toBeTruthy();
    const workspaceId = stringField(workspace ?? {}, "id");
    if (workspaceId === null) throw new Error("workspace id missing");

    const listScheduled = async (): Promise<string | null> => {
      const conversations = records(dataOf(await ipc(page, "session.list", { workspace_id: workspaceId })));
      const scheduled = conversations.find((item) => stringField(item, "title") === "Scheduled task");
      return stringField(scheduled ?? {}, "id");
    };

    // The executor scans every 10s (env fallback); a due entry creates "Scheduled task".
    await expect.poll(async () => (await listScheduled()) !== null, {
      timeout: 60_000,
      intervals: [500, 1_000],
    }).toBe(true);
    const scheduledId = await listScheduled();
    if (scheduledId === null) throw new Error("scheduled conversation missing");

    await expect.poll(async () => {
      const messages = records(dataOf(await ipc(page, "session.messages", { workspace_id: workspaceId, conversation_id: scheduledId })));
      return messages
        .filter((item) => stringField(item, "role") === "assistant")
        .map((item) => stringField(item, "content") ?? "")
        .join("\n");
    }, { timeout: 60_000, intervals: [500, 1_000] }).toContain("scheduled auto-run ok");

    // Reload the SPA and assert the persisted auto-generated conversation/message are visible.
    await page.reload({ waitUntil: "networkidle" });
    await page.getByRole("button", { name: /Scheduled task/ }).click();
    await expect(page.getByText("scheduled auto-run ok")).toBeVisible({ timeout: 20_000 });
    await page.screenshot({ path: "target/evidence/round11/schedule-autorun.png" });
  });
});
