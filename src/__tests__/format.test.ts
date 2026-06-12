import { describe, it, expect } from "vitest";
import { formatDuration, parseDurationFromText, stripDurations, resolveDelta } from "../utils/format";

describe("formatDuration", () => {
  it("formats under 60m", () => { expect(formatDuration(45)).toBe("45m"); });
  it("formats hours", () => { expect(formatDuration(120)).toBe("2h"); });
  it("formats h+m", () => { expect(formatDuration(90)).toBe("1h 30m"); });
});

describe("parseDurationFromText", () => {
  it("plain number", () => { expect(parseDurationFromText("90")).toBe(90); });
  it("hours", () => { expect(parseDurationFromText("1.5h")).toBe(90); });
  it("minutes", () => { expect(parseDurationFromText("30m")).toBe(30); });
  it("compound", () => { expect(parseDurationFromText("1h 30m")).toBe(90); });
  it("Chinese text", () => { expect(parseDurationFromText("准备会议（15m），面聊（45m）")).toBe(60); });
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
});
