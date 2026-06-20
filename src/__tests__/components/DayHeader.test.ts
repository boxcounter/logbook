// src/__tests__/components/DayHeader.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import DayHeader from "../../components/DayHeader.vue";

describe("DayHeader", () => {
  it("renders title and formatted summary", () => {
    const wrapper = mount(DayHeader, {
      props: { title: "Thursday, June 19", isToday: true, entryCount: 10, totalMinutes: 345 },
    });
    expect(wrapper.text()).toContain("Thursday, June 19");
    expect(wrapper.text()).toContain("10");
    expect(wrapper.text()).toContain("5h 45m");
  });

  it("shows Today badge only when isToday is true", () => {
    const today = mount(DayHeader, { props: { title: "X", isToday: true, entryCount: 0, totalMinutes: 0 } });
    expect(today.find("[data-test='today-badge']").exists()).toBe(true);
    const past = mount(DayHeader, { props: { title: "X", isToday: false, entryCount: 0, totalMinutes: 0 } });
    expect(past.find("[data-test='today-badge']").exists()).toBe(false);
  });

  it("uses singular 'entry' for a count of 1", () => {
    const wrapper = mount(DayHeader, { props: { title: "X", isToday: false, entryCount: 1, totalMinutes: 60 } });
    expect(wrapper.text()).toContain("1 entry");
    expect(wrapper.text()).not.toContain("1 entries");
  });

  it("emits prev-day when the left arrow is clicked", async () => {
    const wrapper = mount(DayHeader, {
      props: { title: "X", isToday: false, entryCount: 0, totalMinutes: 0, canGoNext: true },
    });
    await wrapper.find("[data-test='prev-day']").trigger("click");
    expect(wrapper.emitted("prev-day")).toBeTruthy();
  });

  it("emits next-day when the right arrow is clicked and canGoNext is true", async () => {
    const wrapper = mount(DayHeader, {
      props: { title: "X", isToday: false, entryCount: 0, totalMinutes: 0, canGoNext: true },
    });
    await wrapper.find("[data-test='next-day']").trigger("click");
    expect(wrapper.emitted("next-day")).toBeTruthy();
  });

  it("does not emit next-day when canGoNext is false", async () => {
    const wrapper = mount(DayHeader, {
      props: { title: "X", isToday: true, entryCount: 0, totalMinutes: 0, canGoNext: false },
    });
    await wrapper.find("[data-test='next-day']").trigger("click");
    expect(wrapper.emitted("next-day")).toBeFalsy();
  });
});
