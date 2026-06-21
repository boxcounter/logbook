// src/__tests__/components/composite/EntryRow.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import { reactive } from "vue";
import EntryRow from "../../../components/composite/EntryRow.vue";
import { STORE_KEY } from "../../../stores/useStore";
import { makeEntry, makeDimensions, makeCommitment } from "../../mocks/fixtures";

function mountRow(entryOverrides = {}, extraProps: Record<string, unknown> = {}) {
  const store = reactive({
    dimensions: makeDimensions(),
    fromTemplate: false,
    commitments: [makeCommitment({ goals: ["Bug fixes"] })],
  });
  const entry = makeEntry({ item: "Review PR", duration: 90, dimensions: { category: "Coding" }, ...entryOverrides });
  return mount(EntryRow, {
    props: { entry, index: 0, ...extraProps },
    global: { provide: { [STORE_KEY as symbol]: store } },
  });
}

describe("EntryRow", () => {
  it("renders item text and formatted duration", () => {
    const wrapper = mountRow();
    expect(wrapper.text()).toContain("Review PR");
    expect(wrapper.text()).toContain("1h 30m");
  });

  it("applies the just-added highlight class only when justAdded is true", () => {
    const plain = mountRow();
    expect(plain.find("[data-test='entry-row']").classes()).not.toContain("just-added");
    const added = mountRow({}, { justAdded: true });
    expect(added.find("[data-test='entry-row']").classes()).toContain("just-added");
  });

  it("renders a chip per filled dimension", () => {
    const wrapper = mountRow();
    expect(wrapper.text()).toContain("Coding");
  });

  it("enters edit mode on double-click", async () => {
    const wrapper = mountRow();
    await wrapper.find("[data-test='entry-row']").trigger("dblclick");
    expect(wrapper.findComponent({ name: "EntryRowEdit" }).exists()).toBe(true);
  });

  it("enters edit mode when the ⋯ trigger is clicked", async () => {
    const wrapper = mountRow();
    await wrapper.find("[data-test='edit-trigger']").trigger("click");
    expect(wrapper.findComponent({ name: "EntryRowEdit" }).exists()).toBe(true);
  });

  it("emits update on save when item/duration changed", async () => {
    const wrapper = mountRow();
    await wrapper.find("[data-test='entry-row']").trigger("dblclick");
    const editor = wrapper.findComponent({ name: "EntryRowEdit" });
    editor.vm.$emit("save", "Review PR #2", 120, { category: "Coding" });
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("update")?.[0]).toEqual([wrapper.props("entry").id, "Review PR #2", 120]);
    expect(wrapper.emitted("updateDimensions")).toBeFalsy();
  });

  it("emits updateDimensions on save when only dimensions changed", async () => {
    const wrapper = mountRow();
    await wrapper.find("[data-test='entry-row']").trigger("dblclick");
    const editor = wrapper.findComponent({ name: "EntryRowEdit" });
    editor.vm.$emit("save", "Review PR", 90, { category: "Coding", goal: "Bug fixes" });
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("updateDimensions")?.[0]).toEqual([wrapper.props("entry").id, { category: "Coding", goal: "Bug fixes" }]);
    expect(wrapper.emitted("update")).toBeFalsy();
  });

  it("draws a top hairline divider for non-first rows", () => {
    const wrapper = mountRow({}, { index: 1 });
    expect(wrapper.find("[data-test='entry-row']").classes()).toContain("border-t");
  });

  it("does not draw a top divider on the first row", () => {
    const wrapper = mountRow({}, { index: 0 });
    expect(wrapper.find("[data-test='entry-row']").classes()).not.toContain("border-t");
  });

  it("emits delete from the editor", async () => {
    const wrapper = mountRow();
    await wrapper.find("[data-test='entry-row']").trigger("dblclick");
    wrapper.findComponent({ name: "EntryRowEdit" }).vm.$emit("delete");
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("delete")?.[0]).toEqual([wrapper.props("entry").id]);
  });
});
