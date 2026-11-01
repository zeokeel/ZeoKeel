import { describe, expect, it } from "vitest";
import { buildCommandCatalog } from "../src/App";
import { translate } from "../src/i18n";

const t = (key: string, vars?: Record<string, string | number>): string =>
  translate("zh-CN", key, vars);

describe("single command catalog (⌘K + composer /)", () => {
  it("personal has exactly 6 commands; enterprise adds the 3 content panels", () => {
    const personal = buildCommandCatalog({ t, enterprise: false, streaming: false, lastSendText: null })
      .filter((command) => command.surface === "palette");
    expect(personal.map((command) => command.id)).toEqual([
      "new-conversation",
      "toggle-sidebar",
      "open-settings",
      "toggle-inspector",
      "cancel",
      "retry",
    ]);
    const enterprise = buildCommandCatalog({ t, enterprise: true, streaming: false, lastSendText: null })
      .filter((command) => command.surface === "palette");
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
    const idle = buildCommandCatalog({ t, enterprise: false, streaming: false, lastSendText: null })
      .filter((command) => command.surface === "palette");
    expect(idle.find((command) => command.id === "cancel")?.disabled).toBe(true);
    expect(idle.find((command) => command.id === "retry")?.disabled).toBe(true);
    const streaming = buildCommandCatalog({ t, enterprise: false, streaming: true, lastSendText: "hi" })
      .filter((command) => command.surface === "palette");
    // All three create entries share one streaming guard; ⌘K shows the reason.
    const newConversation = streaming.find((command) => command.id === "new-conversation");
    expect(newConversation?.disabled).toBe(true);
        expect(newConversation?.disabledReason).toBe("当前正在生成，停止后可新建");
    expect(streaming.find((command) => command.id === "cancel")?.disabled).toBe(false);
    expect(streaming.find((command) => command.id === "retry")?.disabled).toBe(true);
    const afterTurn = buildCommandCatalog({ t, enterprise: false, streaming: false, lastSendText: "hi" })
      .filter((command) => command.surface === "palette");
    expect(afterTurn.find((command) => command.id === "retry")?.disabled).toBe(false);
  });
});

  it("composer slash rows are DSH commands (Batch2 placeholders disabled, no UI actions)", () => {
    const catalog = buildCommandCatalog({ t, enterprise: false, streaming: false, lastSendText: null });
    const slash = catalog.filter((command) => command.surface === "slash");
    expect(slash.map((command) => command.name)).toEqual([
      "file",
      "goal",
      "plan",
      "feedback",
      "compact",
      "permission",
      "model",
      "export",
      "skill",
    ]);
    // Batch2 progress: `/compact` is a real command; plan/skill stay disabled placeholders.
    expect(slash.filter((command) => command.disabled === true).map((command) => command.name)).toEqual([
      "plan",
      "skill",
    ]);
    const compact = slash.find((command) => command.name === "compact");
    expect(compact?.kind).toBe("execute");
    expect(compact?.disabled).toBeUndefined();
    expect(slash.some((command) => command.id === "new-conversation")).toBe(false);
  });
