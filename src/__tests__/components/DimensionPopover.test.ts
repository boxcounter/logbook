// src/__tests__/components/DimensionPopover.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import DimensionPopover from "../../components/DimensionPopover.vue";
import { makeDimension, makeCommitment } from "../mocks/fixtures";

const dimensions = [
  makeDimension({ name: "Category", key: "category", source: "static", values: ["Engineering", "PM"], required: true }),
  makeDimension({ name: "Goal", key: "goal", source: "monthly", required: true }),
  makeDimension({ name: "Business Line", key: "business-line", source: "static", values: ["Slax"], required: false }),
];
const commitments = [makeCommitment({ goals: ["Bug fixes", "Code review"] })];

function mountPop(dimValues: Record<string, string> = {}) {
  return mount(DimensionPopover, { props: { dimensions, commitments, dimValues } });
}

describe("DimensionPopover", () => {
  it("lists all dimensions with required/optional meta in dim phase", () => {
    const wrapper = mountPop();
    expect(wrapper.text()).toContain("Category");
    expect(wrapper.text()).toContain("Goal");
    expect(wrapper.text()).toContain("required");
    expect(wrapper.text()).toContain("optional");
  });

  it("shows static dimension values after selecting a dimension", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // Category
    expect(wrapper.text()).toContain("Engineering");
    expect(wrapper.text()).toContain("PM");
  });

  it("shows monthly goal options for a monthly-source dimension", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("[data-test='dim-item']")[1].trigger("click"); // Goal
    expect(wrapper.text()).toContain("Bug fixes");
    expect(wrapper.text()).toContain("Code review");
  });

  it("emits select with [dimKey, value] when a value is chosen", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click");
    await wrapper.findAll("[data-test='val-item']")[0].trigger("click");
    expect(wrapper.emitted("select")?.[0]).toEqual(["category", "Engineering"]);
  });

  it("emits close once all required dimensions are filled after a selection", async () => {
    // category already filled; selecting goal value fills the last required dim
    const wrapper = mountPop({ category: "Engineering" });
    await wrapper.findAll("[data-test='dim-item']")[1].trigger("click"); // Goal
    await wrapper.findAll("[data-test='val-item']")[0].trigger("click"); // Bug fixes
    expect(wrapper.emitted("close")).toBeTruthy();
  });

  it("back button returns from val phase to dim phase", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click");
    await wrapper.find("[data-test='back-btn']").trigger("click");
    expect(wrapper.findAll("[data-test='dim-item']").length).toBe(3);
  });
});
