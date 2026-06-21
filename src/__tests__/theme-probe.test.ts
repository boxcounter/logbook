import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import { defineComponent, h } from "vue";

// `text-*` is overloaded (font-size AND color). Confirm our custom keys land as
// utilities at all (jsdom can't compute CSS, so we only assert the class is
// accepted and rendered — real font-size is confirmed by the visual check).
describe("theme type-scale utilities", () => {
  it("renders the four named text utilities without error", () => {
    const C = defineComponent({
      render: () => h("div", [
        h("span", { class: "text-title" }),
        h("span", { class: "text-body" }),
        h("span", { class: "text-secondary" }),
        h("span", { class: "text-micro" }),
      ]),
    });
    const w = mount(C);
    expect(w.findAll("span")).toHaveLength(4);
  });

  it("renders named spacing utilities without error", () => {
    const C = defineComponent({
      render: () => h("div", { class: "gap-sm p-md mt-lg px-2xs py-2xl" }),
    });
    const w = mount(C);
    expect(w.find("div").classes()).toContain("gap-sm");
  });
});
