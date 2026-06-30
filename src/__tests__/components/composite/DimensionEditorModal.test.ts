import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
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
});
