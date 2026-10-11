import { describe, expect, it } from "vitest";
import { filterCommands, hexToRgb } from "./commandPalette";

describe("command palette", () => {
  it("filters command names by a case-insensitive prefix", () => {
    expect(filterCommands(">SL").map((command) => command.id)).toEqual(["sleep"]);
    expect(filterCommands(">")).toHaveLength(9);
    expect(filterCommands("sleep")).toEqual([]);
  });

  it("converts valid HEX colors and rejects incomplete input", () => {
    expect(hexToRgb("#FF5733")).toBe("rgb(255, 87, 51)");
    expect(hexToRgb("#0af")).toBe("rgb(0, 170, 255)");
    expect(hexToRgb("#FF57")).toBeNull();
  });
});
