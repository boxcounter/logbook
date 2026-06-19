// src/__tests__/components/composite/EntryRowEdit.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import EntryRowEdit from "../../../components/composite/EntryRowEdit.vue";
import { makeEntry, makeDimension, makeCommitment } from "../../mocks/fixtures";

const dimensions = [
  makeDimension({ name: "Category", key: "category", source: "static", values: ["Engineering"], required: true }),
  makeDimension({ name: "Goal", key: "goal", source: "monthly", required: true }),
];
const commitments = [makeCommitment({ goals: ["Bug fixes"] })];

function mountEdit(entryOverrides = {}) {
  const entry = makeEntry({ item: "Old item", duration: 45, dimensions: { category: "Engineering" }, ...entryOverrides });
  return mount(EntryRowEdit, { props: { entry, dimensions, commitments } });
}

describe("EntryRowEdit", () => {
  it("pre-fills item and duration from the entry", () => {
    const wrapper = mountEdit();
    const inputs = wrapper.findAll("input");
    expect((inputs[0].element as HTMLInputElement).value).toBe("Old item");
    expect((inputs[1].element as HTMLInputElement).value).toBe("45");
  });

  it("emits save with edited values", async () => {
    const wrapper = mountEdit();
    const inputs = wrapper.findAll("input");
    await inputs[0].setValue("New item");
    await inputs[1].setValue("60");
    await wrapper.find("[data-test='save']").trigger("click");
    expect(wrapper.emitted("save")?.[0]).toEqual(["New item", 60, { category: "Engineering" }]);
  });

  it("resolves a delta duration like +15", async () => {
    const wrapper = mountEdit();
    await wrapper.findAll("input")[1].setValue("+15");
    await wrapper.find("[data-test='save']").trigger("click");
    expect(wrapper.emitted("save")?.[0]).toEqual(["Old item", 60, { category: "Engineering" }]);
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

  it("removes a dimension chip and excludes it from save", async () => {
    const wrapper = mountEdit();
    await wrapper.find("[data-test='chip-remove']").trigger("click");
    await wrapper.find("[data-test='save']").trigger("click");
    expect(wrapper.emitted("save")?.[0]).toEqual(["Old item", 45, {}]);
  });

  it("opens DimensionPopover when + tag is clicked", async () => {
    const wrapper = mountEdit();
    await wrapper.find("[data-test='add-tag']").trigger("click");
    expect(wrapper.findComponent({ name: "DimensionPopover" }).exists()).toBe(true);
  });
});
