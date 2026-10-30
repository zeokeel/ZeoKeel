import { describe, expect, it } from "vitest";
import { defaultSettings, isRecord, readBoolean, readNumber, readString } from "../src/index";

describe("ui-kit runtime guards", () => {
  it("narrows unknown records without assertions", () => {
    expect(isRecord({})).toBe(true);
    expect(isRecord([])).toBe(false);
    expect(isRecord(null)).toBe(false);
    expect(readString({ value: "x" }, "value")).toBe("x");
    expect(readString({ value: 1 }, "value")).toBeNull();
    expect(readNumber({ value: 2 }, "value")).toBe(2);
    expect(readBoolean({ value: false }, "value")).toBe(false);
  });

  it("exposes safe defaults", () => {
    const settings = defaultSettings();
    expect(settings.schema).toBe(1);
    expect(settings.locale).toBe("zh-CN");
    expect(settings.theme).toBe("auto");
  });
});
