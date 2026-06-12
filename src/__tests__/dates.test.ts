import { describe, it, expect } from "vitest";
import { formatDate, datesInPeriod, parseDate } from "../utils/dates";

describe("formatDate", () => {
  it("formats correctly", () => {
    expect(formatDate(new Date(2026, 5, 12))).toBe("2026-06-12");
  });
});

describe("datesInPeriod", () => {
  it("day returns single date", () => {
    expect(datesInPeriod("2026-06-12", "day")).toEqual(["2026-06-12"]);
  });
  it("week returns 7 dates", () => {
    expect(datesInPeriod("2026-06-12", "week").length).toBe(7);
  });
  it("month returns correct count", () => {
    expect(datesInPeriod("2026-06-12", "month").length).toBe(30);
  });
});

describe("parseDate", () => {
  it("parses ISO date", () => {
    expect(parseDate("2026-06-12").getDate()).toBe(12);
  });
});
