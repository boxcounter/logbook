import { describe, it, expect } from "vitest";
import { formatDuration, formatDurationCompact, parseDurationFromText, stripDurations, resolveDelta } from "../utils/format";

describe("formatDuration", () => {
  it("formats under 60m", () => { expect(formatDuration(45)).toBe("45m"); });
  it("formats hours", () => { expect(formatDuration(120)).toBe("2h"); });
  it("formats h+m", () => { expect(formatDuration(90)).toBe("1h 30m"); });
});

describe("parseDurationFromText", () => {
  it("plain number returns null (no unit)", () => { expect(parseDurationFromText("90")).toBeNull(); });
  it("plain number 520 returns null", () => { expect(parseDurationFromText("520")).toBeNull(); });
  it("hours", () => { expect(parseDurationFromText("1.5h")).toBe(90); });
  it("minutes", () => { expect(parseDurationFromText("30m")).toBe(30); });
  it("compound", () => { expect(parseDurationFromText("1h 30m")).toBe(90); });
  it("compound with h", () => { expect(parseDurationFromText("2h 15m")).toBe(135); });
  it("Chinese text", () => { expect(parseDurationFromText("准备会议（15m），面聊（45m）")).toBe(60); });
  it("text with plain number not extracted", () => { expect(parseDurationFromText("Code review 45")).toBeNull(); });
  it("empty", () => { expect(parseDurationFromText("")).toBeNull(); });
  it("no duration", () => { expect(parseDurationFromText("nothing")).toBeNull(); });
});

describe("stripDurations", () => {
  it("removes duration", () => {
    expect(stripDurations("Sprint planning 1.5h")).toBe("Sprint planning");
  });
  it("cleans brackets", () => {
    expect(stripDurations("Meeting (30m)")).toBe("Meeting");
  });
});

describe("resolveDelta", () => {
  it("adds", () => { expect(resolveDelta("+30", 60)).toBe(90); });
  it("subtracts", () => { expect(resolveDelta("-15", 60)).toBe(45); });
  it("absolute", () => { expect(resolveDelta("90", 60)).toBe(90); });
  it("clamps zero", () => { expect(resolveDelta("-100", 60)).toBe(0); });

  // Arithmetic expressions (addition / subtraction only)
  it("evaluates addition expression", () => { expect(resolveDelta("5+60", 0)).toBe(65); });
  it("evaluates subtraction expression", () => { expect(resolveDelta("100-30", 0)).toBe(70); });
  it("evaluates expression ignoring current", () => { expect(resolveDelta("5+60", 99)).toBe(65); });

  // Expressions with duration unit suffixes
  it("expr: 52+15m", () => { expect(resolveDelta("52+15m", 0)).toBe(67); });
  it("expr: 52+1h", () => { expect(resolveDelta("52+1h", 0)).toBe(112); });
  it("expr: 52+1.5h", () => { expect(resolveDelta("52+1.5h", 0)).toBe(142); });
  it("expr: 1h+30m", () => { expect(resolveDelta("1h+30m", 0)).toBe(90); });
  it("expr: 52+1h+15m", () => { expect(resolveDelta("52+1h+15m", 0)).toBe(127); });

  // Delta with expression: +N still means add-to-current
  it("delta add still works", () => { expect(resolveDelta("+30", 60)).toBe(90); });
  it("delta subtract still works", () => { expect(resolveDelta("-15", 60)).toBe(45); });

  // Delta with unit suffix
  it("delta +1h adds 60 min", () => { expect(resolveDelta("+1h", 20)).toBe(80); });
  it("delta +1.5h adds 90 min", () => { expect(resolveDelta("+1.5h", 20)).toBe(110); });
  it("delta -1h subtracts 60 min", () => { expect(resolveDelta("-1h", 90)).toBe(30); });
  it("delta +0.5h adds 30 min", () => { expect(resolveDelta("+0.5h", 30)).toBe(60); });
  it("delta +60m adds 60 min", () => { expect(resolveDelta("+60m", 20)).toBe(80); });
  it("delta -30m subtracts 30 min", () => { expect(resolveDelta("-30m", 60)).toBe(30); });
  it("delta with h suffix clamps zero", () => { expect(resolveDelta("-2h", 60)).toBe(0); });

  // Edge cases
  it("falls back to current on invalid expression", () => { expect(resolveDelta("abc", 60)).toBe(60); });
  it("clamps negative expression result to zero", () => { expect(resolveDelta("5-100", 0)).toBe(0); });
  it("handles whitespace in expression", () => { expect(resolveDelta(" 5 + 60 ", 0)).toBe(65); });
});

describe("formatDurationCompact", () => {
  it("zero", () => { expect(formatDurationCompact(0)).toBe("0"); });
  it("30m → 0.5h", () => { expect(formatDurationCompact(30)).toBe("0.5h"); });
  it("45m → 0.8h (rounded)", () => { expect(formatDurationCompact(45)).toBe("0.8h"); });
  it("60m → 1h", () => { expect(formatDurationCompact(60)).toBe("1h"); });
  it("90m → 1.5h", () => { expect(formatDurationCompact(90)).toBe("1.5h"); });
  it("120m → 2h", () => { expect(formatDurationCompact(120)).toBe("2h"); });
  it("150m → 2.5h", () => { expect(formatDurationCompact(150)).toBe("2.5h"); });
  it("5m → 0.1h", () => { expect(formatDurationCompact(5)).toBe("0.1h"); });
  it("865m → 14.4h", () => { expect(formatDurationCompact(865)).toBe("14.4h"); });
  it("870m → 14.5h", () => { expect(formatDurationCompact(870)).toBe("14.5h"); });
  it("1m rounds to 0h", () => { expect(formatDurationCompact(1)).toBe("0h"); });
  it("3m rounds to 0.1h", () => { expect(formatDurationCompact(3)).toBe("0.1h"); });
});
