import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../src/App";
import type { FakeTransport } from "./fake-transport";
import { readyTransport } from "./fixtures";

async function openChangesTab(transport: FakeTransport): Promise<void> {
  render(<App transport={transport} />);
  await screen.findByRole("heading", { name: "今天想完成什么？" });
  await userEvent.click(screen.getByRole("button", { name: "切换右栏" }));
  await screen.findByRole("complementary", { name: "右侧检查器" });
  await userEvent.click(screen.getByRole("tab", { name: "变更" }));
}

describe("inspector changes tab (workspace.changes)", () => {
  it("renders changed paths and a non-blocking warning", async () => {
    const transport = readyTransport([]);
    transport.handle("workspace.changes", () => ({
      items: [
        { path: "src/main.rs", add: 1, del: 0 },
        { path: "docs/old.md", add: 0, del: 1 },
      ],
      warning: "not a git repository or git status failed: exit status: 128",
    }));
    await openChangesTab(transport);
    expect(await screen.findByText(/src\/main\.rs \+1 −0/)).toBeTruthy();
    expect(screen.getByText(/docs\/old\.md \+0 −1/)).toBeTruthy();
    expect(screen.getByRole("status").textContent).toContain("变更扫描提示");
    expect(transport.callsFor("workspace.changes")[0]).toMatchObject({ workspace_id: "ws-home" });
  });

  it("shows the empty state for a clean workspace", async () => {
    const transport = readyTransport([]);
    transport.handle("workspace.changes", () => ({ items: [] }));
    await openChangesTab(transport);
    expect(await screen.findByText("工作区没有未提交的变更")).toBeTruthy();
    expect(screen.queryByRole("status")).toBeNull();
  });

  it("shows loading while the scan is running", async () => {
    const transport = readyTransport([]);
    let resolveScan: ((value: unknown) => void) | null = null;
    transport.handle("workspace.changes", () => new Promise((resolve) => { resolveScan = resolve; }));
    await openChangesTab(transport);
    expect(await screen.findByText("正在加载变更…")).toBeTruthy();
    await act(async () => {
      if (resolveScan !== null) resolveScan({ items: [{ path: "a.txt", add: 1, del: 0 }] });
      await Promise.resolve();
    });
    expect(await screen.findByText(/a\.txt \+1 −0/)).toBeTruthy();
  });

  it("renders a retryable error and refetches on retry", async () => {
    const transport = readyTransport([]);
    transport.fail("workspace.changes", "git status failed: timeout");
    await openChangesTab(transport);
    expect(await screen.findByText("变更加载失败")).toBeTruthy();
    expect(screen.getByText("git status failed: timeout")).toBeTruthy();

    transport.failures.delete("workspace.changes");
    transport.handle("workspace.changes", () => ({ items: [{ path: "b.txt", add: 1, del: 1 }] }));
    await userEvent.click(screen.getByRole("button", { name: "重新加载" }));
    expect(await screen.findByText(/b\.txt \+1 −1/)).toBeTruthy();
    expect(transport.callsFor("workspace.changes").length).toBeGreaterThanOrEqual(2);
  });

  it("does not fetch while the files tab is active", async () => {
    const transport = readyTransport([]);
    render(<App transport={transport} />);
    await screen.findByRole("heading", { name: "今天想完成什么？" });
    await userEvent.click(screen.getByRole("button", { name: "切换右栏" }));
    await screen.findByRole("complementary", { name: "右侧检查器" });
    await waitFor(() => {
      expect(transport.callsFor("workspace.files").length).toBeGreaterThan(0);
    });
    expect(transport.callsFor("workspace.changes")).toHaveLength(0);
  });
});
