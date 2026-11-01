import { expect, test, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/**
 * Activity right-panel interaction contract
 * (`specs/changes/2026-11-01-client-activity-panels/interaction.md`, ACT2-*).
 *
 * Real `serve` carrier (real SQLite + deterministic fake upstream + baseline git
 * workspace). Every P0/P1 id writes its evidence JSON on success; the file
 * existing means the id passed in this run (gate:
 * `python3 scripts/ui-interaction-gate.py --strict`).
 *
 * Journeys use role/accessible-name locators and real keyboard input; the only
 * structural hooks are the documented `data-testid`s.
 */
const SERVE = "http://127.0.0.1:8788";
const HOME = "target/evidence/m1/home";
const EVIDENCE_ROOT = "target/evidence/2026-11-01-client-activity-panels";
const EVIDENCE = `${EVIDENCE_ROOT}/interaction`;
const SCREENSHOTS = `${EVIDENCE_ROOT}/screenshots`;
mkdirSync(EVIDENCE, { recursive: true });
mkdirSync(SCREENSHOTS, { recursive: true });

function record(id: string, title: string): void {
  writeFileSync(
    join(EVIDENCE, `${id}.json`),
    `${JSON.stringify({ id, title, at: new Date().toISOString(), passed: true }, null, 2)}\n`,
    "utf8",
  );
}

function schedulesPath(): string {
  return join(HOME, ".zeokeel", "schedules.json");
}

function seedSchedule(): void {
  mkdirSync(join(HOME, ".zeokeel"), { recursive: true });
  writeFileSync(
    schedulesPath(),
    `${JSON.stringify(
      [
        {
          id: "schedule-1",
          origin: "tool-schedule",
          cron: "0 9 * * *",
          prompt: "nightly",
          enabled: true,
          interval_seconds: 3600,
          next_run_ms: Date.now() + 3_600_000,
        },
      ],
      null,
      2,
    )}\n`,
    "utf8",
  );
}

function clearSchedules(): void {
  if (existsSync(schedulesPath())) rmSync(schedulesPath());
}

function isRecordValue(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function stringOf(record: Record<string, unknown>, key: string): string {
  const value = record[key];
  return typeof value === "string" ? value : "";
}

function numberArrayOf(value: unknown): Record<string, unknown>[] {
  return Array.isArray(value) ? value.filter(isRecordValue) : [];
}

// ---------------------------------------------------------------------------
// serve IPC helpers (page.request keeps the app/transport irrelevant here)
// ---------------------------------------------------------------------------

async function callIpc(page: Page, name: string, payload: Record<string, unknown>): Promise<unknown> {
  const response = await page.request.post(`${SERVE}/ipc`, { data: { name, payload } });
  const body: unknown = await response.json();
  return isRecordValue(body) ? body["data"] : undefined;
}

async function callIpcResponse(
  page: Page,
  name: string,
  payload: Record<string, unknown>,
): Promise<{ ok: boolean; data: unknown; error: string }> {
  const response = await page.request.post(`${SERVE}/ipc`, { data: { name, payload } });
  const body: unknown = await response.json();
  if (!isRecordValue(body)) return { ok: false, data: undefined, error: "" };
  return { ok: body["ok"] === true, data: body["data"], error: stringOf(body, "error") };
}

async function activeIds(page: Page): Promise<{ workspaceId: string; conversationId: string }> {
  const workspaces = numberArrayOf(await callIpc(page, "workspace.list", {}));
  const workspaceId = stringOf(workspaces[0] ?? {}, "id");
  const conversations = numberArrayOf(
    await callIpc(page, "session.list", { workspace_id: workspaceId }),
  );
  const conversationId = stringOf(conversations[0] ?? {}, "id");
  return { workspaceId, conversationId };
}

async function snapshotOf(page: Page): Promise<Record<string, unknown>> {
  const ids = await activeIds(page);
  const data = await callIpc(page, "activity.snapshot", {
    workspace_id: ids.workspaceId,
    conversation_id: ids.conversationId,
  });
  return isRecordValue(data) ? data : {};
}

// ---------------------------------------------------------------------------
// UI journeys
// ---------------------------------------------------------------------------

async function conversationRowCount(page: Page): Promise<number> {
  return page.getByRole("button", { name: "会话操作" }).count();
}

async function newConversation(page: Page): Promise<void> {
  const before = await conversationRowCount(page);
  await page.getByRole("button", { name: "新建会话", exact: true }).last().click();
  // The sidebar row appears in the same render that activates the new
  // conversation, so this also settles the session-scoped inspector layout
  // before any tab is opened (avoids a layout-restore race with tab clicks).
  await expect(page.getByRole("button", { name: "会话操作" })).toHaveCount(before + 1);
  await expect(composer(page)).toBeVisible();
}

async function openClient(page: Page): Promise<void> {
  await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
  await expect(page.getByRole("complementary", { name: "工作区与会话" })).toBeVisible();
  await newConversation(page);
}

async function startNewConversation(page: Page): Promise<void> {
  await newConversation(page);
}

function composer(page: Page) {
  return page.getByRole("textbox", { name: "消息输入框" });
}

async function openInspector(page: Page): Promise<void> {
  await page.keyboard.press("Control+b");
  await expect(page.getByRole("complementary", { name: "右侧检查器" })).toBeVisible();
}

async function openActivityTab(page: Page): Promise<void> {
  await page.getByRole("tab", { name: "活动" }).click();
  await expect(page.getByTestId("activity-panel")).toBeVisible();
}

async function sendMessage(page: Page, text: string): Promise<void> {
  await composer(page).fill(text);
  await page.keyboard.press("Enter");
}

async function waitTurnDone(page: Page): Promise<void> {
  await expect(page.getByRole("button", { name: "停止生成" })).toHaveCount(0, { timeout: 30_000 });
}

async function sendAndApproveBackgroundJob(page: Page): Promise<void> {
  await sendMessage(page, "后台任务：起一个后台任务");
  const allow = page.getByRole("button", { name: "允许一次" });
  await expect(allow).toBeVisible({ timeout: 30_000 });
  await allow.click();
  await expect(page.getByText("Hello from ZeoKeel", { exact: true }).first()).toBeVisible({
    timeout: 30_000,
  });
  await waitTurnDone(page);
}

async function jobRowStatus(page: Page): Promise<string> {
  const snapshot = await snapshotOf(page);
  const jobs = numberArrayOf(snapshot["jobs"]);
  return stringOf(jobs[0] ?? {}, "status");
}

async function axeClean(page: Page, label: string): Promise<void> {
  const results = await new AxeBuilder({ page }).analyze();
  const found = results.violations
    .filter((violation) => violation.impact === "serious" || violation.impact === "critical")
    .map((violation) => violation.id);
  writeFileSync(
    join(EVIDENCE_ROOT, "axe.json"),
    `${JSON.stringify(
      {
        label,
        at: new Date().toISOString(),
        seriousOrCritical: found,
        all: results.violations.map((violation) => ({
          id: violation.id,
          impact: violation.impact ?? null,
          nodes: violation.nodes.length,
        })),
      },
      null,
      2,
    )}\n`,
    "utf8",
  );
  expect(found, `${label}: serious/critical axe violations`).toEqual([]);
}

test.describe("activity panels interaction contract", () => {
  test("ACT2-SNAPSHOT-01: tab order 40 renders every section; goal is read via goal.get (not the snapshot)", { tag: ["@interaction:ACT2-SNAPSHOT-01"] }, async ({ page }) => {
    clearSchedules();
    await openClient(page);
    const ids = await activeIds(page);
    // Goal is set through the existing `goal.set` (single write owner) and the
    // panel must read it through `goal.get` — never the activity snapshot.
    const goalSet = await callIpcResponse(page, "goal.set", {
      workspace_id: ids.workspaceId,
      conversation_id: ids.conversationId,
      objective: "完成活动面板验收",
      max_rounds: 5,
    });
    expect(goalSet.ok, JSON.stringify(goalSet)).toBe(true);
    await sendMessage(page, "待办与交付");
    await expect(page.getByText("Hello from ZeoKeel", { exact: true }).first()).toBeVisible({
      timeout: 30_000,
    });
    await waitTurnDone(page);
    await openInspector(page);
    // Fixed tab order (docs/16 §2.2): files → changes → terminal → activity → plan.
    const tabs = page.getByRole("tab");
    await expect(tabs).toHaveCount(5);
    expect(await tabs.allTextContents()).toEqual(["文件", "变更", "终端", "活动", "计划"]);
    await openActivityTab(page);
    const panel = page.getByTestId("activity-panel");
    // Sections with data from persisted tool rows + the separate goal read.
    await expect(panel.getByText("整理需求")).toBeVisible({ timeout: 20_000 });
    await expect(panel.getByText("补充测试")).toBeVisible();
    await expect(panel.getByRole("listitem").filter({ hasText: "demo-data/hello.txt" })).toBeVisible();
    await expect(panel.getByText("完成活动面板验收")).toBeVisible({ timeout: 20_000 });
    // Empty sections show their own empty states (no white panel).
    await expect(panel.getByText("暂无后台任务")).toBeVisible();
    await expect(panel.getByText("暂无子代理")).toBeVisible();
    await expect(panel.getByText("暂无团队运行记录")).toBeVisible();
    await expect(panel.getByText("暂无定时任务")).toBeVisible();
    // Wire contract: the snapshot has no goal field; goal.get carries it.
    const snapshot = await snapshotOf(page);
    expect(Object.keys(snapshot)).not.toContain("goal");
    const goalData = await callIpc(page, "goal.get", {
      workspace_id: ids.workspaceId,
      conversation_id: ids.conversationId,
    });
    const goalView = isRecordValue(goalData) ? goalData["goal"] : undefined;
    expect(stringOf(isRecordValue(goalView) ? goalView : {}, "objective")).toBe("完成活动面板验收");
    record("ACT2-SNAPSHOT-01", "activity tab order 40 renders all sections; goal read through goal.get");
  });

  test("ACT2-POLL-01: 2s polling, manual refresh, and stop-on-tab-switch", { tag: ["@interaction:ACT2-POLL-01"] }, async ({ page }) => {
    let snapshotRequests = 0;
    await page.route("**/ipc", async (route) => {
      const body: unknown = route.request().postDataJSON();
      const name = isRecordValue(body) ? stringOf(body, "name") : "";
      if (name === "activity.snapshot") snapshotRequests += 1;
      await route.continue();
    });
    await openClient(page);
    await openInspector(page);
    await openActivityTab(page);
    await expect(page.getByText("暂无后台任务")).toBeVisible();
    const afterOpen = snapshotRequests;
    expect(afterOpen).toBeGreaterThanOrEqual(1);
    await expect
      .poll(() => snapshotRequests, { timeout: 10_000 })
      .toBeGreaterThan(afterOpen);
    const afterPoll = snapshotRequests;
    await page.getByRole("button", { name: "刷新活动" }).click();
    await expect.poll(() => snapshotRequests, { timeout: 10_000 }).toBeGreaterThan(afterPoll);
    // Switching the tab away stops polling: no `activity.snapshot` request may
    // arrive within the next two poll intervals (deterministic wait, no sleep).
    await page.getByRole("tab", { name: "文件" }).click();
    const afterSwitch = snapshotRequests;
    const strayPoll = await page
      .waitForRequest(
        (request) => {
          const body: unknown = request.postDataJSON();
          return request.url().endsWith("/ipc") && isRecordValue(body) && stringOf(body, "name") === "activity.snapshot";
        },
        { timeout: 3_000 },
      )
      .then(() => true)
      .catch(() => false);
    expect(strayPoll).toBe(false);
    expect(snapshotRequests).toBe(afterSwitch);
    record("ACT2-POLL-01", "activity polling starts on open, manually refreshes, and stops on tab switch");
  });

  test("ACT2-JOB-01: expand output; inline confirm (cancel/Esc do nothing) then stop the job", { tag: ["@interaction:ACT2-JOB-01"] }, async ({ page }) => {
    await openClient(page);
    await sendAndApproveBackgroundJob(page);
    await openInspector(page);
    await openActivityTab(page);
    const panel = page.getByTestId("activity-panel");
    await expect(panel.getByText("运行中").first()).toBeVisible({ timeout: 20_000 });
    await expect.poll(() => jobRowStatus(page), { timeout: 20_000 }).toBe("running");
    // Expand the (still empty) output.
    await panel.getByRole("button", { name: "展开输出" }).first().click();
    await expect(panel.getByText("（暂无输出）")).toBeVisible();
    const ids = await activeIds(page);
    const rowsBefore = numberArrayOf(
      await callIpc(page, "session.messages", {
        workspace_id: ids.workspaceId,
        conversation_id: ids.conversationId,
      }),
    ).length;

    // 1) Cancel the confirmation: no action, job keeps running.
    await panel.getByRole("button", { name: "停止任务" }).first().click();
    const jobDialog = page.getByRole("alertdialog", { name: "确认停止任务" });
    await expect(jobDialog).toBeVisible();
    await expect(jobDialog.getByRole("button", { name: "取消" })).toBeFocused();
    await jobDialog.getByRole("button", { name: "取消" }).click();
    await expect(jobDialog).toHaveCount(0);
    await expect.poll(() => jobRowStatus(page)).toBe("running");

    // 2) Esc cancels as well.
    await panel.getByRole("button", { name: "停止任务" }).first().click();
    await expect(page.getByRole("alertdialog", { name: "确认停止任务" })).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("alertdialog", { name: "确认停止任务" })).toHaveCount(0);
    await expect.poll(() => jobRowStatus(page)).toBe("running");

    // 3) Confirm: the job stops and the list refreshes; no tool row/approval.
    await panel.getByRole("button", { name: "停止任务" }).first().click();
    await page.getByRole("alertdialog", { name: "确认停止任务" }).getByRole("button", { name: "确认" }).click();
    await expect
      .poll(() => jobRowStatus(page), { timeout: 20_000 })
      .not.toBe("running");
    await expect(page.getByRole("alertdialog", { name: "确认停止任务" })).toHaveCount(0);
    await expect(panel.getByRole("button", { name: "停止任务" })).toHaveCount(0, { timeout: 20_000 });
    const rowsAfter = numberArrayOf(
      await callIpc(page, "session.messages", {
        workspace_id: ids.workspaceId,
        conversation_id: ids.conversationId,
      }),
    ).length;
    expect(rowsAfter).toBe(rowsBefore);

    // Unknown/ﬁnished ids are readable errors (single `activity.action` entry).
    const unknown = await callIpcResponse(page, "activity.action", {
      workspace_id: ids.workspaceId,
      conversation_id: ids.conversationId,
      action: "job_kill",
      id: "job-does-not-exist",
    });
    expect(unknown.ok).toBe(false);
    expect(unknown.error).toContain("activity/job-not-found");
    record("ACT2-JOB-01", "job stop requires inline confirm; cancel/Esc keep the job; confirm stops it");
  });

  test("ACT2-SUBAGENT-01: roster by parent; inline confirm then interrupt", { tag: ["@interaction:ACT2-SUBAGENT-01"] }, async ({ page }) => {
    await openClient(page);
    await sendMessage(page, "子代理：检查工作区");
    await expect(page.getByText("Hello from ZeoKeel", { exact: true }).first()).toBeVisible({
      timeout: 30_000,
    });
    await waitTurnDone(page);
    await openInspector(page);
    await openActivityTab(page);
    const panel = page.getByTestId("activity-panel");
    await expect(panel.getByText(/subagent-\d+/).first()).toBeVisible({ timeout: 20_000 });
    await expect(panel.getByRole("button", { name: "中断子代理" }).first()).toBeVisible();
    const ids = await activeIds(page);
    const rowsBefore = numberArrayOf(
      await callIpc(page, "session.messages", {
        workspace_id: ids.workspaceId,
        conversation_id: ids.conversationId,
      }),
    ).length;
    // Cancel first: nothing happens.
    await panel.getByRole("button", { name: "中断子代理" }).first().click();
    const dialog = page.getByRole("alertdialog", { name: "确认中断子代理" });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole("button", { name: "取消" })).toBeFocused();
    await dialog.getByRole("button", { name: "取消" }).click();
    await expect(page.getByRole("alertdialog", { name: "确认中断子代理" })).toHaveCount(0);
    let snapshot = await snapshotOf(page);
    expect(stringOf(numberArrayOf(snapshot["subagents"])[0] ?? {}, "status")).toBe("running");
    // Confirm the interrupt; the roster refreshes to a terminal state.
    await panel.getByRole("button", { name: "中断子代理" }).first().click();
    await page.getByRole("alertdialog", { name: "确认中断子代理" }).getByRole("button", { name: "确认" }).click();
    await expect
      .poll(async () => {
        const next = await snapshotOf(page);
        return stringOf(numberArrayOf(next["subagents"])[0] ?? {}, "status");
      }, { timeout: 20_000 })
      .not.toBe("running");
    await expect(panel.getByRole("button", { name: "中断子代理" })).toHaveCount(0, { timeout: 20_000 });
    const rowsAfter = numberArrayOf(
      await callIpc(page, "session.messages", {
        workspace_id: ids.workspaceId,
        conversation_id: ids.conversationId,
      }),
    ).length;
    expect(rowsAfter).toBe(rowsBefore);
    // Unknown id is a readable error.
    const unknown = await callIpcResponse(page, "activity.action", {
      workspace_id: ids.workspaceId,
      conversation_id: ids.conversationId,
      action: "subagent_interrupt",
      id: "subagent-missing",
    });
    expect(unknown.ok).toBe(false);
    expect(unknown.error).toContain("activity/subagent-not-found");
    record("ACT2-SUBAGENT-01", "subagent roster is parent filtered; confirm interrupts the child");
  });

  test("ACT2-SCHEDULE-01: switch + edit interval/prompt; invalid values are rejected inline without writing", { tag: ["@interaction:ACT2-SCHEDULE-01"] }, async ({ page }) => {
    seedSchedule();
    await openClient(page);
    await openInspector(page);
    await openActivityTab(page);
    const panel = page.getByTestId("activity-panel");
    await expect(panel.getByText("nightly")).toBeVisible({ timeout: 20_000 });
    // Toggle enabled through the switch (tool-owned atomic write).
    await panel.getByRole("switch", { name: "启用 schedule-1" }).click();
    await expect
      .poll(async () => {
        const snapshot = await snapshotOf(page);
        const schedules = numberArrayOf(snapshot["schedules"]);
        return isRecordValue(schedules[0]) ? schedules[0]["enabled"] : null;
      })
      .toBe(false);
    // Invalid interval is rejected inline and leaves the tool-owned file unchanged.
    await panel.getByRole("button", { name: "编辑" }).click();
    const interval = panel.getByLabel("间隔秒数（≥60）");
    await interval.fill("30");
    await panel.getByRole("button", { name: "保存" }).click();
    await expect(panel.getByText("间隔必须是 ≥60 的整数")).toBeVisible();
    const unchanged = await snapshotOf(page);
    const unchangedSchedules = numberArrayOf(unchanged["schedules"]);
    expect(isRecordValue(unchangedSchedules[0]) ? unchangedSchedules[0]["interval_seconds"] : null).toBe(3600);
    // Valid edit goes through the schedule tool.
    await interval.fill("120");
    await panel.getByLabel("提示词").fill("refreshed");
    await panel.getByRole("button", { name: "保存" }).click();
    await expect.poll(async () => {
      const snapshot = await snapshotOf(page);
      const schedules = numberArrayOf(snapshot["schedules"]);
      return isRecordValue(schedules[0]) ? schedules[0]["interval_seconds"] : null;
    }).toBe(120);
    const edited = numberArrayOf((await snapshotOf(page))["schedules"]);
    expect(stringOf(edited[0] ?? {}, "prompt")).toBe("refreshed");
    record("ACT2-SCHEDULE-01", "schedule switch/edit go through the tool; invalid interval rejected inline");
  });

  test("ACT2-SCHEDULE-02: delete needs inline confirm; unknown ids map to activity/schedule-invalid", { tag: ["@interaction:ACT2-SCHEDULE-02"] }, async ({ page }) => {
    seedSchedule();
    await openClient(page);
    await openInspector(page);
    await openActivityTab(page);
    const panel = page.getByTestId("activity-panel");
    await expect(panel.getByText("nightly")).toBeVisible({ timeout: 20_000 });
    // Cancel first: nothing changes.
    await panel.getByRole("button", { name: "删除" }).click();
    const dialog = page.getByRole("alertdialog", { name: "确认删除定时任务" });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole("button", { name: "取消" })).toBeFocused();
    await dialog.getByRole("button", { name: "取消" }).click();
    await expect(page.getByRole("alertdialog", { name: "确认删除定时任务" })).toHaveCount(0);
    expect(numberArrayOf((await snapshotOf(page))["schedules"]).length).toBe(1);
    // Keyboard-confirm the deletion.
    await panel.getByRole("button", { name: "删除" }).click();
    const keyboardDialog = page.getByRole("alertdialog", { name: "确认删除定时任务" });
    await expect(keyboardDialog).toBeVisible();
    await keyboardDialog.getByRole("button", { name: "确认" }).focus();
    await page.keyboard.press("Enter");
    await expect(panel.getByText("暂无定时任务")).toBeVisible({ timeout: 20_000 });
    await expect.poll(async () => numberArrayOf((await snapshotOf(page))["schedules"]).length).toBe(0);
    // Unknown/removed ids are readable errors (host pre-checks through `list`).
    const ids = await activeIds(page);
    const unknown = await callIpcResponse(page, "activity.action", {
      workspace_id: ids.workspaceId,
      conversation_id: ids.conversationId,
      action: "schedule_delete",
      id: "schedule-404",
    });
    expect(unknown.ok).toBe(false);
    expect(unknown.error).toContain("activity/schedule-invalid");
    const again = await callIpcResponse(page, "activity.action", {
      workspace_id: ids.workspaceId,
      conversation_id: ids.conversationId,
      action: "schedule_delete",
      id: "schedule-1",
    });
    expect(again.ok).toBe(false);
    expect(again.error).toContain("activity/schedule-invalid");
    record("ACT2-SCHEDULE-02", "schedule delete requires inline confirm; unknown ids are readable errors");
  });

  test("ACT2-TURNTAIL-01: turn tail lists present deliverables and opens the right-panel preview", { tag: ["@interaction:ACT2-TURNTAIL-01"] }, async ({ page }) => {
    await openClient(page);
    await sendMessage(page, "待办与交付");
    await expect(page.getByText("Hello from ZeoKeel", { exact: true }).first()).toBeVisible({
      timeout: 30_000,
    });
    await waitTurnDone(page);
    const card = page.getByTestId("turn-tail-present");
    await expect(card).toBeVisible();
    await expect(card).toContainText("hello.txt");
    await expect(card).toContainText("演示交付物");
    await card.getByRole("button", { name: "在右栏查看 hello.txt" }).click();
    // The right panel opens on the files preview (Phase 2b file_read).
    await expect(page.getByRole("complementary", { name: "右侧检查器" })).toBeVisible();
    await expect(page.getByText("demo-data/hello.txt").first()).toBeVisible();
    await expect(page.getByText("Hello ZeoKeel!").first()).toBeVisible();
    // No present → no empty card (fresh conversation, plain turn).
    await startNewConversation(page);
    await expect(page.getByTestId("turn-tail-present")).toHaveCount(0);
    await sendMessage(page, "普通问题");
    await expect(page.getByText("Hello from ZeoKeel", { exact: true }).first()).toBeVisible({
      timeout: 30_000,
    });
    await waitTurnDone(page);
    await expect(page.getByTestId("turn-tail-present")).toHaveCount(0);
    record("ACT2-TURNTAIL-01", "turn tail shows present deliverables; click opens the files preview");
  });

  test("ACT2-STALE-01: a polling failure keeps the last snapshot with a stale marker", { tag: ["@interaction:ACT2-STALE-01"] }, async ({ page }) => {
    seedSchedule();
    await openClient(page);
    await openInspector(page);
    await openActivityTab(page);
    const panel = page.getByTestId("activity-panel");
    await expect(panel.getByText("nightly")).toBeVisible({ timeout: 20_000 });
    await page.route("**/ipc", async (route) => {
      const body: unknown = route.request().postDataJSON();
      const name = isRecordValue(body) ? stringOf(body, "name") : "";
      if (name === "activity.snapshot") {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({ ok: false, error: "activity/failed: transient" }),
        });
        return;
      }
      await route.continue();
    });
    await expect(panel.getByText("轮询失败，显示上次成功的数据")).toBeVisible({ timeout: 15_000 });
    // Old data is kept (no white panel, no repeated toast spam).
    await expect(panel.getByText("nightly")).toBeVisible();
    await page.unroute("**/ipc");
    await expect(panel.getByText("轮询失败，显示上次成功的数据")).toHaveCount(0, { timeout: 15_000 });
    clearSchedules();
    record("ACT2-STALE-01", "polling failure keeps the last snapshot and shows a stale marker");
  });

  test("ACT2-ERROR-01: failed load shows a section error with retry; empty states never white out", { tag: ["@interaction:ACT2-ERROR-01"] }, async ({ page }) => {
    clearSchedules();
    await openClient(page);
    await openInspector(page);
    await page.route("**/ipc", async (route) => {
      const body: unknown = route.request().postDataJSON();
      const name = isRecordValue(body) ? stringOf(body, "name") : "";
      if (name === "activity.snapshot") {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({ ok: false, error: "activity/failed: boom" }),
        });
        return;
      }
      await route.continue();
    });
    await openActivityTab(page);
    const panel = page.getByTestId("activity-panel");
    await expect(panel.getByRole("alert")).toBeVisible({ timeout: 15_000 });
    await expect(panel.getByText("活动加载失败，请重试")).toBeVisible();
    await page.unroute("**/ipc");
    await panel.getByRole("button", { name: "重试" }).click();
    await expect(panel.getByText("暂无待办")).toBeVisible({ timeout: 20_000 });
    await expect(panel.getByText("暂无子代理")).toBeVisible();
    await expect(panel.getByText("暂无交付物")).toBeVisible();
    await expect(panel.getByText("当前会话暂无目标")).toBeVisible();
    record("ACT2-ERROR-01", "failed activity load shows an error with retry; sections keep their empty states");
  });

  test("ACT2-A11Y-01: axe 0 serious/critical; tabs, actions and confirmations are keyboard reachable", { tag: ["@interaction:ACT2-A11Y-01"] }, async ({ page }) => {
    seedSchedule();
    await openClient(page);
    await sendMessage(page, "待办与交付");
    await expect(page.getByText("Hello from ZeoKeel", { exact: true }).first()).toBeVisible({
      timeout: 30_000,
    });
    await waitTurnDone(page);
    await openInspector(page);
    // Keyboard tab traversal: reach the activity tab with ArrowRight from files.
    await page.getByRole("tab", { name: "文件" }).focus();
    for (let index = 0; index < 3; index += 1) {
      await page.keyboard.press("ArrowRight");
    }
    await expect(page.getByRole("tab", { name: "活动" })).toHaveAttribute("aria-selected", "true");
    await expect(page.getByTestId("activity-panel")).toBeVisible();
    // The refresh action is keyboard reachable.
    await page.getByRole("button", { name: "刷新活动" }).focus();
    await expect(page.getByRole("button", { name: "刷新活动" })).toBeFocused();
    // Destructive confirmations expose an accessible alertdialog; Esc closes it.
    await page.getByTestId("activity-panel").getByRole("button", { name: "删除" }).first().click();
    const dialog = page.getByRole("alertdialog", { name: "确认删除定时任务" });
    await expect(dialog).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    await axeClean(page, "activity panel");
    record("ACT2-A11Y-01", "axe clean; activity tab/actions/confirmations keyboard reachable");
  });

  test("ACT2-VIS-01: activity default screen candidate baseline (P2; human confirmation required)", { tag: ["@interaction:ACT2-VIS-01"] }, async ({ page }) => {
    clearSchedules();
    await openClient(page);
    await openInspector(page);
    await openActivityTab(page);
    await expect(page.getByTestId("activity-panel")).toBeVisible();
    // P2: the committed baseline is only accepted by a human via
    // `pnpm e2e:ui:update`; this run records a candidate screenshot instead of
    // failing on a missing/adjusted snapshot (C20 visual-baseline rule).
    await page.screenshot({
      path: join(SCREENSHOTS, "activity-light-1280.candidate.png"),
      animations: "disabled",
      caret: "hide",
    });
    record("ACT2-VIS-01", "activity light/1280 candidate screenshot captured (human confirmation pending)");
  });
});
