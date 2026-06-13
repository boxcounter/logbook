import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import EntryGroup from "../../components/EntryGroup.vue";
import EntryItem from "../../components/EntryItem.vue";
import { makeEntry } from "../mocks/fixtures";
import { STORE_KEY } from "../../stores/useStore";
import { createTestStore } from "../mocks/store";
import { makeConfig } from "../mocks/fixtures";

// EntryGroup renders EntryItem which uses useStore()
const store = createTestStore({ config: makeConfig() });
const provide = { [STORE_KEY as symbol]: store };

// ============================================================

describe("EntryGroup", () => {
  it("renders label, entry count, and total duration", () => {
    const entries = [makeEntry({ duration: 15 }), makeEntry({ duration: 45 })];
    const wrapper = mount(EntryGroup, {
      props: { label: "Monday", entries },
      global: { provide },
    });
    const text = wrapper.text();
    expect(text).toContain("Monday");
    expect(text).toContain("2 entries");
    expect(text).toContain("1h");
  });

  it("renders EntryItem for each entry when open", () => {
    const entries = [makeEntry(), makeEntry(), makeEntry()];
    const wrapper = mount(EntryGroup, {
      props: { label: "Test", entries, defaultOpen: true },
      global: { provide },
    });
    const items = wrapper.findAllComponents(EntryItem);
    expect(items).toHaveLength(3);
  });

  it("collapses/expands on header click", async () => {
    const entries = [makeEntry()];
    const wrapper = mount(EntryGroup, {
      props: { label: "Test", entries, defaultOpen: true },
      global: { provide },
    });
    expect(wrapper.findAllComponents(EntryItem)).toHaveLength(1);

    await wrapper.find("button").trigger("click");
    expect(wrapper.findAllComponents(EntryItem)).toHaveLength(0);

    await wrapper.find("button").trigger("click");
    expect(wrapper.findAllComponents(EntryItem)).toHaveLength(1);
  });

  it("respects defaultOpen=false prop", () => {
    const entries = [makeEntry()];
    const wrapper = mount(EntryGroup, {
      props: { label: "Test", entries, defaultOpen: false },
      global: { provide },
    });
    expect(wrapper.findAllComponents(EntryItem)).toHaveLength(0);
  });

  it("bubbles update event from EntryItem", async () => {
    const entries = [makeEntry({ id: "test-id-123", item: "Old item", duration: 30 })];
    const wrapper = mount(EntryGroup, {
      props: { label: "Test", entries, defaultOpen: true },
      global: { provide },
    });

    const item = wrapper.findComponent(EntryItem);
    await item.vm.$emit("update", "test-id-123", "New item", 60);

    expect(wrapper.emitted("update")).toBeTruthy();
    expect(wrapper.emitted("update")![0]).toEqual(["test-id-123", "New item", 60]);
  });

  it("bubbles delete and updateDimensions events from EntryItem", async () => {
    const entries = [makeEntry({ id: "abc" })];
    const wrapper = mount(EntryGroup, {
      props: { label: "Test", entries, defaultOpen: true },
      global: { provide },
    });

    const item = wrapper.findComponent(EntryItem);
    await item.vm.$emit("delete", "abc");
    await item.vm.$emit("update-dimensions", "abc", { goal: "test" });

    expect(wrapper.emitted("delete")).toBeTruthy();
    expect(wrapper.emitted("delete")![0]).toEqual(["abc"]);
    expect(wrapper.emitted("updateDimensions")).toBeTruthy();
    expect(wrapper.emitted("updateDimensions")![0]).toEqual(["abc", { goal: "test" }]);
  });
});
