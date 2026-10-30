import { describe, expect, it } from "vitest";
import { readUsage, usageFromReply } from "../src/UsagePart";

describe("agent usage parsing", () => {
  it("reads required counters and optional cost", () => {
    const summary = readUsage({
      usage: { input_tokens: 10, cached_tokens: 2, output_tokens: 4, reasoning_tokens: 1, total_tokens: 14, cost_micros: 5000 },
    });
    expect(summary).toEqual({ input: 10, cached: 2, output: 4, reasoning: 1, total: 14, costMicros: 5000 });
  });

  it("normalizes absent/unreported counters to 0 (contract: required, runtime normalization)", () => {
    expect(readUsage({ usage: { input_tokens: 3 } })).toEqual({
      input: 3, cached: 0, output: 0, reasoning: 0, total: 0, costMicros: null,
    });
    expect(readUsage({ usage: {} })).toEqual({
      input: 0, cached: 0, output: 0, reasoning: 0, total: 0, costMicros: null,
    });
    expect(readUsage({ usage: { input_tokens: -1, cached_tokens: "2" } })).toEqual({
      input: 0, cached: 0, output: 0, reasoning: 0, total: 0, costMicros: null,
    });
  });

  it("returns null when no usage is present", () => {
    expect(readUsage(null)).toBeNull();
    expect(readUsage({})).toBeNull();
    expect(usageFromReply({})).toBeNull();
    expect(usageFromReply({ usage: null })).toBeNull();
  });

  it("normalizes to the AgentUsage contract shape (required counters, optional cost)", () => {
    expect(usageFromReply({ usage: { input_tokens: 3, output_tokens: 2, total_tokens: 5, cached_tokens: 0, reasoning_tokens: 0 } })).toEqual({
      input_tokens: 3,
      output_tokens: 2,
      total_tokens: 5,
      cached_tokens: 0,
      reasoning_tokens: 0,
      cost_micros: null,
    });
    expect(usageFromReply({ usage: { input_tokens: 1, output_tokens: 1, total_tokens: 2, cached_tokens: 0, reasoning_tokens: 0, cost_micros: 9 } })?.cost_micros).toBe(9);
  });
});
