/**
 * B13 i18n completeness: zh-CN/en-US dictionaries must stay in lockstep.
 *
 * The TS type `Record<MessageKey, string>` already fails the build on a missing
 * key; this test additionally guards the runtime invariants a type cannot
 * express: no empty copy, matching `{placeholder}` sets, and every key resolving
 * through `translate` in both locales (no silent key echo).
 */
import { describe, expect, it } from "vitest";
import { messagesByLocale, translate } from "../src/i18n";

const LOCALES = ["zh-CN", "en-US"] as const;

function placeholders(template: string): string[] {
  return [...template.matchAll(/\{(\w+)\}/g)].map((match) => match[1] ?? "").sort();
}

describe("i18n dictionary parity (B13)", () => {
  it("has the same key set in zh-CN and en-US", () => {
    const zh = Object.keys(messagesByLocale["zh-CN"]).sort();
    const en = Object.keys(messagesByLocale["en-US"]).sort();
    expect(en).toEqual(zh);
    expect(zh.length).toBeGreaterThan(100);
  });

  it("has no empty copy and no duplicated placeholders in either locale", () => {
    for (const locale of LOCALES) {
      for (const [key, template] of Object.entries(messagesByLocale[locale])) {
        expect(template.trim(), `${locale} ${key} must not be empty`).not.toBe("");
        const names = placeholders(template);
        expect(new Set(names).size, `${locale} ${key} duplicates a placeholder`).toBe(names.length);
      }
    }
  });

  it("uses identical placeholder names across locales", () => {
    for (const key of Object.keys(messagesByLocale["zh-CN"])) {
      const zh = placeholders(messagesByLocale["zh-CN"][key] ?? "");
      const en = placeholders(messagesByLocale["en-US"][key] ?? "");
      expect(en, `${key} placeholder mismatch`).toEqual(zh);
    }
  });

  it("translates every key in both locales (no key echo)", () => {
    for (const locale of LOCALES) {
      for (const key of Object.keys(messagesByLocale["zh-CN"])) {
        expect(translate(locale, key), `${locale} ${key}`).not.toBe(key);
      }
    }
  });

  it("keeps the settings web-search and language keys localized (B11/B13)", () => {
    const zh = messagesByLocale["zh-CN"];
    const en = messagesByLocale["en-US"];
    expect(zh["settingsSectionWebSearch"]).toBe("网络搜索");
    expect(en["settingsSectionWebSearch"]).toBe("Web search");
    expect(zh["settingsLanguageZh"]).toBe("中文（简体）");
    expect(en["settingsLanguageEn"]).toBe("English (US)");
    expect(zh["settingsInvalidSecureUrl"]).toContain("https://");
    expect(en["settingsInvalidSecureUrl"]).toContain("https://");
  });
});
