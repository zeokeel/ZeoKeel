import { describe, expect, it } from "vitest";
import { readUsage, usageFromReply } from "../src/UsagePart";

describe("agent usage parsing", () => {
  it("reads token/cache/reasoning/cost counters", () => {
    const summary = readUsage({
      usage: { input_tokens: 10, cached_tokens: 2, output_tokens: 4, reasoning_tokens: 1, total_tokens: 14, cost_micros: 5000 },
    });
    expect(summary).toEqual({ input: 10, cached: 2, output: 4, reasoning: 1, total: 14, costMicros: 5000 });
  });

  it("treats missing or invalid usage as absent", () => {
    expect(readUsage(null).input).toBeNull();
    expect(readUsage({ usage: { input_tokens: -1 } }).input).toBeNull();
    expect(readUsage({ usage: { input_tokens: "12" } }).input).toBeNull();
    expect(usageFromReply({ usage: {} })).toBeNull();
  });

  it("normalizes to the AgentUsage contract shape", () => {
    expect(usageFromReply({ usage: { input_tokens: 3, output_tokens: 2 } })).toEqual({
      input_tokens: 3,
      output_tokens: 2,
      total_tokens: null,
      cached_tokens: null,
      reasoning_tokens: null,
      cost_micros: null,
    });
  });
});
