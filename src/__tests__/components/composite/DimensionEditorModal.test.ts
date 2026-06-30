import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import DimensionEditorModal from "../../../components/composite/DimensionEditorModal.vue";
import type { Dimension } from "../../../types";

const MOCK_DIMENSIONS: Dimension[] = [
  { name: "Goal", key: "goal", source: "monthly", values: undefined, required: false, deleted: false },
  { name: "Biz", key: "biz", source: "static", values: ["Product", "Marketing", "Engineering"], required: true, deleted: false },
  { name: "Importance", key: "importance-urgency", source: "static", values: ["P0", "P1"], required: false, deleted: false },
];

function mountModal(overrides = {}) {
  return mount(DimensionEditorModal, {
    props: { open: true, dimensions: MOCK_DIMENSIONS, rootPath: "/test", year: 2026, month: 6, ...overrides },
    global: { stubs: { teleport: true } },
  });
}

describe("DimensionEditorModal", () => {
  it("renders when open is true", () => {
    const wrapper = mountModal();
    expect(wrapper.find('[data-test="overlay"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("Edit Dimensions");
  });

  it("does not render when open is false", () => {
    const wrapper = mountModal({ open: false });
    expect(wrapper.find('[data-test="overlay"]').exists()).toBe(false);
  });

  it("emits close when Cancel is clicked", async () => {
    const wrapper = mountModal();
    await wrapper.find('[data-test="cancel"]').trigger("click");
    expect(wrapper.emitted("close")).toBeTruthy();
  });

  it("renders all dimensions in the left panel", () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    expect(wrapper.text()).toContain("Goal");
    expect(wrapper.text()).toContain("Biz");
    expect(wrapper.text()).toContain("Importance");
  });

  it("selects a dimension on click", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    const bizRow = wrapper.findAll('[data-test="dim-row"]')[1]; // second row = Biz
    await bizRow.trigger("click");
    const nameInput = wrapper.find('input[placeholder="Dimension name"]');
    expect((nameInput.element as HTMLInputElement).value).toBe("Biz");
  });

  it("shows values for selected static dimension", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    // Default selects index 0 (Goal, monthly). Click Biz (index 1) for static values.
    const bizRow = wrapper.findAll('[data-test="dim-row"]')[1];
    await bizRow.trigger("click");
    const valueInputs = wrapper.findAll('[data-test="value-input"]');
    const values = valueInputs.map((el) => (el.element as HTMLInputElement).value);
    expect(values).toEqual(["Product", "Marketing", "Engineering"]);
  });

  it("updates dimension name on input", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    const nameInput = wrapper.find('input[placeholder="Dimension name"]');
    await nameInput.setValue("Business");
    expect((nameInput.element as HTMLInputElement).value).toBe("Business");
  });

  it("shows key and source in readonly mode", () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    // Default selects index 0 = Goal, key is "goal", source is "monthly"
    expect(wrapper.text()).toContain("goal");
    expect(wrapper.text()).toContain("monthly");
    expect(wrapper.text()).toContain("locked");
  });

  it("shows monthly info card", () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    // Goal is monthly (index 0, selected by default)
    expect(wrapper.text()).toContain("Values are derived from commitment goals");
  });

  it("does not show values section for monthly dimensions", () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    // Goal is monthly — no values list or "New value" input
    expect(wrapper.find('input[placeholder="New value"]').exists()).toBe(false);
  });

  it("toggles required checkbox", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    const checkbox = wrapper.find('input[type="checkbox"]');
    expect((checkbox.element as HTMLInputElement).checked).toBe(false);
    await checkbox.setValue(true);
    expect((checkbox.element as HTMLInputElement).checked).toBe(true);
  });

  it("adds a new value", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    // Select Biz (index 1) — static with values
    const bizRow = wrapper.findAll('[data-test="dim-row"]')[1];
    await bizRow.trigger("click");
    const newValInput = wrapper.find('input[placeholder="New value"]');
    await newValInput.setValue("Design");
    await wrapper.find('[data-test="add-value"]').trigger("click");
    const valueInputs = wrapper.findAll('[data-test="value-input"]');
    const values = valueInputs.map((el) => (el.element as HTMLInputElement).value);
    expect(values).toContain("Design");
  });

  it("deletes a value", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    // Select Biz (index 1) — has Product, Marketing, Engineering
    const bizRow = wrapper.findAll('[data-test="dim-row"]')[1];
    await bizRow.trigger("click");
    const before = wrapper.findAll('[data-test="value-input"]');
    expect(before.map((el) => (el.element as HTMLInputElement).value)).toContain("Product");
    await wrapper.findAll('[data-test="delete-value"]')[0].trigger("click");
    const after = wrapper.findAll('[data-test="value-input"]');
    expect(after.map((el) => (el.element as HTMLInputElement).value)).not.toContain("Product");
    expect(after.length).toBe(before.length - 1);
  });

  it("clears new value input after adding", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    const bizRow = wrapper.findAll('[data-test="dim-row"]')[1];
    await bizRow.trigger("click");
    const newValInput = wrapper.find('input[placeholder="New value"]');
    await newValInput.setValue("Design");
    await wrapper.find('[data-test="add-value"]').trigger("click");
    await nextTick();
    const after = wrapper.find('input[placeholder="New value"]');
    expect((after.element as HTMLInputElement).value).toBe("");
  });

  it("toggles delete dimension", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    let btn = wrapper.find('[data-test="delete-dim"]');
    expect(btn.text()).toContain("Delete dimension");
    await btn.trigger("click");
    await nextTick();
    btn = wrapper.find('[data-test="delete-dim"]');
    expect(btn.text()).toContain("Restore");
    await btn.trigger("click");
    await nextTick();
    btn = wrapper.find('[data-test="delete-dim"]');
    expect(btn.text()).toContain("Delete dimension");
  });

  it("shows placeholder when no dimension is selected", () => {
    const wrapper = mountModal({
      open: true,
      dimensions: [],
    });
    expect(wrapper.text()).toContain("Select a dimension to edit");
  });
});
