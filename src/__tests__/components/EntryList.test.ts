import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import EntryList from "../../components/EntryList.vue";
import EntryItem from "../../components/EntryItem.vue";
import EntryGroup from "../../components/EntryGroup.vue";
import { makeEntry } from "../mocks/fixtures";
import { STORE_KEY } from "../../stores/useStore";
import { createTestStore } from "../mocks/store";
import { makeConfig } from "../mocks/fixtures";

// EntryList renders EntryItem/EntryGroup which use useStore()
const store = createTestStore({ config: makeConfig() });
const provide = { [STORE_KEY as symbol]: store };

// ============================================================

describe("EntryList", () => {
  it("day mode empty: shows empty state message", () => {
    const wrapper = mount(EntryList, {
      props: { entries: [], granularity: "day" },
      global: { provide },
    });
    expect(wrapper.text()).toContain("No entries yet");
  });

  it("week mode empty: shows empty state message", () => {
    const wrapper = mount(EntryList, {
      props: { entries: [], granularity: "week", periodEntries: {} },
      global: { provide },
    });
    expect(wrapper.text()).toContain("No entries for this period");
  });

  it("month mode empty: shows empty state message", () => {
    const wrapper = mount(EntryList, {
      props: { entries: [], granularity: "month", periodEntries: {} },
      global: { provide },
    });
    expect(wrapper.text()).toContain("No entries for this period");
  });

  it("day mode with entries: renders flat EntryItem list", () => {
    const entries = [makeEntry(), makeEntry(), makeEntry()];
    const wrapper = mount(EntryList, {
      props: { entries, granularity: "day" },
      global: { provide },
    });
    expect(wrapper.findAllComponents(EntryItem)).toHaveLength(3);
    expect(wrapper.findAllComponents(EntryGroup)).toHaveLength(0);
  });

  it("week mode: groups entries by day and renders EntryGroup per day", () => {
    const e1 = makeEntry({ item: "Mon task" });
    const e2 = makeEntry({ item: "Tue task" });
    const periodEntries = {
      "2026-06-08": [e1],  // Monday
      "2026-06-09": [e2],  // Tuesday
    };
    const wrapper = mount(EntryList, {
      props: { entries: [e1, e2], granularity: "week", periodEntries },
      global: { provide },
    });
    expect(wrapper.findAllComponents(EntryGroup)).toHaveLength(2);
  });

  it("month mode: groups entries by ISO week", () => {
    const e1 = makeEntry({ item: "Week 1 task" });
    const e2 = makeEntry({ item: "Week 3 task" });
    const periodEntries = {
      "2026-06-01": [e1],  // Monday of week 1
      "2026-06-15": [e2],  // Monday of week 3
    };
    const wrapper = mount(EntryList, {
      props: { entries: [e1, e2], granularity: "month", periodEntries },
      global: { provide },
    });
    const groups = wrapper.findAllComponents(EntryGroup);
    expect(groups.length).toBeGreaterThanOrEqual(1);
  });

  it("week mode: skips days with zero entries", () => {
    const e1 = makeEntry();
    const periodEntries = {
      "2026-06-08": [e1],  // has entries
      "2026-06-09": [],     // empty — should be skipped
    };
    const wrapper = mount(EntryList, {
      props: { entries: [e1], granularity: "week", periodEntries },
      global: { provide },
    });
    // Only 1 group despite 2 days in periodEntries
    expect(wrapper.findAllComponents(EntryGroup)).toHaveLength(1);
  });

  it("bubbles update event from child", async () => {
    const entries = [makeEntry({ id: "e1", item: "Test", duration: 30 })];
    const wrapper = mount(EntryList, {
      props: { entries, granularity: "day" },
      global: { provide },
    });

    const item = wrapper.findComponent(EntryItem);
    await item.vm.$emit("update", "e1", "Updated", 45);

    expect(wrapper.emitted("update")).toBeTruthy();
    expect(wrapper.emitted("update")![0]).toEqual(["e1", "Updated", 45]);
  });

  it("bubbles delete event from child", async () => {
    const entries = [makeEntry({ id: "e1" })];
    const wrapper = mount(EntryList, {
      props: { entries, granularity: "day" },
      global: { provide },
    });

    const item = wrapper.findComponent(EntryItem);
    await item.vm.$emit("delete", "e1");

    expect(wrapper.emitted("delete")).toBeTruthy();
    expect(wrapper.emitted("delete")![0]).toEqual(["e1"]);
  });

  it("bubbles updateDimensions event from child", async () => {
    const entries = [makeEntry({ id: "e1" })];
    const wrapper = mount(EntryList, {
      props: { entries, granularity: "day" },
      global: { provide },
    });

    const item = wrapper.findComponent(EntryItem);
    await item.vm.$emit("update-dimensions", "e1", { goal: "Code review" });

    expect(wrapper.emitted("updateDimensions")).toBeTruthy();
    expect(wrapper.emitted("updateDimensions")![0]).toEqual(["e1", { goal: "Code review" }]);
  });
});
