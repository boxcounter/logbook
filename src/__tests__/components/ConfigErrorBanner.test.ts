import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import { STORE_KEY } from "../../stores/useStore";
import { createTestStore } from "../mocks/store";
import ConfigErrorBanner from "../../components/ConfigErrorBanner.vue";
import { makeConfigErrors } from "../mocks/fixtures";

describe("ConfigErrorBanner", () => {
  it("renders error count and list", () => {
    const errors = makeConfigErrors();
    const store = createTestStore({ configErrors: errors });
    const wrapper = mount(ConfigErrorBanner, {
      global: {
        provide: {
          [STORE_KEY as symbol]: store,
        },
      },
    });

    expect(wrapper.text()).toContain("Configuration Errors (2)");
    expect(wrapper.text()).toContain("MissingName");
    expect(wrapper.text()).toContain("MissingKey");
  });

  it("renders instruction text", () => {
    const store = createTestStore({ configErrors: makeConfigErrors() });
    const wrapper = mount(ConfigErrorBanner, {
      global: {
        provide: {
          [STORE_KEY as symbol]: store,
        },
      },
    });

    expect(wrapper.text()).toContain("config.yaml");
  });

  it('shows "Configuration Errors (0)" when no errors', () => {
    const store = createTestStore({ configErrors: [] });
    const wrapper = mount(ConfigErrorBanner, {
      global: {
        provide: {
          [STORE_KEY as symbol]: store,
        },
      },
    });

    expect(wrapper.text()).toContain("Configuration Errors (0)");
  });

  it("renders single error without count confusion", () => {
    const store = createTestStore({
      configErrors: [{ kind: "MissingName", message: "Dimension 0 has an empty name" }],
    });
    const wrapper = mount(ConfigErrorBanner, {
      global: {
        provide: {
          [STORE_KEY as symbol]: store,
        },
      },
    });

    expect(wrapper.text()).toContain("Configuration Errors (1)");
    expect(wrapper.text()).toContain("MissingName");
  });
});
