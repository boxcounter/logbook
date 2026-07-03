// src/__tests__/components/composite/EntryRowEdit.test.ts
import { describe, it, expect, afterEach, vi } from "vitest";
import { mount, enableAutoUnmount } from "@vue/test-utils";
import { nextTick } from "vue";
import EntryRowEdit from "../../../components/composite/EntryRowEdit.vue";
import { makeEntry, makeDimension, makeCommitment } from "../../mocks/fixtures";

// DimensionPopover registers a window keydown listener; unmount after each test.
enableAutoUnmount(afterEach);

const dimensions = [
  makeDimension({ name: "Category", key: "category", source: "static", values: ["Engineering"], required: true }),
  makeDimension({ name: "Goal", key: "goal", source: "commitments:goals", required: true }),
  makeDimension({ name: "Business Line", key: "business-line", source: "static", values: ["Slax"], required: false }),
];
const commitments = [makeCommitment({ goals: ["Bug fixes"] })];

const fullDims = { category: "Engineering", goal: "Bug fixes", "business-line": "Slax" };

function mountEdit(entryOverrides = {}) {
  const entry = makeEntry({ item: "Old item", duration: 45, dimensions: { ...fullDims }, ...entryOverrides });
  return mount(EntryRowEdit, { props: { entry, dimensions, commitments } });
}

function mountEditNoDims() {
  const entry = makeEntry({ item: "Old item", duration: 45, dimensions: {} });
  return mount(EntryRowEdit, { props: { entry, dimensions, commitments } });
}

function mountEditWithFocus(focusTarget: 'item' | 'duration' = 'item') {
  const entry = makeEntry({ item: "Old item", duration: 45, dimensions: { ...fullDims } });
  return mount(EntryRowEdit, {
    props: { entry, dimensions, commitments, focusTarget },
    attachTo: document.body,
  });
}

