import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import Toast from "../../../components/base/Toast.vue";

function mountToast(props: { show: boolean; message: string; undoLabel?: string }) {
  return mount(Toast, {
    props,
    global: { stubs: { Teleport: true } },
  });
}

describe("Toast", () => {
  it("renders when show is true", () => {
    const wrapper = mountToast({ show: true, message: "Done" });
    expect(wrapper.text()).toContain("Done");
  });

  it("does not render when show is false", () => {
    const wrapper = mountToast({ show: false, message: "Done" });
    expect(wrapper.find("div.fixed").exists()).toBe(false);
  });

  it("shows undo button when undoLabel is provided", () => {
    const wrapper = mountToast({ show: true, message: "Deleted", undoLabel: "Undo" });
    expect(wrapper.text()).toContain("Undo");
  });

  it("emits undo on button click", async () => {
    const wrapper = mountToast({ show: true, message: "X", undoLabel: "Undo" });
    await wrapper.find("button.font-semibold").trigger("click");
    expect(wrapper.emitted("undo")).toHaveLength(1);
  });

  it("emits dismiss on close click", async () => {
    const wrapper = mountToast({ show: true, message: "X" });
    const buttons = wrapper.findAll("button");
    const closeBtn = buttons[buttons.length - 1];
    await closeBtn.trigger("click");
    expect(wrapper.emitted("dismiss")).toHaveLength(1);
  });
});
