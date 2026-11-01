import { describe, expect, it } from "vitest";
import type { Message } from "@zeokeel/ui-kit";
import { groupToolRows } from "../src/toolRows";

function message(
  id: string,
  role: string,
  content: string,
  turnId: string,
  createdAt: number,
): Message {
  return {
    id,
    conversation_id: "c",
    role,
    content,
    created_at_ms: createdAt,
    status: "done",
    usage: null,
    turn_id: turnId,
  };
}

function toolRow(id: string, turnId: string, payload: unknown): Message {
  return message(id, "tool", JSON.stringify(payload), turnId, 2);
}

describe("tool row grouping", () => {
  it("groups rows onto the assistant of the same turn", () => {
    const messages = [
      message("u1", "user", "go", "turn-1", 1),
      toolRow("t1", "turn-1", { name: "fs.write", args: "{}", status: "success", result: "ok" }),
      toolRow("t2", "turn-1", { name: "bash", args: "{}", status: "denied", result: "no" }),
      message("a1", "assistant", "done", "turn-1", 3),
    ];
    const grouped = groupToolRows(messages);
    expect(grouped.visible.map((row) => row.id)).toEqual(["u1", "a1"]);
    expect(grouped.toolsByAssistant.get("a1")?.map((tool) => tool.status)).toEqual([
      "success",
      "denied",
    ]);
    expect(grouped.unassigned).toEqual([]);
  });

  it("falls back to the next assistant for legacy rows and keeps trailing rows unassigned", () => {
    const legacy = [
      message("u1", "user", "go", "", 1),
      toolRow("t1", "", { name: "fs.read", result: "body" }),
      message("a1", "assistant", "done", "", 3),
      toolRow("t2", "", { name: "fs.read", result: "orphan" }),
    ];
    const grouped = groupToolRows(legacy);
    expect(grouped.toolsByAssistant.get("a1")?.map((tool) => tool.id)).toEqual(["t1"]);
    expect(grouped.toolsByAssistant.get("a1")?.[0]?.status).toBe("success");
    expect(grouped.unassigned.map((tool) => tool.id)).toEqual(["t2"]);
  });

  it("keeps unparseable rows as degraded raw records", () => {
    const messages = [
      message("u1", "user", "go", "turn-1", 1),
      message("t1", "tool", "not json", "turn-1", 2),
      message("a1", "assistant", "done", "turn-1", 3),
    ];
    const grouped = groupToolRows(messages);
    const tool = grouped.toolsByAssistant.get("a1")?.[0];
    expect(tool?.unparsed).toBe(true);
    expect(tool?.status).toBe("failed");
    expect(tool?.result).toBe("not json");
  });
});
