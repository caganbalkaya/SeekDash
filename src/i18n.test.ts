import { describe, expect, it } from "vitest";
import { resources, translate, translatePlural } from "./i18n";

describe("i18n resources", () => {
  it("has the same keys in every language", () => {
    const base = Object.keys(resources.en).sort();
    for (const language of ["tr", "de", "fr"] as const) expect(Object.keys(resources[language]).sort()).toEqual(base);
  });
  it("interpolates values and falls back to English", () => {
    expect(translate("de", "common.items", { count: 3 })).toContain("3");
    expect(translate("en", "search.webTitle", { query: "glass" })).toContain("glass");
    const original = resources.de["common.settings"];
    delete (resources.de as Partial<typeof resources.de>)["common.settings"];
    expect(translate("de", "common.settings")).toBe(resources.en["common.settings"]);
    resources.de["common.settings"] = original;
    expect(translatePlural("en", 1, "common.item", "common.items")).toBe("1 item");
    expect(translatePlural("en", 2, "common.item", "common.items")).toBe("2 items");
  });
});
