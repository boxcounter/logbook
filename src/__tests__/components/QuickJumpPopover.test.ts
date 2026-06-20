// src/__tests__/components/QuickJumpPopover.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import QuickJumpPopover from "../../components/QuickJumpPopover.vue";
import type { AvailableMonth } from "../../stores/useStore";

const months: AvailableMonth[] = [
  { year: 2026, month: 6 },
  { year: 2026, month: 3 },
  { year: 2025, month: 8 },
];

function mountPop() {
  return mount(QuickJumpPopover, { props: { year: 2026, month: 6, availableMonths: months } });
}

describe("QuickJumpPopover", () => {
  it("year select lists unique years, descending", () => {
    const wrapper = mountPop();
    const years = wrapper.findAll("select")[0].findAll("option").map(o => parseInt(o.element.value, 10));
    expect(years).toEqual([2026, 2025]);
  });

  it("month select shows only months for the selected year", () => {
    const wrapper = mountPop();
    const monthVals = wrapper.findAll("select")[1].findAll("option").map(o => parseInt(o.element.value, 10));
    expect(monthVals).toEqual(expect.arrayContaining([3, 6]));
    expect(monthVals).not.toContain(8);
  });

  it("changing the month select emits jump", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("select")[1].setValue(3);
    expect(wrapper.emitted("jump")?.[0]).toEqual([{ year: 2026, month: 3 }]);
  });

  it("changing the year then month emits jump with the new year", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("select")[0].setValue(2025);
    await wrapper.findAll("select")[1].setValue(8);
    expect(wrapper.emitted("jump")?.[0]).toEqual([{ year: 2025, month: 8 }]);
  });

  it("Esc emits close", async () => {
    const wrapper = mountPop();
    await wrapper.trigger("keydown", { key: "Escape" });
    expect(wrapper.emitted("close")).toBeTruthy();
  });

  it("focuses its root on mount so Esc works without clicking a select first", () => {
    const wrapper = mount(QuickJumpPopover, {
      props: { year: 2026, month: 6, availableMonths: months },
      attachTo: document.body,
    });
    expect(document.activeElement).toBe(wrapper.element);
    wrapper.unmount();
  });
});
