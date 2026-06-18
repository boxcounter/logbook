import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import AppChip from "../../../components/base/AppChip.vue";

describe("AppChip", () => {
  it("renders label and value", () => {
    const wrapper = mount(AppChip, { props: { label: "Goal", value: "Sprint" } });
    expect(wrapper.text()).toContain("Goal");
    expect(wrapper.text()).toContain("Sprint");
  });

  it("applies category color", () => {
    const wrapper = mount(AppChip, { props: { label: "C", value: "v", color: "category" } });
    const span = wrapper.find("span");
    expect(span.classes().join(" ")).toContain("bg-[var(--color-chip-category-bg)]");
  });

  it("shows close icon when closable", () => {
    const wrapper = mount(AppChip, { props: { label: "G", value: "v", closable: true } });
    expect(wrapper.text()).toContain("×");
  });

  it("hides close icon when not closable", () => {
    const wrapper = mount(AppChip, { props: { label: "G", value: "v", closable: false } });
    expect(wrapper.text()).not.toContain("×");
  });

  it("emits close on click when closable", async () => {
    const wrapper = mount(AppChip, { props: { label: "G", value: "v", closable: true } });
    await wrapper.trigger("click");
    expect(wrapper.emitted("close")).toHaveLength(1);
  });

  it("missing color uses dashed border", () => {
    const wrapper = mount(AppChip, { props: { label: "X", value: "?", color: "missing" } });
    const classes = wrapper.find("span").classes().join(" ");
    expect(classes).toContain("border-dashed");
  });
});
