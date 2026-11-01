import { expect, type Locator, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/**
 * Shared human-journey helpers for the baseline interaction suite
 * (`specs/changes/2026-11-01-ui-interaction-verification/interaction.md`).
 *
 * Locators are role/accessible-name based on purpose; the few structural
 * locators below are the documented test hooks of the shipped surface.
 */
export const SERVE_URL = "http://127.0.0.1:8788";
export const EVIDENCE_ROOT = "target/evidence/2026-11-01-ui-interaction-verification";
const INTERACTION_EVIDENCE = join(EVIDENCE_ROOT, "interaction");

export interface InteractionEvidence {
  id: string;
  title: string;
  at: string;
  passed: boolean;
}

/**
 * Writes the per-ID evidence JSON (`{id,title,at,passed}`) at the end of a
 * passing test (design.md §2.4). Only reachable after the test body passes, so
 * the file existing means that ID passed in this run.
 */
export function recordInteraction(id: string, title: string): void {
  mkdirSync(INTERACTION_EVIDENCE, { recursive: true });
  const evidence: InteractionEvidence = { id, title, at: new Date().toISOString(), passed: true };
  writeFileSync(join(INTERACTION_EVIDENCE, `${id}.json`), `${JSON.stringify(evidence, null, 2)}\n`, "utf8");
}

/** Opens the SPA on the real serve carrier and waits for the shell. */
export async function openClient(page: Page): Promise<void> {
  await page.goto(`${SERVE_URL}/`, { waitUntil: "networkidle" });
  await expect(page.getByRole("complementary", { name: "工作区与会话" })).toBeVisible();
}

/** The baseline composer input (`AssistantComposer`, role=textbox + aria-label). */
export function composerInput(page: Page): Locator {
  return page.getByRole("textbox", { name: "消息输入框" });
}

/** The enabled composer send action (exact name; "重试发送" also contains 发送). */
export function sendButton(page: Page): Locator {
  return page.getByRole("button", { name: "发送", exact: true });
}

/** User message paragraphs rendered by `AssistantThread` (test hook). */
export function userTexts(page: Page): Locator {
  return page.getByTestId("user-text");
}

/** Creates a fresh conversation through the sidebar "新建会话" action. */
export async function startNewConversation(page: Page): Promise<void> {
  await page.getByRole("button", { name: "新建会话" }).last().click();
  await expect(composerInput(page)).toBeVisible();
}

/** Ensures this test owns an active conversation (each test is independent). */
export async function ensureConversation(page: Page): Promise<void> {
  if ((await composerInput(page).count()) === 0) {
    await startNewConversation(page);
    return;
  }
  await expect(composerInput(page)).toBeVisible();
}

/** Types a message and submits it with the real Enter key. */
export async function sendMessage(page: Page, text: string): Promise<void> {
  const input = composerInput(page);
  await input.fill(text);
  await page.keyboard.press("Enter");
}

/** Waits for the deterministic fake-upstream assistant reply. */
export async function expectReply(page: Page): Promise<void> {
  await expect(page.getByText("Hello from ZeoKeel", { exact: true }).first()).toBeVisible();
}

/** Waits for the persisted usage part (fake upstream: in 10 / out 5 / total 15). */
export async function expectUsage(page: Page): Promise<void> {
  await expect(page.getByText("输入 10", { exact: true })).toBeVisible();
  await expect(page.getByText("输出 5", { exact: true })).toBeVisible();
  await expect(page.getByText("合计 15", { exact: true })).toBeVisible();
}

/** asserts axe has 0 serious/critical violations; failures print the ids. */
export async function expectNoSeriousAxe(page: Page, label: string): Promise<void> {
  const results = await new AxeBuilder({ page }).analyze();
  const found = results.violations
    .filter((violation) => violation.impact === "serious" || violation.impact === "critical")
    .map((violation) => violation.id);
  expect(found, `${label}: serious/critical axe violations`).toEqual([]);
}
