import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { App } from "../src/App";
import { portalHref, parseUpgradeInfo, upgradeInfoOf, UpgradeRequiredError } from "../src/upgrade";
import type { Transport } from "../src/types";
import { readyTransport } from "./fixtures";

describe("forced upgrade 426 parsing", () => {
  it("parses the flat server body shape", () => {
    expect(parseUpgradeInfo(426, JSON.stringify({ ok: false, error: "CLIENT_TOO_OLD", current: "0.1.0", minimum: "0.3.0", portal_url: "https://srv/portal" }))).toEqual({
      current: "0.1.0",
      minimum: "0.3.0",
      portalUrl: "https://srv/portal",
    });
  });

  it("parses the nested error-object body shape", () => {
    expect(parseUpgradeInfo(426, JSON.stringify({ error: { code: "CLIENT_TOO_OLD", current_version: "1.2.3", min: "2.0.0" } }))).toEqual({
      current: "1.2.3",
      minimum: "2.0.0",
      portalUrl: null,
    });
  });

  it("treats any 426 as a forced upgrade, even without versions/body", () => {
    expect(parseUpgradeInfo(426, "not json")).toEqual({ current: null, minimum: null, portalUrl: null });
    expect(parseUpgradeInfo(409, JSON.stringify({ error: "CLIENT_TOO_OLD" }))).toBeNull();
    expect(parseUpgradeInfo(500, "")).toBeNull();
  });

  it("extracts upgrade info from typed/string errors and builds /portal hrefs", () => {
    const info = { current: "0.1.0", minimum: "0.2.0", portalUrl: null };
    expect(upgradeInfoOf(new UpgradeRequiredError(info))).toEqual(info);
    expect(upgradeInfoOf(new Error("CLIENT_TOO_OLD"))).toEqual({ current: null, minimum: null, portalUrl: null });
    expect(upgradeInfoOf(new Error("network"))).toBeNull();
    expect(portalHref("https://server.example/")).toBe("https://server.example/portal");
    expect(portalHref("")).toBeNull();
  });
});

describe("forced upgrade gate", () => {
  function upgradeTransport(error: unknown, calls: string[]): Transport {
    const base = readyTransport([]);
    return {
      async call(name, payload) {
        calls.push(name);
        if (name === "meta.get") throw error;
        return base.call(name, payload);
      },
      subscribe: (handler) => base.subscribe(handler),
    };
  }

  it("shows a full-screen, non-dismissable upgrade page and stops business calls", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const calls: string[] = [];
    const transport = upgradeTransport(new UpgradeRequiredError({ current: "0.1.0", minimum: "0.3.0", portalUrl: "https://srv.example/portal" }), calls);
    render(<App transport={transport} />);

    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(screen.getByText("需要更新客户端")).toBeTruthy();
    expect(screen.getByText("0.1.0")).toBeTruthy();
    expect(screen.getByText("0.3.0")).toBeTruthy();
    const link = screen.getByRole("link", { name: "打开下载页" });
    expect(link.getAttribute("href")).toBe("https://srv.example/portal");
    expect(link.getAttribute("target")).toBe("_blank");
    // No dismiss/retry controls, and boot stops after the failing meta.get (no business retry).
    expect(screen.queryByRole("button")).toBeNull();
    expect(screen.queryByText(/重试|关闭|取消/)).toBeNull();
    await new Promise((resolve) => setTimeout(resolve, 20));
    // The gate is terminal: no automatic retry/refresh calls after it appears (initial boot
    // requests may already be in flight, but nothing new is issued).
    expect(calls[0]).toBe("meta.get");
    const afterGate = calls.length;
    await new Promise((resolve) => setTimeout(resolve, 30));
    expect(calls.length).toBe(afterGate);
    warn.mockRestore();
  });

  it("falls back to server_url + /portal and unknown versions for bare 426 errors", async () => {
    localStorage.setItem("zk.enterprise.server", "https://srv.example/");
    const calls: string[] = [];
    const transport = upgradeTransport(new Error("CLIENT_TOO_OLD"), calls);
    render(<App transport={transport} />);
    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(screen.getByText("需要更新客户端")).toBeTruthy();
    expect(screen.getAllByText("未知").length).toBeGreaterThanOrEqual(2);
    const link = screen.getByRole("link", { name: "打开下载页" });
    expect(link.getAttribute("href")).toBe("https://srv.example/portal");
  });
});
