import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import AppSelect from "../../../components/base/AppSelect.vue";

const opts = [
  { value: "eng", label: "Engineering" },
  { value: "design", label: "Design" },
];

describe("AppSelect", () => {
  it("renders options", () => {
    const wrapper = mount(AppSelect, { props: { options: opts } });
    expect(wrapper.text()).toContain("Engineering");
    expect(wrapper.text()).toContain("Design");
  });

  it("emits update:modelValue on select", async () => {
    const wrapper = mount(AppSelect, { props: { options: opts } });
    const root = wrapper.findComponent({ name: "ListboxRoot" });
    await root.vm.$emit("update:modelValue", "design");
    expect(wrapper.emitted("update:modelValue")?.[0]).toEqual(["design"]);
  });
});
