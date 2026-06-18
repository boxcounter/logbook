import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import EntryRow from "../../../components/composite/EntryRow.vue";
import { makeEntry, makeConfig } from "../../mocks/fixtures";
import { STORE_KEY } from "../../../stores/useStore";
import { createTestStore } from "../../mocks/store";

function mountRow(entryOverrides: Record<string, unknown> = {}, configOverrides: Record<string, unknown> = {}) {
  const store = createTestStore({ config: makeConfig(configOverrides as any) });
  const entry = makeEntry(entryOverrides as any);
  const wrapper = mount(EntryRow, {
    props: { entry, index: 0 },
    global: { provide: { [STORE_KEY as symbol]: store } },
  });
  return { wrapper, store, entry };
}

describe("EntryRow", () => {
  it("renders index, item text, and formatted duration", () => {
    const { wrapper } = mountRow({ item: "Write tests", duration: 75 });
    const text = wrapper.text();
    expect(text).toContain("1");
    expect(text).toContain("Write tests");
    expect(text).toContain("1h 15m");
  });

  it("double-click item text enters edit mode", async () => {
    const { wrapper } = mountRow({ item: "Original" });
    const display = wrapper.find("span.flex-1");
    await display.trigger("dblclick");
    expect(wrapper.find("input.flex-1").exists()).toBe(true);
  });

  it("edit item: Enter commits the change", async () => {
    const { wrapper } = mountRow({ id: "e1", item: "Old", duration: 30 });
    const display = wrapper.find("span.flex-1");
    await display.trigger("dblclick");
    const input = wrapper.find("input.flex-1");
    await input.setValue("New item");
    await input.trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("update")?.[0]).toEqual(["e1", "New item", 30]);
  });

  it("edit item: Escape cancels", async () => {
    const { wrapper } = mountRow({ item: "Original" });
    const display = wrapper.find("span.flex-1");
    await display.trigger("dblclick");
    const input = wrapper.find("input.flex-1");
    await input.setValue("Changed");
    await input.trigger("keydown", { key: "Escape" });
    expect(wrapper.emitted("update")).toBeUndefined();
    expect(wrapper.text()).toContain("Original");
  });

  it("double-click duration enters edit mode", async () => {
    const { wrapper } = mountRow({ duration: 45 });
    // There are two .tabular-nums elements: index span (0) and duration span (1)
    const durDisplay = wrapper.findAll(".tabular-nums")[1];
    await durDisplay.trigger("dblclick");
    expect(wrapper.find("input[class*='w-\\[56px\\]']").exists()).toBe(true);
  });

  it("edit duration: delta +30 adds to existing", async () => {
    const { wrapper } = mountRow({ id: "e1", item: "Task", duration: 60 });
    const durDisplay = wrapper.findAll(".tabular-nums")[1];
    await durDisplay.trigger("dblclick");
    const input = wrapper.find("input[class*='w-\\[56px\\]']");
    await input.setValue("+30");
    await input.trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("update")?.[0]).toEqual(["e1", "Task", 90]);
  });

  it("emits delete on button click", async () => {
    const { wrapper } = mountRow({ id: "del-me" });
    const btn = wrapper.find("button");
    await btn.trigger("click");
    expect(wrapper.emitted("delete")?.[0]).toEqual(["del-me"]);
  });

  it("shows dimension chips for set dimensions", () => {
    const { wrapper } = mountRow({
      dimensions: { goal: "Ship feature", "business-line": "Platform" },
    });
    expect(wrapper.text()).toContain("Ship feature");
    expect(wrapper.text()).toContain("Platform");
  });
});
