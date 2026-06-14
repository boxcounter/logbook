import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mount } from "@vue/test-utils";
import DayStrip from "../../components/DayStrip.vue";

describe("DayStrip", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    // Set "today" to 2026-06-14
    vi.setSystemTime(new Date(2026, 5, 14));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  function mountStrip(props: {
    dates: string[];
    selectedDate: string;
    monthEntries: Record<string, unknown[]>;
  }) {
    return mount(DayStrip, { props });
  }

  it("renders correct number of day cells", () => {
    const dates = Array.from({ length: 30 }, (_, i) => `2026-06-${String(i + 1).padStart(2, "0")}`);
    const wrapper = mountStrip({ dates, selectedDate: "2026-06-14", monthEntries: {} });
    const cells = wrapper.findAll("[data-day]");
    expect(cells).toHaveLength(30);
  });

  it("selected date has highlight class", () => {
    const dates = ["2026-06-01", "2026-06-02", "2026-06-03"];
    const wrapper = mountStrip({ dates, selectedDate: "2026-06-02", monthEntries: {} });
    const cells = wrapper.findAll("[data-day]");
    const selected = cells.find(c => c.attributes("data-day") === "2026-06-02");
    expect(selected?.classes()).toContain("bg-blue-600");
  });

  it("today has distinct indicator when not selected", () => {
    const dates = ["2026-06-13", "2026-06-14", "2026-06-15"];
    const wrapper = mountStrip({ dates, selectedDate: "2026-06-13", monthEntries: {} });
    const todayCell = wrapper.find('[data-day="2026-06-14"]');
    // Should have underline or bold class but not the selected blue bg
    expect(todayCell.classes()).not.toContain("bg-blue-600");
    // Should have a "today" class or similar marker
    expect(todayCell.classes()).toContain("font-semibold");
  });

  it("future dates are grey and not clickable", () => {
    const dates = ["2026-06-13", "2026-06-14", "2026-06-15", "2026-06-16"];
    const wrapper = mountStrip({ dates, selectedDate: "2026-06-13", monthEntries: {} });
    const cell15 = wrapper.find('[data-day="2026-06-15"]');
    const cell16 = wrapper.find('[data-day="2026-06-16"]');
    expect(cell15.classes()).toContain("text-gray-300");
    expect(cell16.classes()).toContain("text-gray-300");
  });

  it("future dates emit no event on click", async () => {
    const dates = ["2026-06-14", "2026-06-15"];
    const wrapper = mountStrip({ dates, selectedDate: "2026-06-14", monthEntries: {} });
    const cell15 = wrapper.find('[data-day="2026-06-15"]');
    await cell15.trigger("click");
    expect(wrapper.emitted("selectDay")).toBeFalsy();
  });

  it("clicking a past date emits selectDay", async () => {
    const dates = ["2026-06-10", "2026-06-11"];
    const wrapper = mountStrip({ dates, selectedDate: "2026-06-10", monthEntries: {} });
    const cell = wrapper.find('[data-day="2026-06-11"]');
    await cell.trigger("click");
    expect(wrapper.emitted("selectDay")?.[0]).toEqual(["2026-06-11"]);
  });

  it("days with entries show a blue dot", () => {
    const dates = ["2026-06-01", "2026-06-02"];
    const monthEntries = { "2026-06-01": [{ id: "e1", item: "X", duration: 30, dimensions: {} }] };
    const wrapper = mountStrip({ dates, selectedDate: "2026-06-01", monthEntries });
    const cellWithEntry = wrapper.find('[data-day="2026-06-01"]');
    const cellWithoutEntry = wrapper.find('[data-day="2026-06-02"]');
    // Cell with entry should contain a dot element
    expect(cellWithEntry.find("[data-dot]").exists()).toBe(true);
    // Cell without entry should not
    expect(cellWithoutEntry.find("[data-dot]").exists()).toBe(false);
  });

  it("every 7th cell has wider right margin", () => {
    const dates = Array.from({ length: 14 }, (_, i) => `2026-06-${String(i + 1).padStart(2, "0")}`);
    const wrapper = mountStrip({ dates, selectedDate: "2026-06-01", monthEntries: {} });
    const cells = wrapper.findAll("[data-day]");
    // 7th cell (index 6) should have extra margin class
    expect(cells[6].classes()).toContain("mr-2");
    // 14th cell (index 13) should have extra margin class
    expect(cells[13].classes()).toContain("mr-2");
    // 8th cell (index 7) should NOT have it
    expect(cells[7].classes()).not.toContain("mr-2");
  });
});
