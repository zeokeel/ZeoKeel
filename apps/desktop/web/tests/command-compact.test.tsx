import { describe, expect, it } from "vitest";
import { parseCompactResult, parseCompactionGetResult } from "../src/protocol";

const MARKER = {
  marker_id: "compaction:abc",
  summary: "earlier summary",
  through_message_id: "m03",
  created_ms: 10,
  input_tokens: 30,
  output_tokens: 12,
  compressed_messages: 4,
  keep_recent: 6,
};

describe("/compact boundary validation", () => {
  it("parses a compact result with marker and noop flag", () => {
    expect(parseCompactResult({ marker: MARKER, noop: false })).toEqual({
      marker: {
        markerId: "compaction:abc",
        summary: "earlier summary",
        throughMessageId: "m03",
        createdMs: 10,
        inputTokens: 30,
        outputTokens: 12,
        compressedMessages: 4,
        keepRecent: 6,
      },
      noop: false,
    });
    expect(parseCompactResult({ marker: MARKER, noop: true })?.noop).toBe(true);
    // Invalid marker payloads are rejected at the boundary (C10).
    expect(parseCompactResult({ marker: { marker_id: "x" }, noop: false })).toBeNull();
    expect(parseCompactResult("nope")).toBeNull();
  });

  it("parses compaction_get (marker or null)", () => {
    expect(parseCompactionGetResult({ marker: MARKER })?.marker?.markerId).toBe("compaction:abc");
    expect(parseCompactionGetResult({ marker: null })).toEqual({ marker: null });
    expect(parseCompactionGetResult({})).toEqual({ marker: null });
  });
});
