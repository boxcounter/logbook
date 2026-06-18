import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import Popover from "../../../components/base/Popover.vue";

describe("Popover", () => {
  it("renders trigger slot content", () => {
    const wrapper = mount(Popover, {
      slots: { trigger: '<span class="trigger-text">Open</span>' },
    });
    expect(wrapper.text()).toContain("Open");
  });

  it("renders default slot content when open", async () => {
    const wrapper = mount(Popover, {
      attachTo: document.body,
      slots: { default: '<div class="popover-inner">Content</div>' },
    });
    // PopoverContent renders conditionally; open it first
    await wrapper.find("button").trigger("click");
    // Content is teleported to document.body
    expect(document.body.innerHTML).toContain("Content");
  });
});
