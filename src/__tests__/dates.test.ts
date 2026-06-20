import { describe, it, expect } from "vitest";
import { formatDate, datesInMonth, parseDate, addDays } from "../utils/dates";

describe("formatDate", () => {
  it("formats correctly", () => {
    expect(formatDate(new Date(2026, 5, 12))).toBe("2026-06-12");
  });
});

describe("datesInMonth", () => {
  it("returns correct count for 30-day month", () => {
    expect(datesInMonth("2026-06-12").length).toBe(30);
  });
  it("returns correct count for 31-day month", () => {
    expect(datesInMonth("2026-07-15").length).toBe(31);
  });
  it("returns correct count for February", () => {
    expect(datesInMonth("2026-02-10").length).toBe(28);
  });
  it("first date is the 1st", () => {
    expect(datesInMonth("2026-06-15")[0]).toBe("2026-06-01");
  });
  it("last date is the last day of month", () => {
    const dates = datesInMonth("2026-06-15");
    expect(dates[dates.length - 1]).toBe("2026-06-30");
  });
});

describe("parseDate", () => {
  it("parses ISO date", () => {
    expect(parseDate("2026-06-12").getDate()).toBe(12);
  });
});

describe("addDays", () => {
  it("adds a positive offset within a month", () => {
    expect(addDays("2026-06-12", 3)).toBe("2026-06-15");
  });
  it("subtracts across a month boundary", () => {
    expect(addDays("2026-06-01", -1)).toBe("2026-05-31");
  });
  it("adds across a month boundary", () => {
    expect(addDays("2026-06-30", 1)).toBe("2026-07-01");
  });
  it("handles year boundaries", () => {
    expect(addDays("2026-12-31", 1)).toBe("2027-01-01");
  });
  it("handles leap-year February", () => {
    expect(addDays("2028-02-28", 1)).toBe("2028-02-29");
  });
});
