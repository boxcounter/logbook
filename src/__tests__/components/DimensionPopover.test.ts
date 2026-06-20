// src/__tests__/components/DimensionPopover.test.ts
import { describe, it, expect, afterEach } from "vitest";
import { mount, enableAutoUnmount } from "@vue/test-utils";
import DimensionPopover from "../../components/DimensionPopover.vue";
import { makeDimension, makeCommitment } from "../mocks/fixtures";

// Unmount after each test so the popover's window keydown listener is removed.
enableAutoUnmount(afterEach);

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

  it("Esc in dim phase emits close", async () => {
    const wrapper = mountPop();
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("close")).toBeTruthy();
  });

  it("Esc in val phase returns to dim phase without closing", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // → val phase
    expect(wrapper.find("[data-test='back-btn']").exists()).toBe(true);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await wrapper.vm.$nextTick();
    expect(wrapper.findAll("[data-test='dim-item']").length).toBe(3); // back to dim
    expect(wrapper.emitted("close")).toBeFalsy();
  });

  // ---- keyboard navigation ----

  // Returns the index of the currently highlighted dim-item (-1 if none).
  function activeDimIndex(wrapper: ReturnType<typeof mountPop>): number {
    return wrapper.findAll("[data-test='dim-item']").findIndex(
      (n) => n.attributes("data-active") === "true"
    );
  }

  it("highlights the first unfilled dimension on open", async () => {
    // category filled → first unfilled is Goal (index 1)
    const wrapper = mountPop({ category: "Engineering" });
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(1);
  });

  it("highlights index 0 when no dimension is filled", async () => {
    const wrapper = mountPop();
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(0);
  });

  it("syncs highlight to a dim-item on mouseenter", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("[data-test='dim-item']")[2].trigger("mouseenter");
    expect(activeDimIndex(wrapper)).toBe(2);
  });
});
