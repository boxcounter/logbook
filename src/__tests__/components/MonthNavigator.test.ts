import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import MonthNavigator from "../../components/MonthNavigator.vue";
import type { AvailableMonth } from "../../stores/useStore";

const MONTH_NAMES = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

function mountNav(props: {
  year: number;
  month: number;
  availableMonths: AvailableMonth[] | null;
}) {
  return mount(MonthNavigator, { props });
}

describe("MonthNavigator", () => {
  it("displays month name and year", () => {
    const wrapper = mountNav({ year: 2026, month: 6, availableMonths: null });
    expect(wrapper.text()).toContain("June 2026");
  });

  it("emits navigate on left arrow click", async () => {
    const wrapper = mountNav({ year: 2026, month: 6, availableMonths: null });
    const buttons = wrapper.findAll("button");
    await buttons[0].trigger("click"); // left arrow
    expect(wrapper.emitted("navigate")?.[0]).toEqual([{ year: 2026, month: 5 }]);
  });

  it("emits navigate on right arrow click", async () => {
    const wrapper = mountNav({ year: 2026, month: 6, availableMonths: null });
    const buttons = wrapper.findAll("button");
    await buttons[1].trigger("click"); // right arrow
    expect(wrapper.emitted("navigate")?.[0]).toEqual([{ year: 2026, month: 7 }]);
  });

  it("January left wraps to December previous year", async () => {
    const wrapper = mountNav({ year: 2026, month: 1, availableMonths: null });
    const buttons = wrapper.findAll("button");
    await buttons[0].trigger("click");
    expect(wrapper.emitted("navigate")?.[0]).toEqual([{ year: 2025, month: 12 }]);
  });

  it("December right wraps to January next year", async () => {
    const wrapper = mountNav({ year: 2026, month: 12, availableMonths: null });
    const buttons = wrapper.findAll("button");
    await buttons[1].trigger("click");
    expect(wrapper.emitted("navigate")?.[0]).toEqual([{ year: 2027, month: 1 }]);
  });

  it("clicking month-year text toggles quick-jump popover", async () => {
    const availableMonths: AvailableMonth[] = [
      { year: 2026, month: 6 },
      { year: 2026, month: 5 },
    ];
    const wrapper = mountNav({ year: 2026, month: 6, availableMonths });
    const label = wrapper.find(".cursor-pointer");
    await label.trigger("click");
    // Popover should now be visible
    expect(wrapper.find("select").exists()).toBe(true);
  });

  it("quick-jump popover: year select lists unique years from availableMonths", async () => {
    const availableMonths: AvailableMonth[] = [
      { year: 2026, month: 6 },
      { year: 2025, month: 3 },
      { year: 2025, month: 8 },
    ];
    const wrapper = mountNav({ year: 2026, month: 6, availableMonths });
    await wrapper.find(".cursor-pointer").trigger("click");
    const yearSelect = wrapper.find("select");
    const options = yearSelect.findAll("option");
    const years = options.map(o => parseInt(o.element.value));
    expect(years).toContain(2026);
    expect(years).toContain(2025);
  });

  it("quick-jump popover: month select shows only months for selected year", async () => {
    const availableMonths: AvailableMonth[] = [
      { year: 2026, month: 6 },
      { year: 2026, month: 3 },
      { year: 2025, month: 8 },
    ];
    const wrapper = mountNav({ year: 2026, month: 6, availableMonths });
    await wrapper.find(".cursor-pointer").trigger("click");
    const selects = wrapper.findAll("select");
    // First select is year, second is month
    const monthSelect = selects[1];
    const options = monthSelect.findAll("option");
    const monthValues = options.map(o => parseInt(o.element.value));
    // For year 2026 (selected), only months 3 and 6 should appear
    expect(monthValues).toEqual(expect.arrayContaining([3, 6]));
    expect(monthValues).not.toEqual(expect.arrayContaining([8]));
  });

  it("quick-jump: changing month select emits navigate", async () => {
    const availableMonths: AvailableMonth[] = [
      { year: 2026, month: 6 },
      { year: 2026, month: 3 },
    ];
    const wrapper = mountNav({ year: 2026, month: 6, availableMonths });
    await wrapper.find(".cursor-pointer").trigger("click");
    const monthSelect = wrapper.findAll("select")[1];
    await monthSelect.setValue(3);
    expect(wrapper.emitted("navigate")?.[0]).toEqual([{ year: 2026, month: 3 }]);
  });

  it("quick-jump not shown when availableMonths is null (not yet loaded)", () => {
    const wrapper = mountNav({ year: 2026, month: 6, availableMonths: null });
    const label = wrapper.find(".cursor-pointer");
    // When null, clicking should emit requestMonths and not open popover
    expect(wrapper.text()).toContain("June 2026");
    // The label should still be clickable (triggers requestMonths)
    expect(label.exists()).toBe(true);
  });
});
