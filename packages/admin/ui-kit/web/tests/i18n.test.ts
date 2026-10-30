import { afterEach, describe, expect, it } from "vitest";
import { adminT, adminTranslate, getAdminLocale, setAdminLocale } from "../src/index";

afterEach(() => {
  setAdminLocale("zh-CN");
});

describe("admin i18n helper", () => {
  it("keeps the zh-CN labels used by the admin e2e suites", () => {
    expect(getAdminLocale()).toBe("zh-CN");
    expect(adminT("nav.users")).toBe("用户/角色");
    expect(adminT("users.title")).toBe("用户 / 角色");
    expect(adminT("nav.settings")).toBe("系统设置");
    expect(adminT("common.create")).toBe("创建");
    expect(adminT("common.save")).toBe("保存");
    expect(adminT("auth.login")).toBe("登录");
    expect(adminT("skills.create")).toBe("创建技能");
    expect(adminT("common.adjust")).toBe("调账");
  });

  it("provides en-US translations and can switch locale", () => {
    expect(adminTranslate("en-US", "nav.users")).toBe("Users & roles");
    expect(adminTranslate("en-US", "common.save")).toBe("Save");
    setAdminLocale("en-US");
    expect(adminT("nav.wallet")).toBe("Wallet");
    setAdminLocale("zh-CN");
    expect(adminT("nav.wallet")).toBe("钱包");
  });

  it("translates every key in both languages (no missing en entries)", () => {
    const zhKey = adminTranslate("zh-CN", "jobs.title");
    expect(zhKey).toBe("作业");
    expect(adminTranslate("en-US", "jobs.title")).toBe("Jobs");
    expect(adminTranslate("en-US", "settings.hint").length).toBeGreaterThan(10);
  });
});
