import { test, expect } from "@playwright/test";
import { copyFileSync, mkdirSync, appendFileSync, writeFileSync, unlinkSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { join } from "node:path";
import AxeBuilder from "@axe-core/playwright";

const SERVE = "http://127.0.0.1:8788";
const HOME = "target/evidence/m1/home";


/** Calls one IPC command through the serve carrier and returns its `data`. */
async function callIpc(
  page: import("@playwright/test").Page,
  name: string,
  payload: Record<string, unknown>,
): Promise<unknown> {
  const response = await page.request.post(`${SERVE}/ipc`, { data: { name, payload } });
  expect(response.ok()).toBeTruthy();
  const body: unknown = await response.json();
  if (typeof body !== "object" || body === null) {
    return undefined;
  }
  const record: Record<string, unknown> = { ...body };
  return record["data"];
}

/** Array rows of an IPC `data` payload (settings/workspace/conversation lists). */
function rowsOf(data: unknown): Array<Record<string, unknown>> {
  if (!Array.isArray(data)) {
    return [];
  }
  return data.flatMap((row) =>
    typeof row === "object" && row !== null ? [{ ...row }] : [],
  );
}

/** `workspace.files` returns `{ items: [...] }`; flatten to rows for assertions. */
function fileRowsOf(data: unknown): Array<Record<string, unknown>> {
  if (typeof data !== "object" || data === null) {
    return [];
  }
  const record: Record<string, unknown> = { ...data };
  return rowsOf(record["items"]);
}

test.describe("M1 serve carrier (real dispatch + SQLite)", () => {
  test("send a tool-backed message, then reload and keep the conversation", async ({ page }) => {
    mkdirSync(join(HOME, "demo-data"), { recursive: true });
    copyFileSync("demo-data/hello.txt", join(HOME, "demo-data/hello.txt"));
    await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
    await page.getByRole("button", { name: "新建会话" }).last().click();
    const composer = page.getByRole("textbox").first();
    await expect(composer).toBeVisible();
    await composer.fill("读取 demo-data/hello.txt");
    await page.keyboard.press("Enter");
    await expect(page.getByText("fs.read", { exact: false }).first()).toBeVisible({ timeout: 20_000 });
    await expect(page.getByText(/Hello (from )?ZeoKeel/, { exact: false }).first()).toBeVisible({ timeout: 20_000 });
    // Usage is persisted with the assistant message and rendered by the usage part.
    await expect(page.getByText("输入 10")).toBeVisible({ timeout: 20_000 });
    await expect(page.getByText("输出 5")).toBeVisible();
    await expect(page.getByText("合计 15")).toBeVisible();
    await page.screenshot({ path: "target/evidence/m1/serve-chat.png" });

    await page.reload({ waitUntil: "networkidle" });
    await expect(page.getByText("读取 demo-data/hello.txt", { exact: false }).first()).toBeVisible({ timeout: 20_000 });
    await expect(page.getByText(/Hello (from )?ZeoKeel/, { exact: false }).first()).toBeVisible({ timeout: 20_000 });
    // Task-39 fix: persisted usage survives reload (real SQLite).
    await expect(page.getByText("输入 10")).toBeVisible({ timeout: 20_000 });
    await expect(page.getByText("输出 5")).toBeVisible();
    await expect(page.getByText("合计 15")).toBeVisible();
    await page.screenshot({ path: "target/evidence/m1/serve-reload.png" });
  });

  test("inspector lists workspace files with Ctrl+B", async ({ page }) => {
    mkdirSync(join(HOME, "demo-data"), { recursive: true });
    copyFileSync("demo-data/hello.txt", join(HOME, "demo-data/hello.txt"));
    await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
    await page.keyboard.press("Control+b");
    await expect(page.getByRole("complementary", { name: "右侧检查器" })).toBeVisible();
    await expect(page.getByText("demo-data/hello.txt", { exact: true })).toBeVisible({ timeout: 10_000 });
    await page.screenshot({ path: "target/evidence/m1/serve-inspector.png" });
    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious, JSON.stringify(serious.map((v) => v.id))).toEqual([]);
    await page.keyboard.press("Control+b");
    await expect(page.getByRole("complementary", { name: "右侧检查器" })).toBeHidden();
  });

  test("inspector changes tab lists modified/added/deleted files and empty state", async ({ page }) => {
    const home = HOME;
    appendFileSync(join(home, "demo-data/hello.txt"), "\nmodified by e2e\n");
    writeFileSync(join(home, "demo-data/added.txt"), "added by e2e\n");
    unlinkSync(join(home, "demo-data/old.txt"));

    await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
    await page.keyboard.press("Control+b");
    await expect(page.getByRole("complementary", { name: "右侧检查器" })).toBeVisible();
    await page.getByRole("tab", { name: "变更" }).click();
    await expect(page.getByText("demo-data/added.txt +1 −0")).toBeVisible({ timeout: 20_000 });
    await expect(page.getByText("demo-data/hello.txt +1 −0")).toBeVisible();
    await expect(page.getByText("demo-data/old.txt +0 −1")).toBeVisible();
    await page.screenshot({ path: "target/evidence/m1/serve-inspector-changes.png" });

    // Clean repo → empty state (switch away and back to refetch).
    execFileSync("git", ["-C", home, "checkout", "--", "."]);
    execFileSync("git", ["-C", home, "clean", "-fd", "demo-data"]);
    await page.getByRole("tab", { name: "文件" }).click();
    await page.getByRole("tab", { name: "变更" }).click();
    await expect(page.getByText("工作区没有未提交的变更")).toBeVisible({ timeout: 20_000 });
    await page.screenshot({ path: "target/evidence/m1/serve-inspector-changes-empty.png" });
  });

  test("settings persist the schedule config across reload (serve + SQLite)", async ({ page }) => {
    await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
    await page.getByRole("button", { name: /设置|Settings/ }).first().click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    // Paginated settings: scheduler lives in the Agent pane (order 30).
    await dialog.getByRole("button", { name: /^Agent$/ }).click();
    await expect(page.getByText("调度/自动化")).toBeVisible();

    const schedule = page.getByLabel("启用定时调度");
    await expect(schedule).not.toBeChecked();
    await schedule.check();
    const scan = page.getByLabel("扫描间隔（秒，5-3600）");
    await scan.fill("30");
    await page.getByRole("button", { name: "保存" }).click();
    await expect(page.getByText("设置已保存")).toBeVisible({ timeout: 20_000 });
    await expect(page.getByRole("dialog")).toBeHidden({ timeout: 20_000 });

    // Reload the SPA (new boot) and reopen settings: the saved values must come back from disk.
    await page.reload({ waitUntil: "networkidle" });
    await page.getByRole("button", { name: /设置|Settings/ }).first().click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await page.getByRole("dialog").getByRole("button", { name: /^Agent$/ }).click();
    await expect(page.getByLabel("启用定时调度")).toBeChecked();
    await expect(page.getByLabel("扫描间隔（秒，5-3600）")).toHaveValue("30");
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toBeHidden();

    // Optional cross-check through IPC (host persisted settings), not just the UI.
    const response = await page.request.post(`${SERVE}/ipc`, {
      data: { name: "settings.get", payload: {} },
    });
    expect(response.ok()).toBeTruthy();
    const body: unknown = await response.json();
    // settings.get returns `{ settings, revision }` (revision fence, spec R4).
    expect(body).toMatchObject({
      ok: true,
      data: {
        settings: { agent: { schedule: { enabled: true, scan_seconds: 30 } } },
        revision: expect.any(Number),
      },
    });
    await page.screenshot({ path: "target/evidence/m1/serve-settings-schedule.png" });
  });

  test("composer / menu executes the shared command catalog (new conversation)", async ({ page }) => {
    await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
    const workspaceId = rowsOf(await callIpc(page, "workspace.list", {}))[0]?.["id"];
    expect(workspaceId).toBeTruthy();
    const before = rowsOf(await callIpc(page, "session.list", { workspace_id: workspaceId }));
    await page.getByRole("button", { name: "新建会话" }).last().click();
    const composer = page.getByRole("textbox", { name: "消息输入框" });
    await composer.click();
    await composer.fill("/新");
    await expect(page.getByRole("listbox", { name: "命令" })).toBeVisible();
    await page.keyboard.press("Enter");
    await expect(composer).toHaveValue("");
    await expect
      .poll(async () => rowsOf(await callIpc(page, "session.list", { workspace_id: workspaceId })).length)
      .toBeGreaterThan(before.length);
  });

  test("composer @ references a workspace file, sends and survives reload", async ({ page }) => {
    mkdirSync(join(HOME, "demo-data"), { recursive: true });
    copyFileSync("demo-data/hello.txt", join(HOME, "demo-data/hello.txt"));
    await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
    await page.getByRole("button", { name: "新建会话" }).last().click();
    const composer = page.getByRole("textbox", { name: "消息输入框" });
    await composer.click();
    await composer.fill("@hello");
    await expect(page.getByRole("listbox", { name: "引用 workspace 文件" })).toBeVisible();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("list", { name: "引用与附件" })).toContainText("hello.txt");
    await composer.fill("看下这个文件");
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("user-text").last()).toHaveText("看下这个文件");
    await expect(page.getByTestId("user-refs").last()).toContainText("demo-data/hello.txt");
    await expect(page.getByText(/Hello (from )?ZeoKeel/, { exact: false }).first()).toBeVisible({ timeout: 20_000 });
    await page.reload({ waitUntil: "networkidle" });
    await expect(page.getByTestId("user-refs").last()).toContainText("demo-data/hello.txt", { timeout: 20_000 });
    await page.screenshot({ path: "target/evidence/2026-11-01-client-composer-surface/screenshots/serve-composer-at.png" });
    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious, JSON.stringify(serious.map((v) => v.id))).toEqual([]);
  });

  test("composer attachment is saved under .zeokeel/attachments and survives reload", async ({ page }) => {
    await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
    await page.getByRole("button", { name: "新建会话" }).last().click();
    const workspaceId = rowsOf(await callIpc(page, "workspace.list", {}))[0]?.["id"];
    // 1x1 transparent PNG header + payload within the 1 MiB / allow-list limits.
    const png = Buffer.from(
      "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
      "base64",
    );
    await page.getByTestId("composer-attach-input").setInputFiles({
      name: "shot.png",
      mimeType: "image/png",
      buffer: png,
    });
    await expect(page.getByText("shot.png", { exact: true })).toBeVisible({ timeout: 20_000 });
    // Thumbnail must be a FileReader `data:` URL (never `blob:`, CSP unchanged).
    const thumbnail = await page.getByRole("img", { name: /附件缩略图/ }).first().getAttribute("src");
    expect(thumbnail?.startsWith("data:image/png")).toBe(true);
    // Host persisted the file inside the workspace (new directory is the <conversation> level).
    await expect
      .poll(async () => {
        const files = fileRowsOf(await callIpc(page, "workspace.files", { workspace_id: workspaceId }));
        return files.map((entry) => String(entry["path"] ?? "")).join("\n");
      })
      .toContain(".zeokeel/attachments");
    const composer = page.getByRole("textbox", { name: "消息输入框" });
    await composer.fill("附件消息");
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("user-refs").last()).toContainText("shot.png");
    await page.reload({ waitUntil: "networkidle" });
    await expect(page.getByTestId("user-refs").last()).toContainText("shot.png", { timeout: 20_000 });
    await page.screenshot({ path: "target/evidence/2026-11-01-client-composer-surface/screenshots/serve-composer-attachment.png" });
    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious, JSON.stringify(serious.map((v) => v.id))).toEqual([]);
  });
});
