import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import AppInput from "../../../components/base/AppInput.vue";

describe("AppInput", () => {
  it("renders with placeholder", () => {
    const wrapper = mount(AppInput, { props: { placeholder: "Type here" } });
    expect(wrapper.find("input").attributes("placeholder")).toBe("Type here");
  });

  it("v-model binding", async () => {
    const wrapper = mount(AppInput, { props: { modelValue: "hello" } });
    expect((wrapper.find("input").element as HTMLInputElement).value).toBe("hello");
  });

  it("emits update:modelValue on input", async () => {
    const wrapper = mount(AppInput);
    const input = wrapper.find("input");
    await input.setValue("new value");
    expect(wrapper.emitted("update:modelValue")?.[0]).toEqual(["new value"]);
  });

  it("has focus ring classes", () => {
    const wrapper = mount(AppInput);
    const classes = wrapper.find("input").classes();
    expect(classes).toContain("rounded-full");
    expect(classes.join(" ")).toContain("border-[var(--color-border-form)]");
  });
});
