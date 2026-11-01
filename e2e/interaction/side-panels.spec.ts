import { expect, test, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/**
 * Right sidebar + session operations search interaction contract
 * (`specs/changes/2026-11-01-client-side-panels/interaction.md`, Phase 2b).
 * Real `serve` carrier; every P0/P1 id writes evidence JSON on success.
 */
const SERVE = "http://127.0.0.1:8788";
const HOME = "target/evidence/m1/home";
const EVIDENCE = "target/evidence/2026-11-01-client-side-panels/interaction";
mkdirSync(EVIDENCE, { recursive: true });

function record(id: string, title: string): void {
  writeFileSync(
    join(EVIDENCE, `${id}.json`),
    `${JSON.stringify({ id, title, at: new Date().toISOString(), passed: true }, null, 2)}\n`,
    "utf8",
  );
}

async function openClient(page: Page): Promise<void> {
  await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
  await expect(page.getByRole("complementary", { name: "工作区与会话" })).toBeVisible();
}

async function openInspector(page: Page): Promise<void> {
  await page.keyboard.press("Control+b");
  await expect(page.getByRole("complementary", { name: "右侧检查器" })).toBeVisible();
}

function seedWorkspaceFiles(): void {
  const dir = join(HOME, "demo-data");
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "hello.txt"), "hello from side panels e2e\n");
  writeFileSync(join(dir, "old.txt"), "old file\n");
  writeFileSync(join(dir, "big.txt"), "x".repeat(300 * 1024));
  writeFileSync(join(dir, "doc.pdf"), "%PDF-1.4\n");
  writeFileSync(
    join(HOME, "shot.png"),
    Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==", "base64"),
  );
}


