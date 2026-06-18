import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import ProgressBar from "../../../components/base/ProgressBar.vue";

describe("ProgressBar", () => {
  it("renders with correct width percentage", () => {
    const wrapper = mount(ProgressBar, { props: { spent: 30, allocation: 60 } });
    const fill = wrapper.find("div > div");
    // jsdom: style is applied via CSSOM but not reflected in element.style or getAttribute("style").
    // outerHTML serialization includes the computed style string, so we assert against that.
    expect(fill.element.outerHTML).toContain("width: 50%");
  });

  it("caps at 100% when spent exceeds allocation", () => {
    const wrapper = mount(ProgressBar, { props: { spent: 100, allocation: 50 } });
    const fill = wrapper.find("div > div");
    expect(fill.element.outerHTML).toContain("width: 100%");
  });

  it("shows 0% when allocation is zero", () => {
    const wrapper = mount(ProgressBar, { props: { spent: 10, allocation: 0 } });
    const fill = wrapper.find("div > div");
    expect(fill.element.outerHTML).toContain("width: 0%");
  });

  it("warm variant uses warm gradient", () => {
    const wrapper = mount(ProgressBar, { props: { spent: 10, allocation: 20, variant: "warm" } });
    const fill = wrapper.find("div > div");
    // jsdom serializes hex colors as rgb()
    expect(fill.element.outerHTML).toContain("rgb(245, 158, 11)");
  });
});
