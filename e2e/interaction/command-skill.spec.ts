import { expect, test, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/**
 * `/<skill>` skill catalog interaction contract (`2026-11-01-client-skill-commands`).
 *
 * Personal edition: the host reads `<workspace>/.agents/skills` (frontmatter
 * `user-invocable != false`, `SKILL.md` ≤8 KiB) through the single `zk-tool-skill`
 * implementation; selecting a candidate inserts a removable Instructions chip and
 * never sends. The e2e seeds the deterministic `serve-m1.sh` workspace.
 */
const SERVE = "http://127.0.0.1:8788";
const WORKSPACE_ROOT = "target/evidence/m1/home";
const EVIDENCE = "target/evidence/2026-11-01-client-skill-commands/interaction";
const SKILLS_ROOT = join(WORKSPACE_ROOT, ".agents/skills");
mkdirSync(EVIDENCE, { recursive: true });

function record(id: string, title: string): void {
  writeFileSync(join(EVIDENCE, `${id}.json`), `${JSON.stringify({ id, title, at: new Date().toISOString(), passed: true }, null, 2)}\n`, "utf8");
}

function seedSkill(name: string, body: string): void {
  const dir = join(SKILLS_ROOT, name);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "SKILL.md"), body, "utf8");
}

function isRecordValue(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function composer(page: Page): ReturnType<Page["getByRole"]> {
  return page.getByRole("textbox", { name: "消息输入框" });
}

async function openConversation(page: Page): Promise<void> {
  await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
  await expect(page.getByRole("complementary", { name: "工作区与会话" })).toBeVisible();
  await page.getByRole("button", { name: "新建会话", exact: true }).last().click();
  await expect(composer(page)).toBeVisible();
}

async function callIpc(page: Page, name: string, payload: Record<string, unknown>): Promise<unknown> {
  const response = await page.request.post(`${SERVE}/ipc`, { data: { name, payload } });
  const body: unknown = await response.json();
  return isRecordValue(body) ? body["data"] : undefined;
}

function idOf(row: Record<string, unknown> | undefined): string {
  const id = row?.["id"];
  return typeof id === "string" ? id : "";
}

function rowsOf(data: unknown): Record<string, unknown>[] {
  const entries: unknown[] = Array.isArray(data) ? data : [];
  return entries.filter(isRecordValue);
}

async function persistedText(page: Page): Promise<string> {
  const workspaceId = idOf(rowsOf(await callIpc(page, "workspace.list", {}))[0]);
  const conversationId = idOf(rowsOf(await callIpc(page, "session.list", { workspace_id: workspaceId }))[0]);
  const messages = rowsOf(await callIpc(page, "session.messages", {
    workspace_id: workspaceId,
    conversation_id: conversationId,
  }));
  return messages
    .filter((message) => message["role"] === "user")
    .map((message) => (typeof message["content"] === "string" ? message["content"] : ""))
    .join("\n");
}

async function axeClean(page: Page): Promise<void> {
  const results = await new AxeBuilder({ page }).analyze();
  const found = results.violations
    .filter((violation) => violation.impact === "serious" || violation.impact === "critical")
    .map((violation) => violation.id);
  expect(found, "serious/critical axe violations").toEqual([]);
}

test.describe("command-skill", () => {
  test.beforeAll(() => {
    // Deterministic personal workspace skills (real `.agents/skills` source).
    try {
      rmSync(SKILLS_ROOT, { recursive: true });
    } catch {
      // First run: nothing seeded yet.
    }
    seedSkill(
      "demo",
      "---\nname: demo\ndescription: 演示技能\ndescription-extra: ignored\n---\n# Demo skill\n\nUse the demo checklist.\n",
    );
    seedSkill("big", "---\nname: big\ndescription: 超大技能\n---\n" + "x".repeat(9 * 1024) + "\n");
    seedSkill("hidden", "---\nname: hidden\nuser-invocable: false\n---\n# hidden\n");
  });

  test("CMD3S-SKILL-01: / lists user-invocable skills, inserts a removable Instructions chip and the next message carries the block", { tag: ["@interaction:CMD3S-SKILL-01"] }, async ({ page }) => {
    await openConversation(page);
    await composer(page).click();
    await page.keyboard.type("/");
    const list = page.getByRole("listbox", { name: "命令" });
    await expect(list).toBeVisible();
    // Frontmatter name + description; `user-invocable: false` is never listed.
    const demo = page.getByRole("option", { name: /demo/ });
    await expect(demo).toBeVisible();
    await expect(demo).toContainText("演示技能");
    await expect(page.getByRole("option", { name: /hidden/ })).toHaveCount(0);
    // Filtering is name/description, case-insensitive.
    await composer(page).fill("/DEMO");
    await expect(page.getByRole("option", { name: /demo/ })).toBeVisible();
    await page.keyboard.press("Enter");
    // Selecting inserts a removable chip; nothing is sent yet.
    const chip = page.getByTestId("composer-skill-chip");
    await expect(chip).toBeVisible();
    await expect(chip).toContainText("技能指令：demo");
    await expect(page.getByTestId("user-text")).toHaveCount(0);
    await expect(composer(page)).toHaveValue("");
    // The block travels with the next message (nested above the reference block).
    await composer(page).fill("请按技能检查");
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("user-text").last()).toHaveText("请按技能检查");
    await expect(page.getByTestId("user-skill").last()).toContainText("技能指令：demo");
    await expect.poll(async () => persistedText(page)).toContain("--- skill instructions ---");
    const persisted = await persistedText(page);
    expect(persisted).toContain("skill: demo");
    expect(persisted).toContain("# Demo skill");
    expect(persisted).toContain("Use the demo checklist.");
    await axeClean(page);
    record("CMD3S-SKILL-01", "skill catalog lists user-invocable skills and sends removable Instructions with the next message");
  });

  test("CMD3S-KEY-01: keyboard filtering/accept, chip removal and Esc keep focus in the composer", { tag: ["@interaction:CMD3S-KEY-01"] }, async ({ page }) => {
    await openConversation(page);
    const input = composer(page);
    await input.click();
    await page.keyboard.type("/dem");
    const option = page.getByRole("option", { name: /demo/ });
    await expect(option).toBeVisible();
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("composer-skill-chip")).toBeVisible();
    // Chip removal is keyboard operable and returns focus to the composer.
    const remove = page.getByTestId("composer-skill-remove");
    await remove.focus();
    await expect(remove).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("composer-skill-chip")).toHaveCount(0);
    await expect(input).toBeFocused();
    // Esc closes the menu without clearing the draft.
    await input.fill("");
    await page.keyboard.type("/dem");
    await expect(page.getByRole("listbox", { name: "命令" })).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("listbox")).toHaveCount(0);
    await expect(input).toHaveValue("/dem");
    await expect(input).toBeFocused();
    record("CMD3S-KEY-01", "skill menu keyboard filtering/accept, chip removal and Esc focus");
  });

  test("CMD3S-ERR-01: loading/error/empty/too-large are readable and never fake-insert", { tag: ["@interaction:CMD3S-ERR-01"] }, async ({ page }) => {
    await openConversation(page);
    const input = composer(page);
    // Empty filter result: readable, no fake option.
    await input.fill("/no-such-skill-zzz");
    await expect(page.getByRole("listbox", { name: "命令" })).toContainText("没有匹配的命令");
    await expect(page.getByTestId("composer-skill-chip")).toHaveCount(0);
    // Loading/error state for skill.list with a retry affordance.
    await page.route("**/ipc", async (route) => {
      const body: unknown = route.request().postDataJSON();
      if (isRecordValue(body) && body["name"] === "skill.list") {
        await route.fulfill({ status: 500, body: "boom" });
        return;
      }
      await route.continue();
    });
    await input.fill("");
    await page.keyboard.type("/");
    await expect(page.getByText("技能加载失败")).toBeVisible();
    await expect(page.getByRole("button", { name: "重试加载技能" })).toBeVisible();
    await page.unroute("**/ipc");
    await page.getByRole("button", { name: "重试加载技能" }).click();
    await expect(page.getByRole("option", { name: /demo/ })).toBeVisible();
    // >8 KiB content is rejected with a readable message; the draft keeps no chip.
    await input.fill("/big");
    await page.getByRole("option", { name: /big/ }).click();
    await expect(page.getByText("技能内容超过 8 KiB，无法插入")).toBeVisible();
    await expect(page.getByTestId("composer-skill-chip")).toHaveCount(0);
    await expect(page.getByTestId("user-text")).toHaveCount(0);
    await axeClean(page);
    record("CMD3S-ERR-01", "skill loading/error/empty/too-large states are readable and never fake-insert");
  });
});
