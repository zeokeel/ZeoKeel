import { test, expect } from "@playwright/test";
import { copyFileSync, existsSync, mkdirSync, appendFileSync, readdirSync, writeFileSync, unlinkSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { join } from "node:path";
import AxeBuilder from "@axe-core/playwright";

const SERVE = "http://127.0.0.1:8788";
const HOME = "target/evidence/m1/home";


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
    // Lazy tree: expand the root directory, then see its direct children.
    await page.getByRole("treeitem", { name: /demo-data/ }).click();
    await expect(page.getByRole("treeitem", { name: /hello\.txt/ })).toBeVisible({ timeout: 10_000 });
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

  test("composer / opens the DSH command surface and runs /model", async ({ page }) => {
    await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
    await page.getByRole("button", { name: "新建会话" }).last().click();
    const composer = page.getByRole("textbox", { name: "消息输入框" });
    await composer.click();
    await composer.fill("/mod");
    await expect(page.getByRole("listbox", { name: "命令" })).toBeVisible();
    await expect(page.getByRole("option", { name: /选择模型/ })).toBeVisible();
    await page.keyboard.press("Enter");
    await expect(composer).toHaveValue("");
    await expect(page.getByRole("dialog", { name: "会话级设置" })).toBeVisible();
    await page.keyboard.press("Escape");
    // `/skill` is real now: it explains the skill group and never sends a prompt.
    await composer.fill("/skill");
    await expect(page.locator("#zk-composer-command-skill")).toBeVisible();
    await page.keyboard.press("Enter");
    await expect(page.getByText("输入 / 查看可用技能")).toBeVisible();
    await expect(page.getByTestId("user-text")).toHaveCount(0);
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
    // Host persisted the file inside the workspace (metadata dirs stay hidden from the tree).
    await expect
      .poll(() => {
        const dir = join(HOME, ".zeokeel/attachments");
        if (!existsSync(dir)) {
          return "";
        }
        return readdirSync(dir, { withFileTypes: true })
          .flatMap((entry) => (entry.isDirectory() ? readdirSync(join(dir, entry.name)) : []))
          .join("\n");
      })
      .toContain("shot.png");
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

  test("channels: inspector tree lazy-loads, previews text and restores the session tab", async ({ page }) => {
    mkdirSync(join(HOME, "demo-data"), { recursive: true });
    copyFileSync("demo-data/hello.txt", join(HOME, "demo-data/hello.txt"));
    await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
    await page.keyboard.press("Control+b");
    await expect(page.getByRole("complementary", { name: "右侧检查器" })).toBeVisible();
    // Lazy tree: expanding a directory fetches only its direct children.
    await page.getByRole("treeitem", { name: /demo-data/ }).click();
    await expect(page.getByRole("treeitem", { name: /hello\.txt/ })).toBeVisible({ timeout: 10_000 });
    await page.getByRole("treeitem", { name: /hello\.txt/ }).click();
    await expect(page.getByText(/hello from zeokeel/i)).toBeVisible({ timeout: 10_000 });
    // Session-scoped layout: the active tab is remembered per conversation.
    await page.getByRole("tab", { name: "变更" }).click();
    await page.reload({ waitUntil: "networkidle" });
    await page.keyboard.press("Control+b");
    await expect(page.getByRole("tab", { name: "变更" })).toHaveAttribute("aria-selected", "true");
    await page.screenshot({ path: "target/evidence/m1/serve-inspector-phase2b.png" });
    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious, JSON.stringify(serious.map((v) => v.id))).toEqual([]);
  });

  test("channels: session menu rename/pin/archive and content search stay in workspace", async ({ page }) => {
    await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
    await page.getByRole("button", { name: "新建会话" }).last().click();
    const composer = page.getByRole("textbox", { name: "消息输入框" });
    await composer.fill("needle in the haystack");
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("user-text").last()).toHaveText("needle in the haystack");
    // ⋯ menu: rename, pin, archive toggle (C18: same workspace).
    await page.getByRole("button", { name: "会话操作" }).last().click();
    await expect(page.getByRole("menuitem", { name: "重命名" })).toBeVisible();
    await page.getByRole("menuitem", { name: "重命名" }).click();
    const rename = page.getByRole("textbox", { name: "会话标题（≤200 字）" });
    await rename.fill("Renamed conversation");
    await page.getByRole("button", { name: "保存" }).click();
    const sessionRow = page.getByRole("listitem").filter({ hasText: "Renamed conversation" }).last();
    await expect(sessionRow).toBeVisible();
    await sessionRow.getByRole("button", { name: "会话操作" }).click();
    await page.getByRole("menuitem", { name: "置顶" }).click();
    await sessionRow.getByRole("button", { name: "会话操作" }).click();
    await page.getByRole("menuitem", { name: "归档" }).click();
    await expect(page.getByRole("listitem").filter({ hasText: "Renamed conversation" })).toHaveCount(0);
    await page.getByRole("button", { name: "显示已归档" }).click();
    await expect(page.getByRole("listitem").filter({ hasText: "Renamed conversation" }).last()).toBeVisible();
    // Content search (current workspace only) finds the message and opens it.
    await page.getByRole("button", { name: "搜索会话" }).first().click();
    await page.getByRole("button", { name: "内容" }).click();
    await page.getByRole("searchbox", { name: "搜索会话" }).fill("needle");
    await expect(page.getByRole("button", { name: /打开会话/ })).toBeVisible({ timeout: 10_000 });
    await page.getByRole("button", { name: /打开会话/ }).click();
    await expect(page.getByTestId("user-text").last()).toHaveText("needle in the haystack");
    await page.screenshot({ path: "target/evidence/m1/serve-sessions-phase2b.png" });
  });

  test("channels: terminal executes a user command and shows output", async ({ page }) => {
    await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
    await page.getByRole("button", { name: "新建会话" }).last().click();
    await page.keyboard.press("Control+b");
    await page.getByRole("tab", { name: "终端" }).click();
    const input = page.getByRole("textbox", { name: "终端命令" });
    await input.fill("printf term-ok");
    await input.press("Enter");
    await expect(page.getByTestId("inspector-terminal-output")).toContainText("term-ok", { timeout: 20_000 });
    await page.screenshot({ path: "target/evidence/m1/serve-terminal-phase2b.png" });
  });
});

test("serve carrier /compact keeps raw messages and shows the compaction divider", async ({ page }) => {
  await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
  await expect(page.getByRole("complementary", { name: "工作区与会话" })).toBeVisible();
  await page.getByRole("button", { name: "新建会话", exact: true }).last().click();
  const composer = page.getByRole("textbox", { name: "消息输入框" });
  for (let index = 0; index < 7; index += 1) {
    await composer.fill(`compact-e2e-${String(index)}`);
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("user-text").last()).toHaveText(`compact-e2e-${String(index)}`);
    await expect(page.getByRole("button", { name: "停止生成" })).toHaveCount(0, { timeout: 30_000 });
  }
  await composer.fill("/compact");
  await page.keyboard.press("Enter");
  await expect(page.getByText("较早消息已压缩").first()).toBeVisible({ timeout: 30_000 });
  await expect(page.getByRole("separator", { name: "较早消息已压缩" }).first()).toBeVisible();
  // Raw history stays visible (store keeps raw; only a system marker is appended).
  await expect(page.getByTestId("user-text").first()).toHaveText("compact-e2e-0");
});