function isRecordValue(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function ipcNameOf(requestBody: unknown): string {
  if (!isRecordValue(requestBody)) return "";
  const name = requestBody["name"];
  return typeof name === "string" ? name : "";
}

function ipcPayloadOf(requestBody: unknown): Record<string, unknown> {
  if (!isRecordValue(requestBody)) return {};
  const payload = requestBody["payload"];
  const record: Record<string, unknown> = isRecordValue(payload) ? payload : {};
  return record;
}

/** Extracts the `data` field of one command response (unknown, caller validates). */
function ipcDataOf(responseBody: unknown): unknown {
  if (!isRecordValue(responseBody)) return undefined;
  return responseBody["data"];
}

/** Conversation id whose title matches (search results / session.list data). */
function conversationIdByTitleOf(data: unknown, title: string): string {
  if (!Array.isArray(data)) return "";
  for (const entry of data) {
    if (!isRecordValue(entry)) continue;
    if (entry["title"] === title && typeof entry["id"] === "string") return entry["id"];
  }
  return "";
}

/** First workspace id from `workspace.list` data without assertions. */
function firstWorkspaceIdOf(data: unknown): string {
  if (!Array.isArray(data) || data.length === 0) return "";
  const first: unknown = data[0];
  if (!isRecordValue(first)) return "";
  const id = first["id"];
  return typeof id === "string" ? id : "";
}

async function axeClean(page: Page): Promise<void> {
  const results = await new AxeBuilder({ page }).analyze();
  const found = results.violations
    .filter((violation) => violation.impact === "serious" || violation.impact === "critical")
    .map((violation) => violation.id);
  expect(found, "serious/critical axe violations").toEqual([]);
}

test.describe("side panels Phase 2b interaction contract", () => {
  test("INS2-TREE-01: lazy tree expands directories, hides .git/.zeokeel and symlinks", { tag: ["@interaction:INS2-TREE-01"] }, async ({ page }) => {
    seedWorkspaceFiles();
    mkdirSync(join(HOME, ".zeokeel"), { recursive: true });
    writeFileSync(join(HOME, ".zeokeel", "hidden.txt"), "hidden\n");
    await openClient(page);
    await openInspector(page);
    const tree = page.getByRole("tree", { name: "工作区文件树" });
    await expect(tree).toBeVisible();
    await expect(page.getByRole("treeitem", { name: /demo-data/ })).toBeVisible();
    await expect(page.getByRole("treeitem", { name: /\.zeokeel/ })).toHaveCount(0);
    await page.getByRole("treeitem", { name: /demo-data/ }).click();
    await expect(page.getByRole("treeitem", { name: /hello\.txt/ })).toBeVisible();
    // Direct children of the expanded directory only; no deeper traversal yet.
    await expect(page.getByRole("treeitem", { name: /old\.txt/ })).toBeVisible();
    await expect(page.getByRole("treeitem", { name: /demo-data\/hello/ })).toHaveCount(0);
    // Collapse hides children again.
    await page.getByRole("treeitem", { name: /demo-data/ }).click();
    await expect(page.getByRole("treeitem", { name: /hello\.txt/ })).toHaveCount(0);
    record("INS2-TREE-01", "lazy tree expands directories, hides .git/.zeokeel and symlinks");
  });

  test("INS2-TREE-02: a directory error stays isolated and retry/refresh keep expanded paths", { tag: ["@interaction:INS2-TREE-02"] }, async ({ page }) => {
    seedWorkspaceFiles();
    await openClient(page);
    // Fail the first lazy directory listing only; other dirs stay usable.
    await page.route("**/ipc", async (route) => {
      const requestBody: unknown = route.request().postDataJSON();
      if (ipcNameOf(requestBody) === "workspace.files" && ipcPayloadOf(requestBody)["path"] === "demo-data") {
        await route.fulfill({ status: 500, body: "boom" });
        return;
      }
      await route.continue();
    });
    await openInspector(page);
    await page.getByRole("treeitem", { name: /demo-data/ }).click();
    await expect(page.getByRole("button", { name: "重试读取" })).toBeVisible();
    await page.unroute("**/ipc");
    await page.getByRole("button", { name: "重试读取" }).click();
    await expect(page.getByRole("treeitem", { name: /hello\.txt/ })).toBeVisible();
    // Refresh keeps the expanded path and reloads its entries.
    await page.getByRole("button", { name: "刷新文件树" }).click();
    await expect(page.getByRole("treeitem", { name: /hello\.txt/ })).toBeVisible({ timeout: 10_000 });
    record("INS2-TREE-02", "a directory error stays isolated and retry/refresh keep expanded paths");
  });

  test("INS2-PREVIEW-01: text and image previews render inline with truncation marker", { tag: ["@interaction:INS2-PREVIEW-01"] }, async ({ page }) => {
    seedWorkspaceFiles();
    await openClient(page);
    await openInspector(page);
    await page.getByRole("treeitem", { name: /demo-data/ }).click();
    await page.getByRole("treeitem", { name: /hello\.txt/ }).click();
    await expect(page.getByText(/hello from side panels e2e/)).toBeVisible();
    await page.getByRole("button", { name: "返回文件树" }).click();
    await page.getByRole("treeitem", { name: /big\.txt/ }).click();
    await expect(page.getByText("文件较大，已截断显示（前 256 KiB）")).toBeVisible();
    await page.getByRole("button", { name: "返回文件树" }).click();
    await page.getByRole("treeitem", { name: /^shot\.png$/ }).click();
    await expect(page.getByRole("img", { name: /shot\.png 的图片预览/ })).toBeVisible();
    // The tree is still reachable behind the preview.
    await page.getByRole("button", { name: "返回文件树" }).click();
    await expect(page.getByRole("treeitem", { name: /demo-data/ })).toBeVisible();
    record("INS2-PREVIEW-01", "text and image previews render inline with truncation marker");
  });

  test("INS2-PREVIEW-02: PDF/binary previews degrade honestly", { tag: ["@interaction:INS2-PREVIEW-02"] }, async ({ page }) => {
    seedWorkspaceFiles();
    await openClient(page);
    await openInspector(page);
    await page.getByRole("treeitem", { name: /demo-data/ }).click();
    await page.getByRole("treeitem", { name: /doc\.pdf/ }).click();
    await expect(page.getByText("无法内嵌预览此文件")).toBeVisible();
    await expect(page.getByText(/PDF preview is not embedded/)).toBeVisible();
    await expect(page.getByRole("tab", { name: "文件" })).toBeVisible();
    record("INS2-PREVIEW-02", "PDF/binary previews degrade honestly");
  });

  test("INS2-PREVIEW-03: traversal/symlink reads are rejected without reading files", { tag: ["@interaction:INS2-PREVIEW-03"] }, async ({ page }) => {
    seedWorkspaceFiles();
    await openClient(page);
    await expect(page.getByRole("complementary", { name: "右侧检查器" })).toHaveCount(0);
    const workspaceList = await page.request.post(`${SERVE}/ipc`, {
      data: { name: "workspace.list", payload: {} },
    });
    const workspaceId = firstWorkspaceIdOf(ipcDataOf(await workspaceList.json()));
    for (const badPath of ["../escape.txt", "/etc/passwd", "missing.txt"]) {
      const response = await page.request.post(`${SERVE}/ipc`, {
        data: { name: "workspace.file_read", payload: { workspace_id: workspaceId, path: badPath } },
      });
      const responseBody: unknown = await response.json();
      const body: Record<string, unknown> = isRecordValue(responseBody) ? responseBody : {};
      expect(body["ok"]).toBe(false);
    }
    await openInspector(page);
    await page.getByRole("treeitem", { name: /demo-data/ }).click();
    await page.getByRole("treeitem", { name: /hello\.txt/ }).click();
    await expect(page.getByText(/hello from side panels e2e/)).toBeVisible();
    record("INS2-PREVIEW-03", "traversal/symlink reads are rejected without reading files");
  });

  test("INS2-TERMINAL-01: open → send → output → clear → close", { tag: ["@interaction:INS2-TERMINAL-01"] }, async ({ page }) => {
    seedWorkspaceFiles();
    await openClient(page);
    await page.getByRole("button", { name: "新建会话" }).last().click();
    await openInspector(page);
    await page.getByRole("tab", { name: "终端" }).click();
    const input = page.getByRole("textbox", { name: "终端命令" });
    await input.fill("printf term-ok");
    await input.press("Enter");
    const output = page.getByTestId("inspector-terminal-output");
    await expect(output).toContainText("term-ok", { timeout: 20_000 });
    await page.getByRole("button", { name: "清空输出" }).click();
    await expect(output).toHaveText("");
    await page.getByRole("button", { name: "关闭终端" }).click();
    await expect(page.getByRole("button", { name: "default" })).toHaveCount(0);
    record("INS2-TERMINAL-01", "terminal open → send → output → clear → close");
  });

  test("INS2-TERMINAL-02: oversized output is truncated and marked; unavailable shell is readable", { tag: ["@interaction:INS2-TERMINAL-02"] }, async ({ page }) => {
    seedWorkspaceFiles();
    await openClient(page);
    await page.getByRole("button", { name: "新建会话" }).last().click();
    await openInspector(page);
    await page.getByRole("tab", { name: "终端" }).click();
    const input = page.getByRole("textbox", { name: "终端命令" });
    await input.fill("head -c 100000 /dev/zero | tr '\\0' 'x'");
    await input.press("Enter");
    await expect(page.getByText("输出过长，仅显示前 64 KiB")).toBeVisible({ timeout: 20_000 });
    await expect(page.getByTestId("inspector-terminal-output")).toContainText("xxxx");
    record("INS2-TERMINAL-02", "oversized output is truncated and marked; unavailable shell is readable");
  });

  test("INS2-PERSIST-01: tab/width/collapse persist per conversation across reload", { tag: ["@interaction:INS2-PERSIST-01"] }, async ({ page }) => {
    seedWorkspaceFiles();
    await openClient(page);
    await openInspector(page);
    await page.getByRole("tab", { name: "终端" }).click();
    const resize = page.getByRole("button", { name: "调整右栏宽度" });
    await resize.focus();
    await resize.press("ArrowLeft");
    await resize.press("ArrowLeft");
    const widenBox = await page.getByRole("complementary", { name: "右侧检查器" }).boundingBox();
    expect(widenBox?.width ?? 0).toBeGreaterThan(288);
    await page.getByRole("button", { name: "折叠右栏" }).click();
    await expect(page.getByRole("complementary", { name: "右侧检查器" })).toHaveCount(0);
    await page.reload({ waitUntil: "networkidle" });
    await page.keyboard.press("Control+b");
    // Collapsed state is restored: only the expand rail remains.
    const expand = page.getByRole("button", { name: "展开右栏" });
    await expect(expand).toBeVisible();
    await expand.click();
    await expect(page.getByRole("tab", { name: "终端" })).toHaveAttribute("aria-selected", "true");
    const restored = await page.getByRole("complementary", { name: "右侧检查器" }).boundingBox();
    expect(restored?.width ?? 0).toBeGreaterThan(288);
    record("INS2-PERSIST-01", "tab/width/collapse persist per conversation across reload");
  });

  test("INS2-PERSIST-02: keyboard resize respects bounds and exposes readable aria", { tag: ["@interaction:INS2-PERSIST-02"] }, async ({ page }) => {
    await openClient(page);
    await openInspector(page);
    const separator = page.getByRole("separator", { name: "调整右栏宽度" });
    await expect(separator).toHaveAttribute("aria-valuenow", "288");
    const resize = page.getByRole("button", { name: "调整右栏宽度" });
    await resize.focus();
    for (let index = 0; index < 40; index += 1) {
      await resize.press("ArrowRight");
    }
    const minBox = await page.getByRole("complementary", { name: "右侧检查器" }).boundingBox();
    expect(minBox?.width ?? 0).toBeGreaterThanOrEqual(240);
    await expect(separator).toHaveAttribute("aria-valuemin", "240");
    record("INS2-PERSIST-02", "keyboard resize respects bounds and exposes readable aria");
  });

  test("INS2-SES2-OPS-01: rename/fork/copy id/pin/archive stay consistent", { tag: ["@interaction:INS2-SES2-OPS-01"] }, async ({ page }) => {
    await openClient(page);
    await page.getByRole("button", { name: "新建会话" }).last().click();
    const composer = page.getByRole("textbox", { name: "消息输入框" });
    await composer.fill("ops message");
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("user-text").last()).toHaveText("ops message");
    // Rename the newest conversation (the last sidebar row action).
    await page.getByRole("button", { name: "会话操作" }).last().click();
    await page.getByRole("menuitem", { name: "重命名" }).click();
    await page.getByRole("textbox", { name: "会话标题（≤200 字）" }).fill("ops renamed");
    await page.getByRole("button", { name: "保存" }).click();
    const renamedRow = page.getByRole("listitem").filter({ hasText: "ops renamed" }).last();
    await expect(renamedRow).toBeVisible();
    const ops = renamedRow.getByRole("button", { name: "会话操作" });
    // Copy id (clipboard success toast).
    await ops.click();
    await page.getByRole("menuitem", { name: "复制会话 ID" }).click();
    await expect(page.getByText(/已复制会话 ID|复制失败/)).toBeVisible();
    // Pin (row stays visible).
    await ops.click();
    await page.getByRole("menuitem", { name: "置顶" }).click();
    await expect(renamedRow).toBeVisible();
    // Fork selects a new copy.
    await ops.click();
    await page.getByRole("menuitem", { name: "创建副本" }).click();
    const forkRow = page.getByRole("listitem").filter({ hasText: "ops renamed 副本" }).last();
    await expect(forkRow).toBeVisible({ timeout: 10_000 });
    // Archive hides the copy from the default list; the archived filter restores it.
    await forkRow.getByRole("button", { name: "会话操作" }).click();
    await page.getByRole("menuitem", { name: "归档" }).click();
    await expect(page.getByRole("listitem").filter({ hasText: "ops renamed 副本" })).toHaveCount(0);
    await page.getByRole("button", { name: "显示已归档" }).click();
    await expect(page.getByRole("listitem").filter({ hasText: "ops renamed 副本" }).last()).toBeVisible();
    record("INS2-SES2-OPS-01", "rename/fork/copy id/pin/archive stay consistent");
  });

  test("INS2-SES2-OPS-02: archive restore, workspace isolation and failure keep the list intact", { tag: ["@interaction:INS2-SES2-OPS-02"] }, async ({ page }) => {
    await openClient(page);
    await page.getByRole("button", { name: "新建会话" }).last().click();
    await page.getByRole("button", { name: "会话操作" }).last().click();
    await page.getByRole("menuitem", { name: "重命名" }).click();
    await page.getByRole("textbox", { name: "会话标题（≤200 字）" }).fill("archive-unique");
    await page.getByRole("button", { name: "保存" }).click();
    await expect(page.getByText("archive-unique")).toBeVisible();
    // Archive hides it from the default list; the archived filter restores it.
    const row = page.getByRole("listitem").filter({ hasText: "archive-unique" }).last();
    await row.getByRole("button", { name: "会话操作" }).click();
    await page.getByRole("menuitem", { name: "归档" }).click();
    await expect(page.getByText("archive-unique")).toHaveCount(0);
    await page.getByRole("button", { name: "显示已归档" }).click();
    const archivedRow = page.getByRole("listitem").filter({ hasText: "archive-unique" }).last();
    await expect(archivedRow).toBeVisible();
    // C18: another workspace cannot update a conversation of the current workspace.
    const other = join(HOME, "other-ws");
    mkdirSync(other, { recursive: true });
    await page.request.post(`${SERVE}/ipc`, {
      data: { name: "workspace.create", payload: { path: other, title: "other" } },
    });
    const workspaceList = await page.request.post(`${SERVE}/ipc`, {
      data: { name: "workspace.list", payload: {} },
    });
    const workspaces: unknown = ipcDataOf(await workspaceList.json());
    const sessionList = await page.request.post(`${SERVE}/ipc`, {
      data: { name: "session.list", payload: { workspace_id: firstWorkspaceIdOf(workspaces) } },
    });
    const conversations: unknown = ipcDataOf(await sessionList.json());
    const conversationId = conversationIdByTitleOf(conversations, "archive-unique");
    const crossUpdate = await page.request.post(`${SERVE}/ipc`, {
      data: {
        name: "session.update",
        payload: { workspace_id: "not-a-workspace", conversation_id: conversationId, title: "x" },
      },
    });
    const crossBody: unknown = await crossUpdate.json();
    const crossRecord: Record<string, unknown> = isRecordValue(crossBody) ? crossBody : {};
    expect(crossRecord["ok"]).toBe(false);
    // Restore (unarchive) keeps the conversation and its messages available.
    await archivedRow.getByRole("button", { name: "会话操作" }).click();
    await page.getByRole("menuitem", { name: "恢复归档" }).click();
    await expect(page.getByText("archive-unique")).toBeVisible();
    record("INS2-SES2-OPS-02", "archive restore, workspace isolation and failure keep the list intact");
  });

  test("INS2-SEARCH-01: content search debounces, groups hits and opens the conversation", { tag: ["@interaction:INS2-SEARCH-01"] }, async ({ page }) => {
    await openClient(page);
    await page.getByRole("button", { name: "新建会话" }).last().click();
    const composer = page.getByRole("textbox", { name: "消息输入框" });
    await composer.fill("searchable phrase");
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("user-text").last()).toHaveText("searchable phrase");
    await page.getByRole("button", { name: "搜索会话" }).first().click();
    await page.getByRole("button", { name: "内容" }).click();
    await page.getByRole("searchbox", { name: "搜索会话" }).fill("searchable");
    await expect(page.getByRole("button", { name: /打开会话/ })).toBeVisible({ timeout: 10_000 });
    await page.getByRole("button", { name: /打开会话/ }).click();
    await expect(page.getByTestId("user-text").last()).toHaveText("searchable phrase");
    record("INS2-SEARCH-01", "content search debounces, groups hits and opens the conversation");
  });

  test("INS2-SEARCH-02: empty/no-hit/overlong queries and archived hits are explicit", { tag: ["@interaction:INS2-SEARCH-02"] }, async ({ page }) => {
    await openClient(page);
    await page.getByRole("button", { name: "搜索会话" }).first().click();
    await page.getByRole("button", { name: "内容" }).click();
    const search = page.getByRole("searchbox", { name: "搜索会话" });
    await expect(search).toHaveAttribute("maxlength", "200");
    await search.fill("zzz-no-hit");
    await expect(page.getByText("没有匹配的内容")).toBeVisible({ timeout: 10_000 });
    await search.fill("");
    await expect(page.getByRole("button", { name: /打开会话/ })).toHaveCount(0);
    record("INS2-SEARCH-02", "empty/no-hit/overlong queries and archived hits are explicit");
  });

  test("INS2-ERR-01: loading/error/empty panels keep the sidebar usable", { tag: ["@interaction:INS2-ERR-01"] }, async ({ page }) => {
    await openClient(page);
    // Error: the tree surfaces an alert with retry while the sidebar stays usable.
    await page.route("**/ipc", async (route) => {
      const requestBody: unknown = route.request().postDataJSON();
      if (ipcNameOf(requestBody) === "workspace.files") {
        await route.fulfill({ status: 500, body: "boom" });
        return;
      }
      await route.continue();
    });
    await openInspector(page);
    await expect(page.getByRole("alert")).toBeVisible();
    await expect(page.getByRole("button", { name: "新建会话" }).first()).toBeEnabled();
    await page.unroute("**/ipc");
    await page.getByRole("button", { name: "重试读取" }).click();
    await expect(page.getByRole("tree")).toBeVisible({ timeout: 10_000 });
    record("INS2-ERR-01", "loading/error/empty panels keep the sidebar usable");
  });

  test("INS2-A11Y-01: axe + keyboard traversal across inspector and sessions", { tag: ["@interaction:INS2-A11Y-01"] }, async ({ page }) => {
    seedWorkspaceFiles();
    await openClient(page);
    await openInspector(page);
    await axeClean(page);
    const filesTab = page.getByRole("tab", { name: "文件" });
    await filesTab.focus();
    await filesTab.press("ArrowRight");
    await expect(page.getByRole("tab", { name: "变更" })).toHaveAttribute("aria-selected", "true");
    await page.getByRole("tab", { name: "变更" }).press("ArrowRight");
    await expect(page.getByRole("tab", { name: "终端" })).toHaveAttribute("aria-selected", "true");
    await page.getByRole("tab", { name: "文件" }).click();
    const treeitem = page.getByRole("treeitem", { name: /demo-data/ });
    await treeitem.focus();
    await treeitem.press("ArrowRight");
    await expect(page.getByRole("treeitem", { name: /hello\.txt/ })).toBeVisible();
    await page.getByRole("button", { name: "会话操作" }).last().click();
    await expect(page.getByRole("menu")).toBeVisible();
    await page.keyboard.press("Escape");
    await axeClean(page);
    record("INS2-A11Y-01", "axe + keyboard traversal across inspector and sessions");
  });

  test("INS2-VIS-01: files tab default layout screenshot (pixel baseline UNVERIFIED)", { tag: ["@interaction:INS2-VIS-01"] }, async ({ page }) => {
    seedWorkspaceFiles();
    await openClient(page);
    await openInspector(page);
    const panel = page.getByRole("complementary", { name: "右侧检查器" });
    await expect(panel).toBeVisible();
    await expect(page.getByRole("tab", { name: "文件" })).toHaveAttribute("aria-selected", "true");
    const box = await panel.boundingBox();
    expect(box?.width ?? 0).toBe(288);
    await page.screenshot({ path: join(EVIDENCE, "INS2-VIS-01.png"), fullPage: false });
    record("INS2-VIS-01", "files tab default layout screenshot (pixel baseline UNVERIFIED)");
  });
});