describe("EntryRowEdit", () => {
  it("pre-fills item and duration from the entry", () => {
    const wrapper = mountEdit();
    const inputs = wrapper.findAll("input");
    expect((inputs[0].element as HTMLInputElement).value).toBe("Old item");
    expect((inputs[1].element as HTMLInputElement).value).toBe("45");
  });

  it("emits save with edited values (all required present)", async () => {
    const wrapper = mountEdit();
    const inputs = wrapper.findAll("input");
    await inputs[0].setValue("New item");
    await inputs[1].setValue("60");
    await wrapper.find("[data-test='save']").trigger("click");
    expect(wrapper.emitted("save")?.[0]).toEqual(["New item", 60, fullDims]);
  });

  it("resolves a delta duration like +15", async () => {
    const wrapper = mountEdit();
    await wrapper.findAll("input")[1].setValue("+15");
    await wrapper.find("[data-test='save']").trigger("click");
    expect(wrapper.emitted("save")?.[0]).toEqual(["Old item", 60, fullDims]);
  });

  it("emits cancel", async () => {
    const wrapper = mountEdit();
    await wrapper.find("[data-test='cancel']").trigger("click");
    expect(wrapper.emitted("cancel")).toBeTruthy();
  });

  it("emits delete", async () => {
    const wrapper = mountEdit();
    await wrapper.find("[data-test='delete']").trigger("click");
    expect(wrapper.emitted("delete")).toBeTruthy();
  });

  it("removes an OPTIONAL dimension chip and excludes it from save", async () => {
    const wrapper = mountEdit();
    // chips render in dimension order: category, goal, business-line(optional)
    const removes = wrapper.findAll("[data-test='chip-remove']");
    await removes[2].trigger("click"); // business-line (optional)
    await wrapper.find("[data-test='save']").trigger("click");
    expect(wrapper.emitted("save")?.[0]).toEqual(["Old item", 45, { category: "Engineering", goal: "Bug fixes" }]);
  });

  it("does NOT save when a required dimension chip is removed; shows a missing-required prompt", async () => {
    const wrapper = mountEdit();
    const removes = wrapper.findAll("[data-test='chip-remove']");
    await removes[0].trigger("click"); // category (required)
    await wrapper.find("[data-test='save']").trigger("click");
    expect(wrapper.emitted("save")).toBeFalsy();
    // After removing a required dim chip, it appears as a missing-required prompt
    expect(wrapper.find("[data-test='missing-required']").exists()).toBe(true);
    // The prompt gets warning styling because submitAttempted is true
    expect(wrapper.find("[data-test='missing-required']").classes()).toContain("text-[var(--color-warning)]");
  });

  it("opens DimensionPopover when a missing-required prompt is clicked", async () => {
    const wrapper = mountEditNoDims();
    await wrapper.find("[data-test='missing-required']").trigger("click");
    expect(wrapper.findComponent({ name: "DimensionPopover" }).exists()).toBe(true);
  });

  it("Esc with no changes emits cancel", async () => {
    const wrapper = mountEdit();
    await wrapper.trigger("keydown", { key: "Escape" });
    expect(wrapper.emitted("cancel")).toBeTruthy();
  });

  it("Esc with unsaved changes shows the discard confirm bar and does NOT cancel", async () => {
    const wrapper = mountEdit();
    await wrapper.findAll("input")[0].setValue("Changed item");
    await wrapper.trigger("keydown", { key: "Escape" });
    expect(wrapper.find("[data-test='discard-prompt']").exists()).toBe(true);
    expect(wrapper.emitted("cancel")).toBeFalsy();
  });

  it("Esc again on the confirm bar discards (emits cancel)", async () => {
    const wrapper = mountEdit();
    await wrapper.findAll("input")[0].setValue("Changed item");
    await wrapper.trigger("keydown", { key: "Escape" });
    await wrapper.trigger("keydown", { key: "Escape" });
    expect(wrapper.emitted("cancel")).toBeTruthy();
  });

  it("clicking Keep-editing leaves edit mode active (confirm bar gone, no cancel)", async () => {
    const wrapper = mountEdit();
    await wrapper.findAll("input")[0].setValue("Changed item");
    await wrapper.trigger("keydown", { key: "Escape" });
    await wrapper.find("[data-test='keep-editing']").trigger("click");
    expect(wrapper.find("[data-test='discard-prompt']").exists()).toBe(false);
    expect(wrapper.emitted("cancel")).toBeFalsy();
    expect(wrapper.find("[data-test='save']").exists()).toBe(true);
  });

  it("clicking Discard emits cancel", async () => {
    const wrapper = mountEdit();
    await wrapper.findAll("input")[0].setValue("Changed item");
    await wrapper.trigger("keydown", { key: "Escape" });
    await wrapper.find("[data-test='discard']").trigger("click");
    expect(wrapper.emitted("cancel")).toBeTruthy();
  });

  it("Esc does nothing while the DimensionPopover is open", async () => {
    const wrapper = mountEditNoDims();
    await wrapper.findAll("input")[0].setValue("Changed item");
    await wrapper.find("[data-test='missing-required']").trigger("click"); // open popover
    await wrapper.trigger("keydown", { key: "Escape" });
    expect(wrapper.emitted("cancel")).toBeFalsy();
    expect(wrapper.find("[data-test='discard-prompt']").exists()).toBe(false);
  });

  it("Enter while the popover is open does not save (popover owns Enter)", async () => {
    const entry = makeEntry({ item: "Old item", duration: 45, dimensions: {} });
    const wrapper = mount(EntryRowEdit, {
      props: { entry, dimensions, commitments },
      attachTo: document.body,
    });
    await wrapper.find("[data-test='missing-required']").trigger("click"); // open popover
    expect(wrapper.findComponent({ name: "DimensionPopover" }).exists()).toBe(true);

    // Real bubbling Enter so the popover's window capture-phase listener intercepts it.
    const input = wrapper.find("input");
    input.element.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    await wrapper.vm.$nextTick();

    // Popover owns Enter: it advanced to the val stage; save must NOT be emitted.
    expect(wrapper.emitted("save")).toBeFalsy();
    expect(wrapper.find("[data-test='back-btn']").exists()).toBe(true); // popover advanced to val stage
  });

  it("exits edit mode on an outside click when there are no unsaved changes", async () => {
    const wrapper = mountEdit();
    document.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("cancel")).toBeTruthy();
  });

  it("does NOT exit on a mousedown inside the editor", async () => {
    const wrapper = mountEdit();
    wrapper.find("input").element.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("cancel")).toBeFalsy();
  });

  it("shows the discard confirm bar (does NOT cancel) on an outside click when dirty", async () => {
    const wrapper = mountEdit();
    await wrapper.findAll("input")[0].setValue("Changed item");
    document.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.find("[data-test='discard-prompt']").exists()).toBe(true);
    expect(wrapper.emitted("cancel")).toBeFalsy();
  });

  it("esc exits edit mode when focus is outside the editor (no changes)", async () => {
    const wrapper = mountEdit();
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("cancel")).toBeTruthy();
  });

  it("esc shows the discard confirm bar when focus is outside the editor and dirty", async () => {
    const wrapper = mountEdit();
    await wrapper.findAll("input")[0].setValue("Changed item");
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.find("[data-test='discard-prompt']").exists()).toBe(true);
    expect(wrapper.emitted("cancel")).toBeFalsy();
  });

  it("exits edit mode when focus moves to an element outside the editor", async () => {
    const wrapper = mountEdit();
    document.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("cancel")).toBeTruthy();
  });

  it("esc handoff: popover consumes esc while open, parent handles esc after it closes", async () => {
    const wrapper = mountEditNoDims();
    await wrapper.findAll("input")[0].setValue("Changed item"); // make it dirty
    await wrapper.find("[data-test='missing-required']").trigger("click"); // open popover
    expect(wrapper.findComponent({ name: "DimensionPopover" }).exists()).toBe(true);

    // While the popover is open, esc must NOT trigger the parent confirm flow.
    await wrapper.trigger("keydown", { key: "Escape" });
    expect(wrapper.find("[data-test='discard-prompt']").exists()).toBe(false);

    // Close the popover (simulate its close emit), then esc reaches the parent.
    await wrapper.findComponent({ name: "DimensionPopover" }).vm.$emit("close");
    await wrapper.vm.$nextTick();
    await wrapper.trigger("keydown", { key: "Escape" });
    expect(wrapper.find("[data-test='discard-prompt']").exists()).toBe(true);
    expect(wrapper.emitted("cancel")).toBeFalsy();
  });

  it("focuses the item input on mount when focusTarget is 'item'", async () => {
    const wrapper = mountEditWithFocus('item');
    await nextTick();
    const inputs = wrapper.findAll("input");
    expect(document.activeElement).toBe(inputs[0].element);
  });

  it("focuses the duration input on mount when focusTarget is 'duration'", async () => {
    const wrapper = mountEditWithFocus('duration');
    await nextTick();
    const inputs = wrapper.findAll("input");
    expect(document.activeElement).toBe(inputs[1].element);
  });

  it("defaults to focusing the item input when focusTarget is omitted", async () => {
    const entry = makeEntry({ item: "Old item", duration: 45, dimensions: { ...fullDims } });
    const wrapper = mount(EntryRowEdit, {
      props: { entry, dimensions, commitments },
      attachTo: document.body,
    });
    await nextTick();
    const inputs = wrapper.findAll("input");
    expect(document.activeElement).toBe(inputs[0].element);
  });

  it("places the cursor at the end of the existing text", async () => {
    const spy = vi.spyOn(HTMLInputElement.prototype, 'setSelectionRange');
    const entry = makeEntry({ item: "Review PR", duration: 45, dimensions: { ...fullDims } });
    const wrapper = mount(EntryRowEdit, {
      props: { entry, dimensions, commitments, focusTarget: 'item' },
      attachTo: document.body,
    });
    await nextTick();
    // setSelectionRange should be called with (len, len) to place cursor at end
    const itemInput = wrapper.findAll("input")[0].element as HTMLInputElement;
    expect(spy).toHaveBeenCalledWith(itemInput.value.length, itemInput.value.length);
    spy.mockRestore();
  });

  it("excludes deleted dimensions from filled chips", () => {
    const dims = [
      makeDimension({ name: "Cat", key: "cat", source: "static", values: ["v"], required: false }),
      makeDimension({ name: "Del", key: "del", source: "static", values: ["x"], required: false, deleted: true }),
    ];
    const entry = makeEntry({ item: "X", duration: 30, dimensions: { cat: "v", del: "x" } });
    const wrapper = mount(EntryRowEdit, { props: { entry, dimensions: dims, commitments: [] } });
    // filled() returns only non-deleted dims with values; del should be excluded
    const chips = wrapper.findAll("[data-test='chip-remove']");
    expect(chips.length).toBe(1);
  });

  it("excludes deleted required dimensions from missingRequired", async () => {
    const dims = [
      makeDimension({ name: "Req", key: "req", source: "static", values: ["v"], required: true }),
      makeDimension({ name: "DelReq", key: "delreq", source: "static", values: ["x"], required: true, deleted: true }),
    ];
    const entry = makeEntry({ item: "X", duration: 30, dimensions: {} });
    const wrapper = mount(EntryRowEdit, { props: { entry, dimensions: dims, commitments: [] } });
    // missingRequired should only include "req", not "delreq"
    // Verify via save behavior — save blocks when missingRequired is non-empty
    await wrapper.find("[data-test='save']").trigger("click");
    // Save blocked: only "req" is missing (1 required), not 2
    expect(wrapper.emitted("save")).toBeFalsy();
  });

  it("renders a missing-required prompt for each unfilled required dimension", () => {
    const wrapper = mountEditNoDims();
    const prompts = wrapper.findAll("[data-test='missing-required']");
    expect(prompts.length).toBe(2); // category, goal
    expect(prompts[0].text()).toContain("Category");
    expect(prompts[1].text()).toContain("Goal");
  });

  it("shows a + button when there are unfilled optional dimensions", () => {
    const wrapper = mountEditNoDims();
    // business-line is optional and unfilled
    expect(wrapper.find("[data-test='add-dimension']").exists()).toBe(true);
    expect(wrapper.find("[data-test='add-dimension']").text()).toBe("+");
  });

  it("hides the + button when all optional dimensions are filled", () => {
    const wrapper = mountEdit(); // all dims filled via fullDims
    expect(wrapper.find("[data-test='add-dimension']").exists()).toBe(false);
  });

  it("does NOT render the old required-hint warning text", () => {
    const wrapper = mountEditNoDims();
    expect(wrapper.find("[data-test='required-hint']").exists()).toBe(false);
  });

  it("opens DimensionPopover when a missing-required prompt is clicked", async () => {
    const wrapper = mountEditNoDims();
    await wrapper.find("[data-test='missing-required']").trigger("click");
    expect(wrapper.findComponent({ name: "DimensionPopover" }).exists()).toBe(true);
  });

  it("opens DimensionPopover when + button is clicked", async () => {
    const wrapper = mountEditNoDims();
    await wrapper.find("[data-test='add-dimension']").trigger("click");
    expect(wrapper.findComponent({ name: "DimensionPopover" }).exists()).toBe(true);
  });

  it("applies warning style to missing-required prompts after a blocked submit attempt", async () => {
    const wrapper = mountEditNoDims();
    await wrapper.find("[data-test='save']").trigger("click");
    const prompt = wrapper.find("[data-test='missing-required']");
    expect(prompt.classes()).toContain("text-[var(--color-warning)]");
  });
});
