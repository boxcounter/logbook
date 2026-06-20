// src/__tests__/components/composite/EntryRowEdit.test.ts
import { describe, it, expect, afterEach } from "vitest";
import { mount, enableAutoUnmount } from "@vue/test-utils";
import EntryRowEdit from "../../../components/composite/EntryRowEdit.vue";
import { makeEntry, makeDimension, makeCommitment } from "../../mocks/fixtures";

// DimensionPopover registers a window keydown listener; unmount after each test.
enableAutoUnmount(afterEach);

const dimensions = [
  makeDimension({ name: "Category", key: "category", source: "static", values: ["Engineering"], required: true }),
  makeDimension({ name: "Goal", key: "goal", source: "monthly", required: true }),
  makeDimension({ name: "Business Line", key: "business-line", source: "static", values: ["Slax"], required: false }),
];
const commitments = [makeCommitment({ goals: ["Bug fixes"] })];

const fullDims = { category: "Engineering", goal: "Bug fixes", "business-line": "Slax" };

function mountEdit(entryOverrides = {}) {
  const entry = makeEntry({ item: "Old item", duration: 45, dimensions: { ...fullDims }, ...entryOverrides });
  return mount(EntryRowEdit, { props: { entry, dimensions, commitments } });
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

  it("does NOT save when a required dimension chip is removed; shows a required hint", async () => {
    const wrapper = mountEdit();
    const removes = wrapper.findAll("[data-test='chip-remove']");
    await removes[0].trigger("click"); // category (required)
    await wrapper.find("[data-test='save']").trigger("click");
    expect(wrapper.emitted("save")).toBeFalsy();
    expect(wrapper.find("[data-test='required-hint']").exists()).toBe(true);
  });

  it("opens DimensionPopover when + tag is clicked", async () => {
    const wrapper = mountEdit();
    await wrapper.find("[data-test='add-tag']").trigger("click");
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
    const wrapper = mountEdit();
    await wrapper.findAll("input")[0].setValue("Changed item");
    await wrapper.find("[data-test='add-tag']").trigger("click"); // open popover
    await wrapper.trigger("keydown", { key: "Escape" });
    expect(wrapper.emitted("cancel")).toBeFalsy();
    expect(wrapper.find("[data-test='discard-prompt']").exists()).toBe(false);
  });
});
