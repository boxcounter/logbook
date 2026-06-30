import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import DimensionEditorModal from "../../../components/composite/DimensionEditorModal.vue";
import type { Dimension } from "../../../types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const MOCK_DIMENSIONS: Dimension[] = [
  { name: "Goal", key: "goal", source: "monthly", values: undefined, required: false, deleted: false },
  { name: "Biz", key: "biz", source: "static", values: ["Product", "Marketing", "Engineering"], required: true, deleted: false },
  { name: "Importance", key: "importance-urgency", source: "static", values: ["P0", "P1"], required: false, deleted: false },
];

beforeEach(() => {
  (invoke as any).mockReset?.();
  (invoke as any).mockResolvedValue?.(MOCK_DIMENSIONS);
});

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
    const checkbox = wrapper.find('[data-test="required-checkbox"]');
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

  it("emits saved with updated dimensions on Save", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    const nameInput = wrapper.find('input[placeholder="Dimension name"]');
    await nameInput.setValue("Business Goal");
    await wrapper.find('[data-test="save"]').trigger("click");
    await nextTick();

    expect(invoke).toHaveBeenCalledWith("save_dimensions", expect.objectContaining({
      rootPath: "/test",
      year: 2026,
      month: 6,
      dimensions: expect.arrayContaining([
        expect.objectContaining({ name: "Business Goal", key: "goal" }),
      ]),
    }));
    expect(wrapper.emitted("saved")).toBeTruthy();
  });

  it("shows discard confirmation when dirty and Cancel is clicked", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    // Make dirty by changing name
    const nameInput = wrapper.find('input[placeholder="Dimension name"]');
    await nameInput.setValue("Business Goal");
    // Click Cancel — should show discard confirmation
    await wrapper.find('[data-test="cancel"]').trigger("click");
    await nextTick();
    expect(wrapper.find('[data-test="discard-confirm"]').exists()).toBe(true);
  });

  it("does not show discard confirmation when not dirty", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    await wrapper.find('[data-test="cancel"]').trigger("click");
    await nextTick();
    // No discard confirm — emits close directly
    expect(wrapper.find('[data-test="discard-confirm"]').exists()).toBe(false);
    expect(wrapper.emitted("close")).toBeTruthy();
  });

  it("discard confirmation Keep editing returns to editor", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    const nameInput = wrapper.find('input[placeholder="Dimension name"]');
    await nameInput.setValue("Business Goal");
    await wrapper.find('[data-test="cancel"]').trigger("click");
    await nextTick();
    // Click "Keep editing" to dismiss discard overlay
    await wrapper.find('[data-test="keep-editing"]').trigger("click");
    await nextTick();
    expect(wrapper.find('[data-test="discard-confirm"]').exists()).toBe(false);
    expect(wrapper.find('[data-test="overlay"]').exists()).toBe(true); // still open
  });

  it("discard confirmation Discard closes modal", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    const nameInput = wrapper.find('input[placeholder="Dimension name"]');
    await nameInput.setValue("Business Goal");
    await wrapper.find('[data-test="cancel"]').trigger("click");
    await nextTick();
    await wrapper.find('[data-test="discard-yes"]').trigger("click");
    await nextTick();
    expect(wrapper.emitted("close")).toBeTruthy();
  });

  it("Cmd+Enter triggers save", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    const overlay = wrapper.find('[data-test="overlay"]');
    await overlay.trigger("keydown", { key: "Enter", metaKey: true });
    await nextTick();
    expect(invoke).toHaveBeenCalledWith("save_dimensions", expect.anything());
  });

  it("Ctrl+Enter triggers save", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    const overlay = wrapper.find('[data-test="overlay"]');
    await overlay.trigger("keydown", { key: "Enter", ctrlKey: true });
    await nextTick();
    expect(invoke).toHaveBeenCalledWith("save_dimensions", expect.anything());
  });

  it("shows save-as-template link in header", () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    expect(wrapper.find('[data-test="save-as-template"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("Save as template");
  });

  it("saveAsTemplate invokes save_dimensions_template", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    await wrapper.find('[data-test="save-as-template"]').trigger("click");
    await nextTick();
    expect(invoke).toHaveBeenCalledWith("save_dimensions_template", expect.objectContaining({
      rootPath: "/test",
      dimensions: MOCK_DIMENSIONS,
    }));
  });

  it("shows error when save fails", async () => {
    (invoke as any).mockRejectedValue(new Error("Validation failed: duplicate key"));
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    await wrapper.find('[data-test="save"]').trigger("click");
    await nextTick();
    await nextTick(); // extra tick for promise rejection
    expect(wrapper.find('[data-test="save-error"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("Validation failed");
  });

  // ── Add dimension form ──────────────────────────────────────────

  it("shows + Add dimension button in left panel", () => {
    const wrapper = mountModal();
    expect(wrapper.find('[data-test="add-dim-btn"]').exists()).toBe(true);
    expect(wrapper.find('[data-test="add-dim-btn"]').text()).toContain("Add dimension");
  });

  it("clicking + Add dimension reveals the inline form", async () => {
    const wrapper = mountModal();
    await wrapper.find('[data-test="add-dim-btn"]').trigger("click");
    expect(wrapper.find('[data-test="add-dim-form"]').exists()).toBe(true);
  });

  it("cancel hides the add form and clears inputs", async () => {
    const wrapper = mountModal();
    await wrapper.find('[data-test="add-dim-btn"]').trigger("click");
    await wrapper.find('[data-test="add-dim-name"]').setValue("Test");
    await wrapper.find('[data-test="add-dim-key"]').setValue("test");
    await wrapper.find('[data-test="add-dim-cancel"]').trigger("click");
    expect(wrapper.find('[data-test="add-dim-form"]').exists()).toBe(false);
    // Re-open: inputs should be cleared
    await wrapper.find('[data-test="add-dim-btn"]').trigger("click");
    expect((wrapper.find('[data-test="add-dim-name"]').element as HTMLInputElement).value).toBe("");
    expect((wrapper.find('[data-test="add-dim-key"]').element as HTMLInputElement).value).toBe("");
  });

  it("add form has Name input, Key input, and Source dropdown", async () => {
    const wrapper = mountModal();
    await wrapper.find('[data-test="add-dim-btn"]').trigger("click");
    expect(wrapper.find('[data-test="add-dim-name"]').exists()).toBe(true);
    expect(wrapper.find('[data-test="add-dim-key"]').exists()).toBe(true);
    expect(wrapper.find('[data-test="add-dim-source"]').exists()).toBe(true);
  });

  it("create adds a new dimension to the draft", async () => {
    const wrapper = mountModal();
    const initialRows = wrapper.findAll('[data-test="dim-row"]').length;
    await wrapper.find('[data-test="add-dim-btn"]').trigger("click");
    await wrapper.find('[data-test="add-dim-name"]').setValue("New Dim");
    await wrapper.find('[data-test="add-dim-key"]').setValue("new-dim");
    await wrapper.find('[data-test="add-dim-create"]').trigger("click");
    const rows = wrapper.findAll('[data-test="dim-row"]');
    expect(rows).toHaveLength(initialRows + 1);
    expect(wrapper.text()).toContain("New Dim");
  });

  it("selects the newly created dimension", async () => {
    const wrapper = mountModal();
    await wrapper.find('[data-test="add-dim-btn"]').trigger("click");
    await wrapper.find('[data-test="add-dim-name"]').setValue("Selected New");
    await wrapper.find('[data-test="add-dim-key"]').setValue("selected-new");
    await wrapper.find('[data-test="add-dim-create"]').trigger("click");
    const nameInput = wrapper.find('input[placeholder="Dimension name"]');
    expect((nameInput.element as HTMLInputElement).value).toBe("Selected New");
  });

  it("shows error for empty key", async () => {
    const wrapper = mountModal();
    await wrapper.find('[data-test="add-dim-btn"]').trigger("click");
    await wrapper.find('[data-test="add-dim-name"]').setValue("New Dim");
    // Leave key empty
    await wrapper.find('[data-test="add-dim-create"]').trigger("click");
    expect(wrapper.find('[data-test="add-dim-error"]').exists()).toBe(true);
    expect(wrapper.find('[data-test="add-dim-error"]').text()).toContain("Key is required");
  });

  it("shows error for invalid key characters", async () => {
    const wrapper = mountModal();
    await wrapper.find('[data-test="add-dim-btn"]').trigger("click");
    await wrapper.find('[data-test="add-dim-name"]').setValue("New Dim");
    await wrapper.find('[data-test="add-dim-key"]').setValue("bad key!");
    await wrapper.find('[data-test="add-dim-create"]').trigger("click");
    expect(wrapper.find('[data-test="add-dim-error"]').text()).toContain("Only letters, numbers, hyphens, and underscores allowed");
  });

  it("shows error for duplicate key", async () => {
    const wrapper = mountModal();
    await wrapper.find('[data-test="add-dim-btn"]').trigger("click");
    await wrapper.find('[data-test="add-dim-name"]').setValue("Another Goal");
    await wrapper.find('[data-test="add-dim-key"]').setValue("goal"); // duplicate of MOCK_DIMENSIONS[0].key
    await wrapper.find('[data-test="add-dim-create"]').trigger("click");
    expect(wrapper.find('[data-test="add-dim-error"]').text()).toContain("Key 'goal' already exists.");
  });

  it("shows special message for duplicate of a soft-deleted key", async () => {
    const dimsWithDeleted: Dimension[] = [
      { name: "Old", key: "old-dim", source: "static", values: [], required: false, deleted: true },
      { name: "Goal", key: "goal", source: "monthly", values: undefined, required: false, deleted: false },
    ];
    const wrapper = mountModal({ open: true, dimensions: dimsWithDeleted });
    await wrapper.find('[data-test="add-dim-btn"]').trigger("click");
    await wrapper.find('[data-test="add-dim-name"]').setValue("New Old");
    await wrapper.find('[data-test="add-dim-key"]').setValue("old-dim"); // deleted duplicate
    await wrapper.find('[data-test="add-dim-create"]').trigger("click");
    expect(wrapper.find('[data-test="add-dim-error"]').text()).toContain("already exists (deleted)");
    expect(wrapper.find('[data-test="add-dim-error"]').text()).toContain("Restore it or choose a different key");
  });

  it("prevents creating a second monthly-source dimension", async () => {
    const wrapper = mountModal(); // Goal (index 0) is monthly
    await wrapper.find('[data-test="add-dim-btn"]').trigger("click");
    await wrapper.find('[data-test="add-dim-name"]').setValue("Second Monthly");
    await wrapper.find('[data-test="add-dim-key"]').setValue("monthly2");
    await wrapper.find('[data-test="add-dim-source"]').setValue("monthly");
    await wrapper.find('[data-test="add-dim-create"]').trigger("click");
    expect(wrapper.find('[data-test="add-dim-error"]').text()).toContain("Only one monthly-source dimension allowed");
  });

  it("resets form after successful create", async () => {
    const wrapper = mountModal();
    await wrapper.find('[data-test="add-dim-btn"]').trigger("click");
    await wrapper.find('[data-test="add-dim-name"]').setValue("New Dim");
    await wrapper.find('[data-test="add-dim-key"]').setValue("new-dim");
    await wrapper.find('[data-test="add-dim-create"]').trigger("click");
    // Form should be hidden; inputs should be cleared
    expect(wrapper.find('[data-test="add-dim-form"]').exists()).toBe(false);
    await wrapper.find('[data-test="add-dim-btn"]').trigger("click");
    expect((wrapper.find('[data-test="add-dim-name"]').element as HTMLInputElement).value).toBe("");
    expect((wrapper.find('[data-test="add-dim-key"]').element as HTMLInputElement).value).toBe("");
    expect((wrapper.find('[data-test="add-dim-source"]').element as HTMLSelectElement).value).toBe("static");
  });

  it("add form uses the specified styling", async () => {
    const wrapper = mountModal();
    await wrapper.find('[data-test="add-dim-btn"]').trigger("click");
    const form = wrapper.find('[data-test="add-dim-form"]');
    expect(form.classes()).toContain("border-[var(--color-brand-solid)]");
    expect(form.classes()).toContain("rounded-[var(--radius-form-lg)]");
    expect(form.classes()).toContain("bg-[var(--color-brand-soft-bg)]");
  });

  // ── Show deleted toggle ─────────────────────────────────────────

  it("does not show deleted toggle when no dimensions are deleted", () => {
    const wrapper = mountModal();
    expect(wrapper.find('[data-test="show-deleted-toggle"]').exists()).toBe(false);
  });

  it("shows deleted toggle when a dimension is soft-deleted", async () => {
    const wrapper = mountModal();
    // Delete Goal (index 0, selected by default)
    await wrapper.find('[data-test="delete-dim"]').trigger("click");
    await nextTick();
    expect(wrapper.find('[data-test="show-deleted-toggle"]').exists()).toBe(true);
  });

  it("defaults toggle off — deleted dimensions hidden from list", async () => {
    const wrapper = mountModal();
    await wrapper.find('[data-test="delete-dim"]').trigger("click");
    await nextTick();
    const rows = wrapper.findAll('[data-test="dim-row"]');
    expect(rows).toHaveLength(2); // Goal hidden, Biz + Importance remain
    expect(wrapper.text()).not.toContain("Goal");
  });

  it("toggle on shows deleted dimensions with opacity-40", async () => {
    const wrapper = mountModal();
    await wrapper.find('[data-test="delete-dim"]').trigger("click");
    await nextTick();
    // Turn toggle on — find the checkbox input inside the label
    const toggleCheckbox = wrapper.find('[data-test="show-deleted-toggle"] input[type="checkbox"]');
    await toggleCheckbox.setValue(true);
    await nextTick();
    const rows = wrapper.findAll('[data-test="dim-row"]');
    expect(rows).toHaveLength(3);
    // Find the deleted Goal row — it should have opacity-40
    const goalRow = rows[0]; // Goal is first in draft order
    expect(goalRow.classes()).toContain("opacity-40");
  });

  // ── Read-only right panel for deleted dimensions ─────────────────

  it("disables name input when selected dimension is deleted", async () => {
    const wrapper = mountModal();
    // Delete Goal (index 0, selected by default)
    await wrapper.find('[data-test="delete-dim"]').trigger("click");
    await nextTick();
    const nameInput = wrapper.find('input[placeholder="Dimension name"]');
    expect((nameInput.element as HTMLInputElement).disabled).toBe(true);
  });

  it("disables required checkbox when selected dimension is deleted", async () => {
    const wrapper = mountModal();
    await wrapper.find('[data-test="delete-dim"]').trigger("click");
    await nextTick();
    const checkbox = wrapper.find('[data-test="required-checkbox"]');
    expect((checkbox.element as HTMLInputElement).disabled).toBe(true);
  });

  it("hides add-value section when selected dimension is deleted", async () => {
    // Use Biz (index 1, static with values)
    const wrapper = mountModal();
    const bizRow = wrapper.findAll('[data-test="dim-row"]')[1];
    await bizRow.trigger("click");
    // First verify add-value area exists
    expect(wrapper.find('[data-test="add-value"]').exists()).toBe(true);
    // Delete Biz
    await wrapper.find('[data-test="delete-dim"]').trigger("click");
    await nextTick();
    // Add-value area and new value input should be gone
    expect(wrapper.find('[data-test="add-value"]').exists()).toBe(false);
    expect(wrapper.find('input[placeholder="New value"]').exists()).toBe(false);
  });

  it("shows Restore button with brand-link color when deleted", async () => {
    const wrapper = mountModal();
    await wrapper.find('[data-test="delete-dim"]').trigger("click");
    await nextTick();
    const btn = wrapper.find('[data-test="delete-dim"]');
    expect(btn.text()).toContain("Restore");
    expect(btn.classes()).toContain("text-[var(--color-brand-link)]");
  });

  it("restore button undoes deletion", async () => {
    const wrapper = mountModal();
    // Delete
    await wrapper.find('[data-test="delete-dim"]').trigger("click");
    await nextTick();
    // Restore
    await wrapper.find('[data-test="delete-dim"]').trigger("click");
    await nextTick();
    const btn = wrapper.find('[data-test="delete-dim"]');
    expect(btn.text()).toContain("Delete dimension");
    expect(btn.classes()).toContain("text-[var(--color-text-disabled)]");
    // Name input should be enabled again
    const nameInput = wrapper.find('input[placeholder="Dimension name"]');
    expect((nameInput.element as HTMLInputElement).disabled).toBe(false);
  });
});
