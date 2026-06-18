import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import EntryList from "../../components/EntryList.vue";
import EntryRow from "../../components/composite/EntryRow.vue";
import { makeEntry } from "../mocks/fixtures";
import { STORE_KEY } from "../../stores/useStore";
import { createTestStore } from "../mocks/store";
import { makeConfig } from "../mocks/fixtures";

const store = createTestStore({ config: makeConfig() });
const provide = { [STORE_KEY as symbol]: store };

describe("EntryList", () => {
  it("empty: shows empty state message", () => {
    const wrapper = mount(EntryList, {
      props: { entries: [] },
      global: { provide },
    });
    expect(wrapper.text()).toContain("No entries yet");
  });

  it("with entries: renders flat EntryRow list", () => {
    const entries = [makeEntry(), makeEntry(), makeEntry()];
    const wrapper = mount(EntryList, {
      props: { entries },
      global: { provide },
    });
    expect(wrapper.findAllComponents(EntryRow)).toHaveLength(3);
  });

  it("bubbles update event from child", async () => {
    const entries = [makeEntry({ id: "e1", item: "Test", duration: 30 })];
    const wrapper = mount(EntryList, {
      props: { entries },
      global: { provide },
    });
    const item = wrapper.findComponent(EntryRow);
    await item.vm.$emit("update", "e1", "Updated", 45);
    expect(wrapper.emitted("update")).toBeTruthy();
    expect(wrapper.emitted("update")![0]).toEqual(["e1", "Updated", 45]);
  });

  it("bubbles delete event from child", async () => {
    const entries = [makeEntry({ id: "e1" })];
    const wrapper = mount(EntryList, {
      props: { entries },
      global: { provide },
    });
    const item = wrapper.findComponent(EntryRow);
    await item.vm.$emit("delete", "e1");
    expect(wrapper.emitted("delete")).toBeTruthy();
    expect(wrapper.emitted("delete")![0]).toEqual(["e1"]);
  });

  it("bubbles updateDimensions event from child", async () => {
    const entries = [makeEntry({ id: "e1" })];
    const wrapper = mount(EntryList, {
      props: { entries },
      global: { provide },
    });
    const item = wrapper.findComponent(EntryRow);
    await item.vm.$emit("updateDimensions", "e1", { goal: "Code review" });
    expect(wrapper.emitted("updateDimensions")).toBeTruthy();
    expect(wrapper.emitted("updateDimensions")![0]).toEqual(["e1", { goal: "Code review" }]);
  });

  it("shows inline summary row when entries exist", () => {
    const entries = [makeEntry({ duration: 30 }), makeEntry({ duration: 45 })];
    const wrapper = mount(EntryList, {
      props: { entries },
      global: { provide },
    });
    const text = wrapper.text();
    expect(text).toContain("2 entries");
    expect(text).toContain("1h 15m");
  });

  it('singular "1 entry" in summary', () => {
    const entries = [makeEntry({ duration: 15 })];
    const wrapper = mount(EntryList, {
      props: { entries },
      global: { provide },
    });
    expect(wrapper.text()).toContain("1 entry");
  });

  it("no summary row when no entries", () => {
    const wrapper = mount(EntryList, {
      props: { entries: [] },
      global: { provide },
    });
    // Summary row should not render when list is empty
    expect(wrapper.find(".border-t-2").exists()).toBe(false);
  });
});
