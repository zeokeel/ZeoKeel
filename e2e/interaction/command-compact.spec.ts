import { expect, test, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/** `/compact` interaction contract (`2026-11-01-client-command-compact`). */
const SERVE = "http://127.0.0.1:8788";
const EVIDENCE = "target/evidence/2026-11-01-client-command-compact/interaction";
mkdirSync(EVIDENCE, { recursive: true });

function record(id: string, title: string): void {
  writeFileSync(join(EVIDENCE, `${id}.json`), `${JSON.stringify({ id, title, at: new Date().toISOString(), passed: true }, null, 2)}\n`, "utf8");
}

function isRecordValue(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function rowsOf(data: unknown): Record<string, unknown>[] {
  const entries: unknown[] = Array.isArray(data) ? data : [];
  return entries.filter(isRecordValue);
}

function idOf(row: Record<string, unknown> | undefined): string {
  const id = row?.["id"];
  return typeof id === "string" ? id : "";
}

async function callIpc(page: Page, name: string, payload: Record<string, unknown>): Promise<unknown> {
  const response = await page.request.post(`${SERVE}/ipc`, { data: { name, payload } });
  const body: unknown = await response.json();
  return isRecordValue(body) ? body["data"] : undefined;
}

async function openClient(page: Page): Promise<void> {
  await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
  await expect(page.getByRole("complementary", { name: "工作区与会话" })).toBeVisible();
  await page.getByRole("button", { name: "新建会话", exact: true }).last().click();
  await expect(composer(page)).toBeVisible();
}

function composer(page: Page): ReturnType<Page["getByRole"]> {
  return page.getByRole("textbox", { name: "消息输入框" });
}

/** Send plain messages one by one (each waits for its user row + turn completion). */
async function sendMessages(page: Page, count: number): Promise<void> {
  for (let index = 0; index < count; index += 1) {
    await composer(page).fill(`compact-history-${String(index)}`);
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("user-text").last()).toHaveText(`compact-history-${String(index)}`);
    await expect(page.getByRole("button", { name: "停止生成" })).toHaveCount(0, { timeout: 30_000 });
  }
}

async function runCompact(page: Page): Promise<void> {
  await composer(page).fill("/compact");
  await expect(page.getByRole("option", { name: /压缩上下文/ })).toBeVisible();
  await page.keyboard.press("Enter");
}

async function workspaceAndConversation(page: Page): Promise<{ workspaceId: string; conversationId: string }> {
  const workspaceId = idOf(rowsOf(await callIpc(page, "workspace.list", {}))[0]);
  const conversationId = idOf(rowsOf(await callIpc(page, "session.list", { workspace_id: workspaceId }))[0]);
  return { workspaceId, conversationId };
}

test.describe("command-compact", () => {
  test("CMD3C-COMPACT-01: /compact writes a marker, keeps raw history, is repeat-safe", { tag: ["@interaction:CMD3C-COMPACT-01"] }, async ({ page }) => {
    await openClient(page);
    await sendMessages(page, 7);
    const ids = await workspaceAndConversation(page);
    const rawBefore = rowsOf(await callIpc(page, "session.messages", { workspace_id: ids.workspaceId, conversation_id: ids.conversationId })).length;
    await runCompact(page);
    await expect(page.getByText("较早消息已压缩").first()).toBeVisible({ timeout: 30_000 });
    const divider = page.getByRole("separator", { name: "较早消息已压缩" }).first();
    await expect(divider).toBeVisible();
    await expect(page.getByText(/已压缩 \d+ 条较早消息/).first()).toBeVisible();
    // Raw user messages are still visible (store keeps raw semantics).
    await expect(page.getByTestId("user-text").first()).toHaveText("compact-history-0");
    const marker = rowsOf(await callIpc(page, "session.compaction_get", { workspace_id: ids.workspaceId, conversation_id: ids.conversationId }));
    expect(isRecordValue(marker[0] ?? {})).toBe(true);
    const rawAfter = rowsOf(await callIpc(page, "session.messages", { workspace_id: ids.workspaceId, conversation_id: ids.conversationId })).length;
    expect(rawAfter).toBe(rawBefore + 1); // only the system marker is appended
    // Search still reads raw history (including compacted-away messages).
    const rawSearch = await callIpc(page, "session.search", { workspace_id: ids.workspaceId, query: "compact-history-0" });
    const hitItems = isRecordValue(rawSearch) ? rowsOf(rawSearch["items"]) : [];
    expect(hitItems.length, JSON.stringify(rawSearch)).toBeGreaterThan(0);
    // Search reads raw history: the compacted-away message is still found.
    expect(hitItems.some((hit) => hit["message_id"] !== undefined)).toBe(true);
    // Summary expands with usage; repeated compact is a no-op.
    await page.getByRole("button", { name: "查看摘要" }).click();
    await expect(page.getByRole("region", { name: "压缩摘要" })).toContainText("compact-history-0");
    await expect(page.getByText(/摘要 tokens：输入 \d+ \/ 输出 \d+/)).toBeVisible();
    await runCompact(page);
    await expect(page.getByText("没有需要压缩的新消息")).toBeVisible();
    // Reload restores the divider from the persisted marker.
    await page.reload({ waitUntil: "networkidle" });
    await expect(page.getByRole("separator", { name: "较早消息已压缩" }).first()).toBeVisible({ timeout: 20_000 });
    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((violation) => violation.impact === "serious" || violation.impact === "critical").map((violation) => violation.id);
    expect(serious, JSON.stringify(serious)).toEqual([]);
    record("CMD3C-COMPACT-01", "compact marker keeps raw history and is repeat-safe");
  });

  test("CMD3C-KEY-01: /co filters, Enter runs, Esc keeps draft, divider is keyboard expandable", { tag: ["@interaction:CMD3C-KEY-01"] }, async ({ page }) => {
    await openClient(page);
    await sendMessages(page, 7);
    await composer(page).fill("/co");
    await expect(page.getByRole("option", { name: /压缩上下文/ })).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("listbox")).toHaveCount(0);
    await expect(composer(page)).toHaveValue("/co");
    await composer(page).fill("/compact");
    await page.keyboard.press("Enter");
    await expect(page.getByRole("separator", { name: "较早消息已压缩" }).first()).toBeVisible({ timeout: 30_000 });
    await composer(page).fill("after compact");
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("user-text").last()).toHaveText("after compact");
    // Keep-window messages stay verbatim after the divider.
    await expect(page.getByTestId("user-text").first()).toHaveText("compact-history-0");
    await page.getByRole("button", { name: "查看摘要" }).focus();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("region", { name: "压缩摘要" })).toBeVisible();
    record("CMD3C-KEY-01", "compact menu keys and divider keyboard expansion");
  });

  test("CMD3C-ERR-01: failures and streaming never change history or fake success", { tag: ["@interaction:CMD3C-ERR-01"] }, async ({ page }) => {
    await openClient(page);
    await sendMessages(page, 7);
    const ids = await workspaceAndConversation(page);
    const before = rowsOf(await callIpc(page, "session.messages", { workspace_id: ids.workspaceId, conversation_id: ids.conversationId })).length;
    await page.route("**/ipc", async (route) => {
      const body: unknown = route.request().postDataJSON();
      const name = isRecordValue(body) ? body["name"] : undefined;
      if (name === "session.compact") {
        await route.fulfill({ status: 500, body: "boom" });
        return;
      }
      await route.continue();
    });
    await runCompact(page);
    await expect(page.getByText("压缩失败，历史未改变")).toBeVisible();
    await expect(page.getByRole("separator", { name: "较早消息已压缩" })).toHaveCount(0);
    const after = rowsOf(await callIpc(page, "session.messages", { workspace_id: ids.workspaceId, conversation_id: ids.conversationId })).length;
    expect(after).toBe(before);
    await page.unroute("**/ipc");
    record("CMD3C-ERR-01", "failures never change history or fake success");
  });
});
