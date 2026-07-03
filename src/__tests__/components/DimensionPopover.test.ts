// src/__tests__/components/DimensionPopover.test.ts
import { describe, it, expect, afterEach } from "vitest";
import { mount, enableAutoUnmount } from "@vue/test-utils";
import DimensionPopover from "../../components/DimensionPopover.vue";
import { makeDimension, makeCommitment } from "../mocks/fixtures";

// Unmount after each test so the popover's window keydown listener is removed.
enableAutoUnmount(afterEach);

const dimensions = [
  makeDimension({ name: "Category", key: "category", source: "static", values: ["Engineering", "PM"], required: true }),
  makeDimension({ name: "Goal", key: "goal", source: "commitments:goals", required: true }),
  makeDimension({ name: "Business Line", key: "business-line", source: "static", values: ["Slax"], required: false }),
];
const commitments = [makeCommitment({ goals: ["Bug fixes", "Code review"] })];

function mountPop(dimValues: Record<string, string> = {}) {
  return mount(DimensionPopover, { props: { dimensions, commitments, dimValues } });
}

describe("DimensionPopover", () => {
  it("lists all dimensions with required/optional meta in dim stage", () => {
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

  it("shows commitments:goals goal options for a commitments:goals source dimension", async () => {
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

  it("back button returns from val stage to dim stage", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click");
    await wrapper.find("[data-test='back-btn']").trigger("click");
    expect(wrapper.findAll("[data-test='dim-item']").length).toBe(3);
  });

  it("Esc in dim stage emits close", async () => {
    const wrapper = mountPop();
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("close")).toBeTruthy();
  });

  it("Esc in val stage returns to dim stage without closing", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // → val stage
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
    const wrapper = mountPop(); // active index 0, 3 dims + role = 4 items
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(1);

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "n", ctrlKey: true }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(2);

    // move to role (index 3)
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(-1); // role is not [data-test='dim-item']

    // wrap 3 -> 0
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(0);
  });

  it("ArrowUp / Ctrl+P move highlight up with wrap", async () => {
    const wrapper = mountPop(); // active index 0, 3 dims + role = 4 items
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp" }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(-1); // wrap 0 -> 3 (role)

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "p", ctrlKey: true }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(2);
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

  it("Enter in dim stage enters the highlighted dimension's value menu", async () => {
    const wrapper = mountPop(); // highlight on Category (index 0)
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));
    await wrapper.vm.$nextTick();
    expect(wrapper.find("[data-test='back-btn']").exists()).toBe(true); // val stage
    expect(wrapper.text()).toContain("Engineering");
  });

  it("val stage highlights the already-selected value", async () => {
    const wrapper = mountPop({ category: "PM" }); // values: Engineering, PM
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // → category val stage
    await wrapper.vm.$nextTick();
    expect(activeValIndex(wrapper)).toBe(1); // "PM"
  });

  it("val stage highlights index 0 when no value selected yet", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click");
    await wrapper.vm.$nextTick();
    expect(activeValIndex(wrapper)).toBe(0);
  });

  it("Enter in val stage emits select for the highlighted value and prevents default", async () => {
    const wrapper = mountPop({ category: "PM" }); // not all required filled (goal missing)
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // category val, active = PM(1)
    const ev = new KeyboardEvent("keydown", { key: "Enter", cancelable: true });
    window.dispatchEvent(ev);
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("select")?.[0]).toEqual(["category", "PM"]);
    expect(ev.defaultPrevented).toBe(true);
  });

  it("returning to dim stage after a select highlights the next unfilled dimension", async () => {
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
    await wrapper.findAll("[data-test='dim-item']")[1].trigger("click"); // Goal val stage
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(1); // Goal still unfilled
  });

  it("shows the ⌃N/⌃P move hint in the footer", () => {
    const wrapper = mountPop();
    expect(wrapper.text()).toContain("move");
  });

  // ---- highlight style (fill, not ring) ----

  it("highlights the active item with a solid brand fill and white text", () => {
    const wrapper = mountPop(); // active = index 0 (Category)
    const item = wrapper.findAll("[data-test='dim-item']")[0];
    expect(item.classes()).toContain("bg-[var(--color-brand-solid)]");
    expect(item.classes()).toContain("text-white");
    expect(item.classes()).not.toContain("ring-1");
  });

  it("a filled, non-active dimension uses brand text + ✓ with no background fill", async () => {
    // category filled → default active is Goal (index 1); Category (0) is filled & not active
    const wrapper = mountPop({ category: "Engineering" });
    await wrapper.vm.$nextTick();
    const cat = wrapper.findAll("[data-test='dim-item']")[0];
    expect(cat.classes()).toContain("text-[var(--color-brand-solid)]");
    expect(cat.classes()).toContain("font-semibold");
    expect(cat.classes()).not.toContain("bg-[var(--color-brand-solid)]");
    expect(cat.text()).toContain("Engineering"); // value shown on the right
    expect(cat.text()).toContain("✓");
  });

  it("cursor on a filled item shows the solid fill and white text (cursor wins over selected)", async () => {
    const wrapper = mountPop({ category: "Engineering" }); // active = Goal(1)
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp" })); // 1 -> 0 (Category)
    await wrapper.vm.$nextTick();
    const cat = wrapper.findAll("[data-test='dim-item']")[0];
    expect(cat.classes()).toContain("bg-[var(--color-brand-solid)]");
    expect(cat.classes()).toContain("text-white");
    expect(cat.classes()).not.toContain("text-[var(--color-brand-solid)]");
  });

  it("val stage: active value uses the solid fill; the selected value uses brand text + ✓", async () => {
    const wrapper = mountPop({ category: "Engineering" }); // category values: Engineering, PM
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // enter Category val; active = Engineering(0)
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" })); // 0 -> 1 (PM)
    await wrapper.vm.$nextTick();
    const vals = wrapper.findAll("[data-test='val-item']");
    expect(vals[1].classes()).toContain("bg-[var(--color-brand-solid)]"); // PM active
    expect(vals[1].classes()).toContain("text-white");
    expect(vals[0].classes()).toContain("text-[var(--color-brand-solid)]"); // Engineering selected, not active
    expect(vals[0].classes()).not.toContain("bg-[var(--color-brand-solid)]");
    expect(vals[0].text()).toContain("✓");
  });

  it("excludes deleted dimensions from the dim list", () => {
    const dims = [
      makeDimension({ name: "Visible", key: "visible", source: "static", values: ["v"], required: true }),
      makeDimension({ name: "Deleted", key: "deleted", source: "static", values: ["d"], required: true, deleted: true }),
    ];
    const wrapper = mount(DimensionPopover, { props: { dimensions: dims, commitments: [], dimValues: {} } });
    const items = wrapper.findAll("[data-test='dim-item']");
    expect(items.length).toBe(1);
    expect(items[0].text()).toContain("Visible");
    expect(wrapper.text()).not.toContain("Deleted");
  });

  // ---- role dimension support ----

  it("shows role option in dim stage when commitments are present", () => {
    const wrapper = mountPop();
    const roleItem = wrapper.find("[data-test='dim-role']");
    expect(roleItem.exists()).toBe(true);
    expect(roleItem.text()).toContain("Role");
  });

  it("does not show role option when commitments are empty", () => {
    const wrapper = mount(DimensionPopover, {
      props: { dimensions, commitments: [], dimValues: {} },
    });
    expect(wrapper.find("[data-test='dim-role']").exists()).toBe(false);
  });

  it("selecting role shows role names from commitments in val stage", async () => {
    const roleCommitments = [
      makeCommitment({ role: "Dev", goals: ["Bug fixes"] }),
      makeCommitment({ role: "PM", goals: ["Planning"] }),
    ];
    const wrapper = mount(DimensionPopover, {
      props: { dimensions, commitments: roleCommitments, dimValues: {} },
    });
    await wrapper.find("[data-test='dim-role']").trigger("click");
    const vals = wrapper.findAll("[data-test='val-item']");
    expect(vals.length).toBe(2);
    expect(vals[0].text()).toContain("Dev");
    expect(vals[1].text()).toContain("PM");
  });

  it("selecting role emits select with key 'role' and the chosen value", async () => {
    const roleCommitments = [
      makeCommitment({ role: "Dev", goals: ["Bug fixes"] }),
    ];
    const wrapper = mount(DimensionPopover, {
      props: { dimensions, commitments: roleCommitments, dimValues: {} },
    });
    await wrapper.find("[data-test='dim-role']").trigger("click");
    await wrapper.findAll("[data-test='val-item']")[0].trigger("click");
    expect(wrapper.emitted("select")?.[0]).toEqual(["role", "Dev"]);
  });

  it("shows val header as 'Role' when role dimension is selected", async () => {
    const wrapper = mountPop();
    await wrapper.find("[data-test='dim-role']").trigger("click");
    // val stage header should contain "Role"
    expect(wrapper.text()).toContain("Role");
  });

  it("cross-filters goals to selected role's goals", async () => {
    const roleCommitments = [
      makeCommitment({ role: "Dev", goals: ["Bug fixes", "Code review"] }),
      makeCommitment({ role: "PM", goals: ["Planning", "Design"] }),
    ];
    const wrapper = mount(DimensionPopover, {
      props: { dimensions, commitments: roleCommitments, dimValues: { role: "Dev" } },
    });
    // Select Goal dimension (index 1)
    await wrapper.findAll("[data-test='dim-item']")[1].trigger("click");
    const vals = wrapper.findAll("[data-test='val-item']");
    expect(vals.length).toBe(2);
    expect(vals[0].text()).toContain("Bug fixes");
    expect(vals[1].text()).toContain("Code review");
    expect(wrapper.text()).not.toContain("Planning");
    expect(wrapper.text()).not.toContain("Design");
  });

  it("cross-filters roles to roles containing the selected goal", async () => {
    const roleCommitments = [
      makeCommitment({ role: "Dev", goals: ["Bug fixes", "Code review"] }),
      makeCommitment({ role: "PM", goals: ["Planning"] }),
    ];
    const wrapper = mount(DimensionPopover, {
      props: { dimensions, commitments: roleCommitments, dimValues: { goal: "Bug fixes" } },
    });
    // Select Role
    await wrapper.find("[data-test='dim-role']").trigger("click");
    const vals = wrapper.findAll("[data-test='val-item']");
    expect(vals.length).toBe(1);
    expect(vals[0].text()).toContain("Dev");
    expect(wrapper.text()).not.toContain("PM");
  });

  it("Enter key selects role when highlighted in dim stage", async () => {
    const wrapper = mountPop(); // 3 dims + role = 4 items, active index 0
    // Move highlight to role (index 3)
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp" })); // wrap 0 -> 3
    await wrapper.vm.$nextTick();
    // Press Enter
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));
    await wrapper.vm.$nextTick();
    // Should be in val stage showing role values
    expect(wrapper.find("[data-test='back-btn']").exists()).toBe(true);
    expect(wrapper.text()).toContain("Role");
  });

  it("role item shows selected role value with checkmark", async () => {
    const wrapper = mountPop({ role: "Dev" });
    const roleItem = wrapper.find("[data-test='dim-role']");
    expect(roleItem.text()).toContain("Dev");
    expect(roleItem.text()).toContain("✓");
  });

  it("role item shows 'optional' badge when no role selected", () => {
    const wrapper = mountPop();
    const roleItem = wrapper.find("[data-test='dim-role']");
    expect(roleItem.text()).toContain("optional");
  });
});
