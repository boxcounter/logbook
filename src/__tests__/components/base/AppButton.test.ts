import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import AppButton from "../../../components/base/AppButton.vue";

describe("AppButton", () => {
  it("renders default primary button", () => {
    const wrapper = mount(AppButton, { slots: { default: "Click" } });
    expect(wrapper.text()).toBe("Click");
    expect(wrapper.attributes("disabled")).toBeUndefined();
  });

  it("emits click event", async () => {
    const wrapper = mount(AppButton);
    await wrapper.trigger("click");
    expect(wrapper.emitted("click")).toHaveLength(1);
  });

  it("does not emit click when disabled", async () => {
    const wrapper = mount(AppButton, { props: { disabled: true } });
    await wrapper.trigger("click");
    expect(wrapper.emitted("click")).toBeUndefined();
    expect(wrapper.attributes("disabled")).toBeDefined();
  });

  it("applies size sm class", () => {
    const wrapper = mount(AppButton, { props: { size: "sm" } });
    expect(wrapper.classes()).toContain("text-[13px]");
  });

  it("applies variant classes", () => {
    const outline = mount(AppButton, { props: { variant: "outline" } });
    expect(outline.classes().join(" ")).toContain("border-2");

    const danger = mount(AppButton, { props: { variant: "danger" } });
    expect(danger.classes().join(" ")).toContain("bg-red-50");
  });

  it("renders slot content", () => {
    const wrapper = mount(AppButton, {
      slots: { default: '<span data-testid="inner">Save</span>' },
    });
    expect(wrapper.text()).toBe("Save");
  });
});
