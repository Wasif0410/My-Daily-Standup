import { describe, expect, it } from "vitest";
import { SECTION_COLORS, sectionColor } from "@/features/boards/sectionColor";

describe("sectionColor", () => {
  it("gives the same section the same colour every time", () => {
    expect(sectionColor("Job Search")).toBe(sectionColor("Job Search"));
  });

  it("matches a heading however it was typed", () => {
    // The boards treat "Job Search" and "  job search " as one heading, so
    // they must not come out in two colours.
    expect(sectionColor("  job search ")).toBe(sectionColor("Job Search"));
  });

  it("always picks from the palette", () => {
    expect(SECTION_COLORS).toContain(sectionColor("Thesis"));
  });

  it("spreads different sections across the palette", () => {
    const names = [
      "Job Search",
      "School",
      "Life",
      "Health",
      "Thesis",
      "Home",
      "Money",
      "Side project",
    ];
    const distinct = new Set(names.map(sectionColor));

    expect(distinct.size).toBeGreaterThanOrEqual(5);
  });

  it("leaves the catch-all headings neutral", () => {
    // "Unsorted" and "No project" are the absence of a section, not one.
    expect(sectionColor("Unsorted")).toBeNull();
    expect(sectionColor("No project")).toBeNull();
  });
});
