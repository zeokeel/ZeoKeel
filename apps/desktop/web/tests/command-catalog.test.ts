import { describe, expect, it } from "vitest";
import { buildCommandCatalog } from "../src/App";
import { translate } from "../src/i18n";

const t = (key: string, vars?: Record<string, string | number>): string =>
  translate("zh-CN", key, vars);

describe("single command catalog (⌘K + composer /)", () => {
  it("personal has exactly 6 commands; enterprise adds the 3 content panels", () => {
    const personal = buildCommandCatalog({ t, enterprise: false, streaming: false, lastSendText: null });
    expect(personal.map((command) => command.id)).toEqual([
      "new-conversation",
      "toggle-sidebar",
      "open-settings",
      "toggle-inspector",
      "cancel",
      "retry",
    ]);
    const enterprise = buildCommandCatalog({ t, enterprise: true, streaming: false, lastSendText: null });
    expect(enterprise.map((command) => command.id)).toEqual([
      "new-conversation",
      "toggle-sidebar",
      "open-settings",
      "toggle-inspector",
      "cancel",
      "retry",
      "open-skills",
      "open-connectors",
      "open-apps",
    ]);
  });

  it("gates cancel/retry by streaming and last-send state", () => {
    const idle = buildCommandCatalog({ t, enterprise: false, streaming: false, lastSendText: null });
    expect(idle.find((command) => command.id === "cancel")?.disabled).toBe(true);
    expect(idle.find((command) => command.id === "retry")?.disabled).toBe(true);
    const streaming = buildCommandCatalog({ t, enterprise: false, streaming: true, lastSendText: "hi" });
    expect(streaming.find((command) => command.id === "cancel")?.disabled).toBe(false);
    expect(streaming.find((command) => command.id === "retry")?.disabled).toBe(true);
    const afterTurn = buildCommandCatalog({ t, enterprise: false, streaming: false, lastSendText: "hi" });
    expect(afterTurn.find((command) => command.id === "retry")?.disabled).toBe(false);
  });
});
