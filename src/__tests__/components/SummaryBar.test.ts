import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import SummaryBar from "../../components/SummaryBar.vue";
import type { Granularity } from "../../types";
import { makeEntry } from "../mocks/fixtures";

function mountBar(entries: ReturnType<typeof makeEntry>[], granularity: Granularity, periodEntries?: Record<string, ReturnType<typeof makeEntry>[]>) {
  return mount(SummaryBar, {
    props: { entries, granularity, periodEntries },
  });
}

// ============================================================

describe("SummaryBar", () => {
  it("renders nothing when no entries", () => {
    const wrapper = mountBar([], "day" as Granularity);
    expect(wrapper.find("div").exists()).toBe(false);
  });

  it("day mode: shows entry count and total minutes", () => {
    const entries = [makeEntry({ duration: 30 }), makeEntry({ duration: 45 }), makeEntry({ duration: 25 })];
    const wrapper = mountBar(entries, "day");
    const text = wrapper.text();
    expect(text).toContain("3 entries");
    expect(text).toContain("1h 40m");
  });

  it('day mode: singular "1 entry"', () => {
    const wrapper = mountBar([makeEntry({ duration: 15 })], "day");
    expect(wrapper.text()).toContain("1 entry");
  });

  it("week mode: shows per-day subtotals", () => {
    const e1 = makeEntry({ duration: 30 });
    const e2 = makeEntry({ duration: 45 });
    const entries: Record<string, ReturnType<typeof makeEntry>[]> = {
      "2026-06-08": [e1],       // Mon
      "2026-06-09": [e2],       // Tue
    };
    const wrapper = mountBar(entries["2026-06-09"], "week", entries);
    const text = wrapper.text();
    expect(text).toContain("Week total");
    // 30 + 45 = 75 minutes
    expect(text).toContain("1h 15m");
  });

  it("week mode: shows week total with separator", () => {
    const entries: Record<string, ReturnType<typeof makeEntry>[]> = {
      "2026-06-08": [makeEntry({ duration: 60 })],
    };
    const wrapper = mountBar([], "week", entries);
    expect(wrapper.text()).toContain("Week total");
    expect(wrapper.text()).toContain("1h");
  });

  it("month mode: groups by week and shows month total", () => {
    const entries: Record<string, ReturnType<typeof makeEntry>[]> = {
      "2026-06-01": [makeEntry({ duration: 60 })],
      "2026-06-15": [makeEntry({ duration: 30 })],
    };
    const wrapper = mountBar([], "month", entries);
    const text = wrapper.text();
    expect(text).toContain("Month total");
    expect(text).toContain("1h 30m");
  });

  it("returns 0 when periodEntries is missing for non-day granularity", () => {
    const wrapper = mountBar([makeEntry({ duration: 30 })], "week", undefined);
    expect(wrapper.find("div").exists()).toBe(false);
  });
});
