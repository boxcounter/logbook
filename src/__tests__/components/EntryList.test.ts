// src/__tests__/components/EntryList.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import { reactive } from "vue";
import EntryList from "../../components/EntryList.vue";
import { STORE_KEY } from "../../stores/useStore";
import { makeEntry, makeDimensions, makeCommitment } from "../mocks/fixtures";

function mountList(entries = [makeEntry({ item: "A", duration: 60 }), makeEntry({ item: "B", duration: 30 })]) {
  const store = reactive({ dimensions: makeDimensions(), usingDefaultDimensions: false, commitments: [makeCommitment()] });
  return mount(EntryList, {
    props: { entries },
    global: { provide: { [STORE_KEY as symbol]: store } },
  });
}

describe("EntryList", () => {
  it("renders one EntryRow per entry", () => {
    const wrapper = mountList();
    expect(wrapper.findAllComponents({ name: "EntryRow" }).length).toBe(2);
  });

  it("shows an empty state when there are no entries", () => {
    const wrapper = mountList([]);
    expect(wrapper.text()).toContain("No entries for this day.");
  });

  it("shows a different empty state for non-today days", () => {
    const wrapper = mountList([]);
    // mountList doesn't pass isToday, so it defaults to undefined/false → non-today message
    expect(wrapper.text()).toContain("No entries for this day.");
    expect(wrapper.text()).not.toContain("Log your first work item below");
  });

  it("shows the full CTA message when isToday is true", () => {
    const store = reactive({ dimensions: makeDimensions(), usingDefaultDimensions: false, commitments: [makeCommitment()] });
    const wrapper = mount(EntryList, {
      props: { entries: [], isToday: true },
      global: { provide: { [STORE_KEY as symbol]: store } },
    });
    expect(wrapper.text()).toContain("No entries yet. Log your first work item below.");
  });

  it("re-emits update from a row", async () => {
    const wrapper = mountList();
    wrapper.findAllComponents({ name: "EntryRow" })[0].vm.$emit("update", "id1", "X", 45);
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("update")?.[0]).toEqual(["id1", "X", 45]);
  });

  it("re-emits delete from a row", async () => {
    const wrapper = mountList();
    wrapper.findAllComponents({ name: "EntryRow" })[0].vm.$emit("delete", "id1");
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("delete")?.[0]).toEqual(["id1"]);
  });
});
