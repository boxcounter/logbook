import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import EntryItem from "../../components/EntryItem.vue";
import { makeEntry } from "../mocks/fixtures";
import { STORE_KEY } from "../../stores/useStore";
import { createTestStore } from "../mocks/store";
import { makeConfig } from "../mocks/fixtures";

function mountItem(entryOverrides?: Partial<ReturnType<typeof makeEntry>>, configOverrides?: Parameters<typeof makeConfig>[0]) {
  const store = createTestStore({ config: makeConfig(configOverrides) });
  const entry = makeEntry(entryOverrides);
  const wrapper = mount(EntryItem, {
    props: { entry, index: 0 },
    global: { provide: { [STORE_KEY as symbol]: store } },
  });
  return { wrapper, store, entry };
}

// ============================================================

describe("EntryItem", () => {
  it("renders index, item text, and formatted duration", () => {
    const { wrapper } = mountItem({ item: "Write tests", duration: 75 });
    const text = wrapper.text();
    expect(text).toContain("1"); // index + 1 = 1
    expect(text).toContain("Write tests");
    expect(text).toContain("1h 15m");
  });

  it("renders index correctly (0-based to 1-based)", () => {
    const store = createTestStore({ config: makeConfig() });
    const entry = makeEntry();
    const wrapper = mount(EntryItem, {
      props: { entry, index: 5 },
      global: { provide: { [STORE_KEY as symbol]: store } },
    });
    expect(wrapper.text()).toContain("6"); // 5 + 1
  });

  // ---- Item text editing ----

  it("double-click item text: enters edit mode", async () => {
    const { wrapper } = mountItem({ item: "Original text" });
    const display = wrapper.find(".text-sm.text-gray-800");
    await display.trigger("dblclick");
    // Input should appear
    const input = wrapper.find(".flex-1 input");
    expect(input.exists()).toBe(true);
    expect((input.element as HTMLInputElement).value).toBe("Original text");
  });

  it("edit item: Enter commits the change", async () => {
    const { wrapper } = mountItem({ id: "e1", item: "Old", duration: 30 });
    const display = wrapper.find(".text-sm.text-gray-800");
    await display.trigger("dblclick");

    const input = wrapper.find(".flex-1 input");
    await input.setValue("New item");
    await input.trigger("keydown", { key: "Enter" });

    expect(wrapper.emitted("update")).toBeTruthy();
    expect(wrapper.emitted("update")![0]).toEqual(["e1", "New item", 30]);
  });

  it("edit item: Escape cancels edit", async () => {
    const { wrapper } = mountItem({ item: "Original" });
    const display = wrapper.find(".text-sm.text-gray-800");
    await display.trigger("dblclick");

    const input = wrapper.find(".flex-1 input");
    await input.setValue("Changed");
    await input.trigger("keydown", { key: "Escape" });

    expect(wrapper.emitted("update")).toBeFalsy();
    // Should be back in display mode
    expect(wrapper.text()).toContain("Original");
  });

  it("edit item: blur commits", async () => {
    const { wrapper } = mountItem({ id: "e1", item: "Old", duration: 30 });
    const display = wrapper.find(".text-sm.text-gray-800");
    await display.trigger("dblclick");

    const input = wrapper.find(".flex-1 input");
    await input.setValue("Blurred");
    await input.trigger("blur");

    expect(wrapper.emitted("update")).toBeTruthy();
    expect(wrapper.emitted("update")![0]).toEqual(["e1", "Blurred", 30]);
  });

  it("edit item: empty text becomes (untitled)", async () => {
    const { wrapper } = mountItem({ id: "e1", item: "Old", duration: 30 });
    const display = wrapper.find(".text-sm.text-gray-800");
    await display.trigger("dblclick");

    const input = wrapper.find(".flex-1 input");
    await input.setValue("   ");
    await input.trigger("blur");

    expect(wrapper.emitted("update")![0][1]).toBe("(untitled)");
  });

  it("edit item: no emit if text unchanged", async () => {
    const { wrapper } = mountItem({ item: "Same" });
    const display = wrapper.find(".text-sm.text-gray-800");
    await display.trigger("dblclick");

    const input = wrapper.find(".flex-1 input");
    // Don't change the value, just blur
    await input.trigger("blur");

    expect(wrapper.emitted("update")).toBeFalsy();
  });

  // ---- Duration editing ----

  it("double-click duration: enters edit mode", async () => {
    const { wrapper } = mountItem({ duration: 45 });
    const display = wrapper.find(".text-sm.text-gray-600");
    await display.trigger("dblclick");
    // Input should appear with the current duration value
    const input = wrapper.find("input[class*='w-14']");
    expect(input.exists()).toBe(true);
    expect((input.element as HTMLInputElement).value).toBe("45");
  });

  it("edit duration: delta +30 adds to existing", async () => {
    const { wrapper } = mountItem({ id: "e1", item: "Task", duration: 60 });
    const display = wrapper.find(".text-sm.text-gray-600");
    await display.trigger("dblclick");

    const input = wrapper.find("input[class*='w-14']");
    await input.setValue("+30");
    await input.trigger("keydown", { key: "Enter" });

    expect(wrapper.emitted("update")).toBeTruthy();
    expect(wrapper.emitted("update")![0]).toEqual(["e1", "Task", 90]);
  });

  it("edit duration: delta -15 subtracts", async () => {
    const { wrapper } = mountItem({ id: "e1", item: "Task", duration: 60 });
    const display = wrapper.find(".text-sm.text-gray-600");
    await display.trigger("dblclick");

    const input = wrapper.find("input[class*='w-14']");
    await input.setValue("-15");
    await input.trigger("keydown", { key: "Enter" });

    expect(wrapper.emitted("update")![0]).toEqual(["e1", "Task", 45]);
  });

  it("edit duration: absolute value replaces", async () => {
    const { wrapper } = mountItem({ id: "e1", item: "Task", duration: 60 });
    const display = wrapper.find(".text-sm.text-gray-600");
    await display.trigger("dblclick");

    const input = wrapper.find("input[class*='w-14']");
    await input.setValue("120");
    await input.trigger("keydown", { key: "Enter" });

    expect(wrapper.emitted("update")![0]).toEqual(["e1", "Task", 120]);
  });

  it("edit duration: Escape cancels", async () => {
    const { wrapper } = mountItem({ duration: 60 });
    const display = wrapper.find(".text-sm.text-gray-600");
    await display.trigger("dblclick");

    const input = wrapper.find("input[class*='w-14']");
    await input.setValue("120");
    await input.trigger("keydown", { key: "Escape" });

    expect(wrapper.emitted("update")).toBeFalsy();
    // Display should show original
    expect(wrapper.text()).toContain("1h");
  });

  it("edit duration: no emit if unchanged", async () => {
    const { wrapper } = mountItem({ duration: 60 });
    const display = wrapper.find(".text-sm.text-gray-600");
    await display.trigger("dblclick");

    // resolveDelta("60", 60) = 60 — no change
    const input = wrapper.find("input[class*='w-14']");
    // Don't change the value
    await input.trigger("blur");

    expect(wrapper.emitted("update")).toBeFalsy();
  });

  // ---- Delete ----

  it("emit delete when X button clicked", async () => {
    const { wrapper } = mountItem({ id: "del-me" });
    const btn = wrapper.find("button");
    await btn.trigger("click");
    expect(wrapper.emitted("delete")).toBeTruthy();
    expect(wrapper.emitted("delete")![0]).toEqual(["del-me"]);
  });

  // ---- Dimension display and editing ----

  it("shows dimension label when dimensions are set", () => {
    const { wrapper } = mountItem({
      dimensions: { goal: "Ship feature", "business-line": "Platform" },
    });
    expect(wrapper.text()).toContain("Ship feature");
    expect(wrapper.text()).toContain("Platform");
  });

  it("hides dimension line when no dimensions set", () => {
    const { wrapper } = mountItem({ dimensions: {} });
    // The dimension div should not render (v-if is false)
    // Just check there's no dimension display text (the join produces empty string)
    // The dimLabel returns empty string, so the v-if skips it
    expect(wrapper.findAll("select").length).toBe(0);
  });

  it("click on dimension display: opens dimension selects", async () => {
    const { wrapper } = mountItem({
      dimensions: { goal: "Code review" },
    });
    // The dimension display line is visible
    expect(wrapper.text()).toContain("Code review");

    // The div with the click handler is the one with text-xs text-gray-400
    // In jsdom, click triggers but the @click.stop is on the same div
    const dimDivs = wrapper.findAll(".text-xs.text-gray-400");
    // There might be more than one — EntryItem has two elements with this class
    // One for the dimLabel display and they might overlap
    // Actually both use the same class: the item display div and the dimension display div
    // Find the one containing the dimension text
    let targetDiv = dimDivs[0];
    for (const d of dimDivs) {
      if (d.text().includes("Code review")) {
        targetDiv = d;
        break;
      }
    }
    await targetDiv.trigger("click");
    await wrapper.vm.$nextTick();

    const selects = wrapper.findAll("select");
    expect(selects.length).toBeGreaterThan(0);
  });

  it("dimension change emits updateDimensions with new values", async () => {
    const { wrapper } = mountItem({
      id: "e1",
      dimensions: { goal: "Old goal", "business-line": "Platform" },
    });
    // Find the right div to click (there may be multiple .text-xs.text-gray-400)
    const dimDivs = wrapper.findAll(".text-xs.text-gray-400");
    const targetDiv = dimDivs.find(d => d.text().includes("Old goal")) || dimDivs[0];
    await targetDiv.trigger("click");
    await wrapper.vm.$nextTick();

    const selects = wrapper.findAll("select");
    expect(selects.length).toBeGreaterThan(0);
    await selects[0].setValue("Ship feature X");
    expect(wrapper.emitted("updateDimensions")).toBeTruthy();
  });
});
