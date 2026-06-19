// src/__tests__/components/HeatmapCalendar.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import HeatmapCalendar from "../../components/HeatmapCalendar.vue";
import { makeEntry } from "../mocks/fixtures";
import type { Entry } from "../../types";

function mountCal(monthEntries: Record<string, Entry[]> = {}, availableMonths: { year: number; month: number }[] | null = null) {
  return mount(HeatmapCalendar, {
    props: { year: 2026, month: 6, selectedDate: "2026-06-19", monthEntries, availableMonths },
  });
}

describe("HeatmapCalendar", () => {
  it("renders the month label", () => {
    expect(mountCal().text()).toContain("June");
    expect(mountCal().text()).toContain("2026");
  });

  it("renders a cell for each day of June (30 days)", () => {
    const cells = mountCal().findAll("[data-test='day-cell']");
    expect(cells.length).toBe(30);
  });

  it("emits navigate on the left and right arrows", async () => {
    const wrapper = mountCal();
    await wrapper.find("[data-test='prev-month']").trigger("click");
    expect(wrapper.emitted("navigate")?.[0]).toEqual([{ year: 2026, month: 5 }]);
    await wrapper.find("[data-test='next-month']").trigger("click");
    expect(wrapper.emitted("navigate")?.[1]).toEqual([{ year: 2026, month: 7 }]);
  });

  it("emits selectDay when a non-future day is clicked", async () => {
    const wrapper = mountCal();
    // Day 1 of June 2026 is in the past relative to selectedDate's month — click it
    const day1 = wrapper.findAll("[data-test='day-cell']")[0];
    await day1.trigger("click");
    expect(wrapper.emitted("selectDay")?.[0]).toEqual(["2026-06-01"]);
  });

  it("shows the month total of logged hours", () => {
    const monthEntries = {
      "2026-06-02": [makeEntry({ duration: 120 })],
      "2026-06-03": [makeEntry({ duration: 90 }), makeEntry({ duration: 30 })],
    };
    expect(mountCal(monthEntries).text()).toContain("4");      // 4h total
  });

  it("emits requestMonths when the label is clicked and months are not loaded", async () => {
    const wrapper = mountCal({}, null);
    await wrapper.find("[data-test='month-label']").trigger("click");
    expect(wrapper.emitted("requestMonths")).toBeTruthy();
  });

  it("shows QuickJumpPopover when the label is clicked and months are loaded", async () => {
    const wrapper = mountCal({}, [{ year: 2026, month: 6 }]);
    await wrapper.find("[data-test='month-label']").trigger("click");
    expect(wrapper.findComponent({ name: "QuickJumpPopover" }).exists()).toBe(true);
  });
});
