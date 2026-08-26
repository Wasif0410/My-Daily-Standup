import { describe, expect, it } from "vitest";
import { formatWeek } from "@/features/boards/weekLabel";
import type { Week } from "@/types/task";

function week(overrides: Partial<Week> = {}): Week {
  return {
    label: "2026-W35",
    start: "2026-08-24",
    end: "2026-08-30",
    today: "2026-08-25",
    days: [],
    ...overrides,
  };
}

describe("formatWeek", () => {
  it("reads the way someone would say it out loud", () => {
    expect(formatWeek(week())).toBe("Aug 24–30 · Week 35");
  });

  it("says the month once when the week does not leave it", () => {
    // A board is 340px wide. "Aug 24 – Aug 30" spends that width saying
    // August twice.
    expect(formatWeek(week())).not.toContain("Aug 30");
  });

  it("says both months when the week crosses a boundary", () => {
    const crossing = week({
      label: "2026-W36",
      start: "2026-08-31",
      end: "2026-09-06",
    });

    // "Aug 31–6" is unreadable; the second month has to survive here.
    expect(formatWeek(crossing)).toBe("Aug 31–Sep 6 · Week 36");
  });

  it("does not shift the date into the previous day", () => {
    // `new Date("2026-08-24")` is UTC midnight, which renders as the 23rd in
    // any negative offset. Parsing by hand is what keeps this right for
    // everyone west of Greenwich.
    expect(formatWeek(week({ start: "2026-08-24" }))).toContain("Aug 24");
  });

  it("handles a single-digit week number", () => {
    const early = week({ label: "2026-W03", start: "2026-01-12", end: "2026-01-18" });

    expect(formatWeek(early)).toBe("Jan 12–18 · Week 03");
  });

  it("falls back to the raw label rather than blanking the header", () => {
    const broken = week({ start: "not-a-date", end: "also-not" });

    expect(formatWeek(broken)).toBe("2026-W35");
  });

  it("still gives the range when the label carries no week number", () => {
    const odd = week({ label: "this week" });

    expect(formatWeek(odd)).toBe("Aug 24–30");
  });
});
