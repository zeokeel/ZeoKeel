import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "../src/App";
import { UpgradeGate } from "../src/UpgradeGate";
import { portalHref, parseUpgradeInfo, resolvePortalHref, upgradeInfoOf, UpgradeRequiredError } from "../src/upgrade";
import type { Transport } from "../src/types";
import { readyTransport } from "./fixtures";

beforeEach(() => {
  localStorage.removeItem("zk.enterprise.server");
});

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

  it("locks portal URLs to http(s) same-origin; unsafe/offsite/protocol-relative fall back", () => {
    const server = "https://srv.example";
    expect(resolvePortalHref("https://srv.example/download", server)).toBe("https://srv.example/download");
    expect(resolvePortalHref("/download", server)).toBe("https://srv.example/download");
    expect(resolvePortalHref("portal", server)).toBe("https://srv.example/portal");
    for (const unsafe of ["javascript:alert(1)", "data:text/html,x", "file:///etc/passwd", "//evil.example/portal", "https://evil.example/portal", "http://srv.example/portal"]) {
      expect(resolvePortalHref(unsafe, server)).toBe("https://srv.example/portal");
    }
    expect(resolvePortalHref(null, server)).toBe("https://srv.example/portal");
    expect(resolvePortalHref("https://srv.example/portal", "https://srv.example/app")).toBe("https://srv.example/portal");
    expect(resolvePortalHref("https://srv.example/portal", "")).toBeNull();
    expect(resolvePortalHref("javascript:alert(1)", "")).toBeNull();
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
    localStorage.setItem("zk.enterprise.server", "https://srv.example");
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const calls: string[] = [];
    const transport = upgradeTransport(new UpgradeRequiredError({ current: "0.1.0", minimum: "0.3.0", portalUrl: "https://srv.example/portal" }), calls);
    render(<App transport={transport} />);

    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(screen.getByText("需要更新客户端")).toBeTruthy();
    expect(screen.getAllByText("0.1.0").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText("0.3.0").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("查看版本说明")).toBeTruthy();
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

  it("resolves a relative portal_url against the active server URL", async () => {
    localStorage.setItem("zk.enterprise.server", "https://srv.example");
    const calls: string[] = [];
    const transport = upgradeTransport(
      new UpgradeRequiredError({ current: "0.1.0", minimum: "9.9.9", portalUrl: "/portal" }),
      calls,
    );
    render(<App transport={transport} />);
    expect(await screen.findByRole("alert")).toBeTruthy();
    const link = screen.getByRole("link", { name: "打开下载页" });
    expect(link.getAttribute("href")).toBe("https://srv.example/portal");
  });

  it("shows the server version in the version-details block when available", () => {
    const base = readyTransport([]);
    render(
      <UpgradeGate
        info={{ current: "0.1.0", minimum: "9.9.9", portalUrl: "/portal" }}
        serverUrl="https://srv.example"
        serverVersion="1.4.2"
        locale="zh-CN"
        transport={base}
      />,
    );
    expect(screen.getByText("查看版本说明")).toBeTruthy();
    expect(screen.getByText("服务器版本")).toBeTruthy();
    expect(screen.getByText("1.4.2")).toBeTruthy();
    expect(screen.getByRole("link", { name: "打开下载页" }).getAttribute("href")).toBe("https://srv.example/portal");
  });

  it("opens the portal through system.open_url in Tauri and window.open in the browser", async () => {
    localStorage.setItem("zk.enterprise.server", "https://srv.example");
    const calls: string[] = [];
    // Malicious 426 portal_url must be replaced by the validated server-origin fallback.
    const transport = upgradeTransport(new UpgradeRequiredError({ current: "0.1.0", minimum: "9.9.9", portalUrl: "javascript:alert(1)" }), calls);
    const openSpy = vi.spyOn(window, "open").mockImplementation(() => null);
    const first = render(<App transport={transport} />);
    expect(await screen.findByRole("alert")).toBeTruthy();
    await userEvent.click(screen.getByRole("link", { name: "打开下载页" }));
    expect(openSpy).toHaveBeenCalledWith("https://srv.example/portal", "_blank", "noopener");
    first.unmount();

    Reflect.set(window, "__TAURI_INTERNALS__", {});
    const tauriCalls: string[] = [];
    const openedUrls: unknown[] = [];
    const base = readyTransport([]);
    const tauriTransport: Transport = {
      async call(name, payload) {
        tauriCalls.push(name);
        if (name === "meta.get") throw new UpgradeRequiredError({ current: "0.1.0", minimum: "9.9.9", portalUrl: "https://evil.example/portal" });
        if (name === "system.open_url") {
          openedUrls.push(payload["url"]);
          return {};
        }
        return base.call(name, payload);
      },
      subscribe: (handler) => base.subscribe(handler),
    };
    render(<App transport={tauriTransport} />);
    expect(await screen.findByRole("alert")).toBeTruthy();
    await userEvent.click(screen.getByRole("link", { name: "打开下载页" }));
    expect(tauriCalls).toContain("system.open_url");
    expect(openedUrls).toEqual(["https://srv.example/portal"]);
    Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
    openSpy.mockRestore();
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
