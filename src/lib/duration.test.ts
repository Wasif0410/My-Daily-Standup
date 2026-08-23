import { describe, expect, it } from "vitest";
import { DURATION_PRESETS, formatMinutes, parseDuration } from "@/lib/duration";

describe("formatMinutes", () => {
  it.each([
    [null, "—"],
    [0, "0m"],
    [35, "35m"],
    [60, "1h"],
    [95, "1h 35m"],
    [1080, "18h"],
  ])("formats %s as %s", (minutes, expected) => {
    expect(formatMinutes(minutes)).toBe(expected);
  });

  it("shows a dash rather than a zero for unrecorded time", () => {
    // "0m" would claim the work took no time. It was simply never logged,
    // and the two must not look the same.
    expect(formatMinutes(null)).toBe("—");
    expect(formatMinutes(0)).toBe("0m");
  });
});

describe("parseDuration", () => {
  it.each([
    ["90", 90],
    ["45m", 45],
    ["2h", 120],
    ["1h30m", 90],
    ["1h 30m", 90],
    ["1.5h", 90],
    ["0", 0],
  ])("parses %s as %s minutes", (input, expected) => {
    expect(parseDuration(input)).toBe(expected);
  });

  it("accepts surrounding whitespace and mixed case", () => {
    expect(parseDuration("  2H 15M  ")).toBe(135);
  });

  it("treats a bare number as minutes", () => {
    // The most common entry is a two-digit number, so it must not require a
    // unit. Guessing hours would silently inflate every total sixtyfold.
    expect(parseDuration("30")).toBe(30);
  });

  it.each([
    ["", null],
    ["   ", null],
    ["soon", null],
    ["-15", null],
    ["h", null],
  ])("rejects %s", (input, expected) => {
    expect(parseDuration(input)).toBe(expected);
  });

  it("rejects a negative value rather than clamping it", () => {
    // Clamping would silently record something the user did not type.
    expect(parseDuration("-30")).toBeNull();
  });
});

describe("DURATION_PRESETS", () => {
  it("offers the common values in ascending order", () => {
    expect(DURATION_PRESETS.map((p) => p.minutes)).toEqual([15, 30, 60, 120]);
  });

  it("labels each preset the way it will be displayed", () => {
    for (const preset of DURATION_PRESETS) {
      expect(preset.label).toBe(formatMinutes(preset.minutes));
    }
  });
});
