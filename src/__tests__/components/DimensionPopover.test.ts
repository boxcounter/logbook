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

  it("ArrowDown / Ctrl+N move highlight down with wrap", async () => {
    const wrapper = mountPop(); // active index 0, 3 dims
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(1);

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "n", ctrlKey: true }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(2);

    // wrap 2 -> 0
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(0);
  });

  it("ArrowUp / Ctrl+P move highlight up with wrap", async () => {
    const wrapper = mountPop(); // active index 0
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp" }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(2); // wrap 0 -> 2

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "p", ctrlKey: true }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(1);
  });

  it("prevents default on navigation keys", async () => {
    const wrapper = mountPop();
    const ev = new KeyboardEvent("keydown", { key: "ArrowDown", cancelable: true });
    window.dispatchEvent(ev);
    await wrapper.vm.$nextTick();
    expect(ev.defaultPrevented).toBe(true);
  });

  function activeValIndex(wrapper: ReturnType<typeof mountPop>): number {
    return wrapper.findAll("[data-test='val-item']").findIndex(
      (n) => n.attributes("data-active") === "true"
    );
  }

  it("Enter in dim phase enters the highlighted dimension's value menu", async () => {
    const wrapper = mountPop(); // highlight on Category (index 0)
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));
    await wrapper.vm.$nextTick();
    expect(wrapper.find("[data-test='back-btn']").exists()).toBe(true); // val phase
    expect(wrapper.text()).toContain("Engineering");
  });

  it("val phase highlights the already-selected value", async () => {
    const wrapper = mountPop({ category: "PM" }); // values: Engineering, PM
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // → category val phase
    await wrapper.vm.$nextTick();
    expect(activeValIndex(wrapper)).toBe(1); // "PM"
  });

  it("val phase highlights index 0 when no value selected yet", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click");
    await wrapper.vm.$nextTick();
    expect(activeValIndex(wrapper)).toBe(0);
  });

  it("Enter in val phase emits select for the highlighted value and prevents default", async () => {
    const wrapper = mountPop({ category: "PM" }); // not all required filled (goal missing)
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // category val, active = PM(1)
    const ev = new KeyboardEvent("keydown", { key: "Enter", cancelable: true });
    window.dispatchEvent(ev);
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("select")?.[0]).toEqual(["category", "PM"]);
    expect(ev.defaultPrevented).toBe(true);
  });

  it("returning to dim phase after a select highlights the next unfilled dimension", async () => {
    // category preset; pick a category value → returns to dim (goal still missing)
    const wrapper = mountPop({ category: "PM" });
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // category val
    await wrapper.findAll("[data-test='val-item']")[0].trigger("click"); // pick Engineering → back to dim
    await wrapper.vm.$nextTick();
    // category now filled (just selected) → next unfilled is Goal (index 1)
    expect(activeDimIndex(wrapper)).toBe(1);
  });

  it("Esc back from val to dim re-highlights the first unfilled dimension", async () => {
    const wrapper = mountPop({ category: "Engineering" });
    await wrapper.findAll("[data-test='dim-item']")[1].trigger("click"); // Goal val phase
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(1); // Goal still unfilled
  });

  it("shows the ⌃N/⌃P move hint in the footer", () => {
    const wrapper = mountPop();
    expect(wrapper.text()).toContain("move");
  });

  // ---- highlight style (fill, not ring) ----

  it("highlights the active item with the active background, not a ring", () => {
    const wrapper = mountPop(); // active = index 0 (Category)
    const item = wrapper.findAll("[data-test='dim-item']")[0];
    expect(item.classes()).toContain("bg-[var(--color-popover-item-active-bg)]");
    expect(item.classes()).not.toContain("ring-1");
    expect(item.classes()).not.toContain("hover:bg-[var(--color-divider)]");
  });

  it("shows the selected background on a filled, non-active dimension", async () => {
    // category filled → default active is Goal (index 1); Category (0) is filled & not active
    const wrapper = mountPop({ category: "Engineering" });
    await wrapper.vm.$nextTick(); // let onMounted set activeIndex to firstUnfilled (Goal=1)
    const cat = wrapper.findAll("[data-test='dim-item']")[0];
    expect(cat.classes()).toContain("bg-[var(--color-popover-item-selected-bg)]");
    expect(cat.classes()).not.toContain("bg-[var(--color-popover-item-active-bg)]");
    expect(cat.classes()).toContain("text-[var(--color-brand-solid)]");
  });

  it("stacks active background and brand text when the cursor is on a filled item", async () => {
    const wrapper = mountPop({ category: "Engineering" }); // active = Goal(1)
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp" })); // 1 -> 0 (Category)
    await wrapper.vm.$nextTick();
    const cat = wrapper.findAll("[data-test='dim-item']")[0];
    expect(cat.classes()).toContain("bg-[var(--color-popover-item-active-bg)]");
    expect(cat.classes()).not.toContain("bg-[var(--color-popover-item-selected-bg)]");
    expect(cat.classes()).toContain("text-[var(--color-brand-solid)]");
    expect(cat.classes()).toContain("font-semibold");
  });

  it("val phase: active value uses active bg, the already-selected value uses selected bg", async () => {
    const wrapper = mountPop({ category: "Engineering" }); // category values: Engineering, PM
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // enter Category val; active = Engineering(0)
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" })); // 0 -> 1 (PM)
    await wrapper.vm.$nextTick();
    const vals = wrapper.findAll("[data-test='val-item']");
    expect(vals[1].classes()).toContain("bg-[var(--color-popover-item-active-bg)]"); // PM active
    expect(vals[0].classes()).toContain("bg-[var(--color-popover-item-selected-bg)]"); // Engineering selected, not active
    expect(vals[0].classes()).not.toContain("bg-[var(--color-popover-item-active-bg)]");
  });
});
