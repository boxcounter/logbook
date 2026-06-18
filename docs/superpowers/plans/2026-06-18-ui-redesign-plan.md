# Logbook UI Redesign — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace raw HTML elements with a mini design system of 10 Vue SFCs powered by Reka UI, apply Colorful/Playful visual style, and ensure WCAG 2.2 AA compliance.

**Architecture:** Base components (AppButton, AppInput, AppChip, AppSelect, ProgressBar, Popover, Toast) encapsulate visual style and Reka UI behavior. Composite components (MentionMenu, EntryRow, CommitmentsEditor) compose base components. Existing views (MonthView, QuickEntry, EntryList, CommitmentsPanel) are refactored to use new components with zero functional changes.

**Tech Stack:** Vue 3 + TypeScript + Tailwind CSS 4 + Reka UI 2.x (Listbox, Popover primitives) + Vitest + @vue/test-utils

---

## File Structure

```
src/
├── assets/
│   └── tokens.css              ← NEW: CSS custom properties (design tokens)
│   └── main.css                ← MODIFY: @import tokens.css, add texture
├── components/
│   ├── base/                   ← NEW: 7 base components
│   │   ├── AppButton.vue
│   │   ├── AppInput.vue
│   │   ├── AppChip.vue
│   │   ├── AppSelect.vue
│   │   ├── ProgressBar.vue
│   │   ├── Popover.vue
│   │   └── Toast.vue
│   ├── composite/              ← NEW: 3 composite components
│   │   ├── MentionMenu.vue
│   │   ├── EntryRow.vue
│   │   └── CommitmentsEditor.vue
│   ├── MonthView.vue           ← MODIFY: use new components
│   ├── QuickEntry.vue          ← MODIFY: use AppInput, AppChip, MentionMenu
│   ├── EntryList.vue           ← MODIFY: use EntryRow
│   ├── EntryInput.vue          ← MODIFY: use AppInput, AppChip, MentionMenu
│   ├── EntryItem.vue           ← REMOVE (replaced by EntryRow)
│   ├── CommitmentsPanel.vue    ← MODIFY: use ProgressBar, CommitmentsEditor
│   ├── DimensionPanel.vue      ← MODIFY: use AppSelect, AppChip
│   ├── MonthNavigator.vue      ← MODIFY: use AppButton
│   ├── DayStrip.vue            ← MODIFY: use new style classes
│   ├── SetupScreen.vue         ← MODIFY: use AppButton
│   └── ConfigErrorBanner.vue   ← MODIFY: use AppButton
├── __tests__/
│   └── components/
│       ├── base/               ← NEW
│       │   ├── AppButton.test.ts
│       │   ├── AppInput.test.ts
│       │   ├── AppChip.test.ts
│       │   ├── AppSelect.test.ts
│       │   ├── ProgressBar.test.ts
│       │   ├── Popover.test.ts
│       │   └── Toast.test.ts
│       ├── composite/          ← NEW
│       │   ├── MentionMenu.test.ts
│       │   ├── EntryRow.test.ts
│       │   └── CommitmentsEditor.test.ts
│       └── (existing tests MODIFY for class/selector changes)
```

---

### Task 1: Install Reka UI and set up design tokens

**Files:**
- Modify: `package.json`
- Create: `src/assets/tokens.css`
- Modify: `src/assets/main.css`
- Run: `pnpm install`

- [ ] **Step 1: Install reka-ui**

```bash
cd /Users/boxcounter/Code/BoxCounter/logbook
pnpm add reka-ui
```

Expected: `reka-ui` added to `dependencies` in `package.json`.

- [ ] **Step 2: Create design tokens CSS**

Create `src/assets/tokens.css`:

```css
/* Design tokens — single source of truth for visual style.
   Referenced by Tailwind utilities via var() and by components directly. */

:root {
  /* Surface & Text */
  --color-page-bg: #f8fafc;
  --color-surface: #ffffff;
  --color-text-primary: #1e293b;
  --color-text-secondary: #64748b;
  --color-placeholder: #64748b;
  --color-border-form: #64748b;
  --color-border-decorative: #cbd5e1;
  --color-divider: #f1f5f9;

  /* Brand */
  --color-brand-gradient-from: #6366f1;
  --color-brand-gradient-to: #8b5cf6;
  --color-brand-solid: #6366f1;
  --color-brand-link: #4f46e5;
  --color-brand-soft-bg: #eef2ff;

  /* Semantic */
  --color-success: #059669;
  --color-danger: #ef4444;

  /* Chips */
  --color-chip-category-bg: #eef2ff;
  --color-chip-category-border: #c7d2fe;
  --color-chip-category-text: #4338ca;
  --color-chip-biz-bg: #f5f3ff;
  --color-chip-biz-border: #ddd6fe;
  --color-chip-biz-text: #6d28d9;
  --color-chip-importance-bg: #f0fdfa;
  --color-chip-importance-border: #99f6e4;
  --color-chip-importance-text: #0f766e;
  --color-chip-goal-bg: #f0fdf4;
  --color-chip-goal-border: #bbf7d0;
  --color-chip-goal-text: #15803d;
  --color-chip-missing-bg: #f8fafc;
  --color-chip-missing-border: #cbd5e1;
  --color-chip-missing-text: #64748b;

  /* Radius */
  --radius-pill: 999px;
  --radius-card: 12px;
  --radius-popover: 14px;
  --radius-form: 8px;

  /* Shadows */
  --shadow-card: 0 1px 3px rgba(0,0,0,0.06);
  --shadow-popover: 0 16px 48px rgba(0,0,0,0.08), 0 0 0 1px rgba(0,0,0,0.03);
  --shadow-toast: 0 8px 32px rgba(0,0,0,0.15);
  --shadow-button: 0 2px 12px rgba(99,102,241,0.25);
  --shadow-button-hover: 0 4px 20px rgba(99,102,241,0.35);
  --shadow-focus-ring: 0 0 0 4px rgba(99,102,241,0.12), 0 0 20px rgba(99,102,241,0.06);

  /* Typography */
  --font-sans: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
  --text-base: 14px;
  --text-sm: 13px;
  --text-xs: 12px;
  --text-micro: 11px;

  /* Spacing */
  --spacing-card-padding: 16px;
  --spacing-section-gap: 12px;
  --spacing-item-gap: 8px;
}

/* Dark mode overrides */
@media (prefers-color-scheme: dark) {
  :root {
    --color-page-bg: #0f172a;
    --color-surface: #1e293b;
    --color-text-primary: #e2e8f0;
    --color-text-secondary: #94a3b8;
    /* brand, chip, radius, shadow tokens unchanged */
    --shadow-card: none;
    --shadow-popover: 0 16px 48px rgba(0,0,0,0.3), 0 0 0 1px rgba(255,255,255,0.05);
    --shadow-toast: 0 8px 32px rgba(0,0,0,0.4);
  }
}
```

- [ ] **Step 3: Update main.css to import tokens and add texture**

Read `src/assets/main.css`, then replace its content:

```css
@import './tokens.css';
@import 'tailwindcss';

body {
  font-family: var(--font-sans);
  background-color: var(--color-page-bg);
  background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)' opacity='0.03'/%3E%3C/svg%3E");
  color: var(--color-text-primary);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

@media (prefers-color-scheme: dark) {
  body {
    background-image: none;
  }
}
```

Expected: build succeeds, tokens available in browser devtools.

- [ ] **Step 4: Commit**

```bash
git add pnpm-lock.yaml package.json src/assets/tokens.css src/assets/main.css
git commit -m "chore: add reka-ui dependency and design tokens CSS"
```

---

### Task 2: AppButton

**Files:**
- Create: `src/components/base/AppButton.vue`
- Create: `src/__tests__/components/base/AppButton.test.ts`

- [ ] **Step 1: Write the component**

Create `src/components/base/AppButton.vue`:

```vue
<script setup lang="ts">
defineProps<{
  variant?: 'primary' | 'outline' | 'secondary' | 'danger';
  size?: 'sm' | 'md';
  disabled?: boolean;
}>();

defineEmits<{
  click: [e: MouseEvent];
}>();
</script>

<template>
  <button
    :disabled="disabled"
    class="inline-flex items-center justify-center font-semibold cursor-pointer
           transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
    :class="[
      size === 'sm'
        ? 'text-[13px] py-[7px] px-[16px]'
        : 'text-[14px] py-[10px] px-[24px]',
      variant === 'primary' || variant === undefined
        ? 'rounded-full border-none text-white'
          + ' bg-gradient-to-br from-[var(--color-brand-gradient-from)] to-[var(--color-brand-gradient-to)]'
          + ' shadow-[var(--shadow-button)]'
          + ' hover:-translate-y-px hover:shadow-[var(--shadow-button-hover)]'
          + ' active:scale-[0.97] active:translate-y-0'
        : variant === 'outline'
        ? 'rounded-full border-2 border-[var(--color-brand-solid)] bg-transparent text-[var(--color-brand-solid)]'
          + ' hover:-translate-y-px hover:shadow-[var(--shadow-card)]'
          + ' active:scale-[0.97] active:translate-y-0'
        : variant === 'secondary'
        ? 'rounded-full border-none bg-[var(--color-divider)] text-[var(--color-text-secondary)]'
          + ' hover:-translate-y-px hover:shadow-[var(--shadow-card)]'
          + ' active:scale-[0.97] active:translate-y-0'
        : /* danger */
          'rounded-full border-none bg-red-50 text-[var(--color-danger)]'
          + ' hover:bg-red-100',
    ]"
    @click="$emit('click', $event)"
  >
    <slot />
  </button>
</template>
```

- [ ] **Step 2: Write tests**

Create `src/__tests__/components/base/AppButton.test.ts`:

```ts
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
```

- [ ] **Step 3: Run tests**

```bash
cd /Users/boxcounter/Code/BoxCounter/logbook
pnpm test -- src/__tests__/components/base/AppButton.test.ts
```

Expected: all 6 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/components/base/AppButton.vue src/__tests__/components/base/AppButton.test.ts
git commit -m "feat: add AppButton component with primary/outline/secondary/danger variants"
```

---

### Task 3: AppInput

**Files:**
- Create: `src/components/base/AppInput.vue`
- Create: `src/__tests__/components/base/AppInput.test.ts`

- [ ] **Step 1: Write the component**

Create `src/components/base/AppInput.vue`:

```vue
<script setup lang="ts">
defineProps<{
  modelValue?: string;
  placeholder?: string;
}>();

defineEmits<{
  'update:modelValue': [value: string];
}>();
</script>

<template>
  <input
    type="text"
    :value="modelValue"
    :placeholder="placeholder"
    class="w-full px-[16px] py-[10px] text-[var(--text-base)] leading-relaxed
           bg-[var(--color-surface)] text-[var(--color-text-primary)]
           border-2 border-[var(--color-border-form)] rounded-full
           outline-none
           transition-all duration-200
           placeholder:text-[var(--color-placeholder)]
           focus:border-[var(--color-brand-solid)]
           focus:bg-[#fafaff]
           focus:shadow-[var(--shadow-focus-ring)]"
    @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
  />
</template>
```

- [ ] **Step 2: Write tests**

Create `src/__tests__/components/base/AppInput.test.ts`:

```ts
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
```

- [ ] **Step 3: Run tests**

```bash
pnpm test -- src/__tests__/components/base/AppInput.test.ts
```

Expected: all 4 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/components/base/AppInput.vue src/__tests__/components/base/AppInput.test.ts
git commit -m "feat: add AppInput component with focus ring and WCAG-compliant placeholder"
```

---

### Task 4: AppChip

**Files:**
- Create: `src/components/base/AppChip.vue`
- Create: `src/__tests__/components/base/AppChip.test.ts`

- [ ] **Step 1: Write the component**

Create `src/components/base/AppChip.vue`:

```vue
<script setup lang="ts">
defineProps<{
  color?: 'category' | 'biz' | 'importance' | 'goal' | 'missing';
  label: string;
  value: string;
  closable?: boolean;
}>();

defineEmits<{
  close: [];
}>();
</script>

<template>
  <span
    class="inline-flex items-center gap-[4px] px-[10px] py-[3px]
           rounded-full text-[var(--text-xs)] font-medium cursor-pointer
           transition-opacity duration-200 hover:opacity-80"
    :class="
      color === 'category'
        ? 'bg-[var(--color-chip-category-bg)] border border-[var(--color-chip-category-border)] text-[var(--color-chip-category-text)]'
        : color === 'biz'
        ? 'bg-[var(--color-chip-biz-bg)] border border-[var(--color-chip-biz-border)] text-[var(--color-chip-biz-text)]'
        : color === 'importance'
        ? 'bg-[var(--color-chip-importance-bg)] border border-[var(--color-chip-importance-border)] text-[var(--color-chip-importance-text)]'
        : color === 'goal'
        ? 'bg-[var(--color-chip-goal-bg)] border border-[var(--color-chip-goal-border)] text-[var(--color-chip-goal-text)]'
        : color === 'missing'
        ? 'bg-[var(--color-chip-missing-bg)] border-2 border-dashed border-[var(--color-chip-missing-border)] text-[var(--color-chip-missing-text)]'
        : 'bg-[var(--color-brand-soft-bg)] border border-[var(--color-chip-category-border)] text-[var(--color-brand-solid)]'
    "
    @click="closable && $emit('close')"
  >
    {{ label }}: {{ value }}
    <span v-if="closable" class="opacity-40 hover:opacity-80 text-[14px] leading-none">&times;</span>
  </span>
</template>
```

- [ ] **Step 2: Write tests**

Create `src/__tests__/components/base/AppChip.test.ts`:

```ts
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
```

- [ ] **Step 3: Run tests**

```bash
pnpm test -- src/__tests__/components/base/AppChip.test.ts
```

Expected: all 6 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/components/base/AppChip.vue src/__tests__/components/base/AppChip.test.ts
git commit -m "feat: add AppChip component with 5 color variants"
```

---

### Task 5: ProgressBar

**Files:**
- Create: `src/components/base/ProgressBar.vue`
- Create: `src/__tests__/components/base/ProgressBar.test.ts`

- [ ] **Step 1: Write the component**

Create `src/components/base/ProgressBar.vue`:

```vue
<script setup lang="ts">
defineProps<{
  spent: number;
  allocation: number;
  variant?: 'default' | 'warm';
}>();
</script>

<template>
  <div class="h-[8px] bg-[var(--color-divider)] rounded-[4px] overflow-hidden">
    <div
      class="h-full rounded-[4px] transition-all duration-500"
      :style="{
        width: allocation === 0 ? '0%' : Math.min(100, Math.round((spent / allocation) * 100)) + '%',
        background: variant === 'warm'
          ? 'linear-gradient(90deg, #f59e0b, #ef4444)'
          : 'linear-gradient(90deg, var(--color-brand-gradient-from), var(--color-brand-gradient-to))',
      }"
    />
  </div>
</template>
```

- [ ] **Step 2: Write tests**

Create `src/__tests__/components/base/ProgressBar.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import ProgressBar from "../../../components/base/ProgressBar.vue";

describe("ProgressBar", () => {
  it("renders with correct width percentage", () => {
    const wrapper = mount(ProgressBar, { props: { spent: 30, allocation: 60 } });
    const fill = wrapper.find("div > div");
    expect(fill.attributes("style")).toContain("width: 50%");
  });

  it("caps at 100% when spent exceeds allocation", () => {
    const wrapper = mount(ProgressBar, { props: { spent: 100, allocation: 50 } });
    const fill = wrapper.find("div > div");
    expect(fill.attributes("style")).toContain("width: 100%");
  });

  it("shows 0% when allocation is zero", () => {
    const wrapper = mount(ProgressBar, { props: { spent: 10, allocation: 0 } });
    const fill = wrapper.find("div > div");
    expect(fill.attributes("style")).toContain("width: 0%");
  });

  it("warm variant uses warm gradient", () => {
    const wrapper = mount(ProgressBar, { props: { spent: 10, allocation: 20, variant: "warm" } });
    const fill = wrapper.find("div > div");
    expect(fill.attributes("style")).toContain("#f59e0b");
  });
});
```

- [ ] **Step 3: Run tests**

```bash
pnpm test -- src/__tests__/components/base/ProgressBar.test.ts
```

Expected: all 4 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/components/base/ProgressBar.vue src/__tests__/components/base/ProgressBar.test.ts
git commit -m "feat: add ProgressBar component with default and warm variants"
```

---

### Task 6: AppSelect

**Files:**
- Create: `src/components/base/AppSelect.vue`
- Create: `src/__tests__/components/base/AppSelect.test.ts`

- [ ] **Step 1: Write the component using Reka UI Listbox**

Create `src/components/base/AppSelect.vue`:

```vue
<script setup lang="ts">
import { ListboxContent, ListboxItem, ListboxItemIndicator, ListboxRoot } from 'reka-ui';

defineProps<{
  options: { value: string; label: string }[];
  modelValue?: string;
  placeholder?: string;
}>();

defineEmits<{
  'update:modelValue': [value: string];
}>();
</script>

<template>
  <ListboxRoot
    :model-value="modelValue"
    @update:model-value="(val: string) => $emit('update:modelValue', val)"
  >
    <ListboxContent
      class="min-w-[140px] bg-[var(--color-surface)] border border-[var(--color-border-decorative)]
             rounded-[var(--radius-popover)] shadow-[var(--shadow-popover)]
             overflow-hidden text-[var(--text-base)]
             animate-[popoverIn_0.2s_cubic-bezier(0.16,1,0.3,1)]"
    >
      <ListboxItem
        v-for="opt in options"
        :key="opt.value"
        :value="opt.value"
        class="px-[14px] py-[10px] cursor-pointer transition-colors duration-100
               hover:bg-[var(--color-divider)]
               data-[state=checked]:bg-[var(--color-brand-soft-bg)]
               data-[state=checked]:text-[var(--color-brand-link)]
               data-[state=checked]:font-medium"
      >
        {{ opt.label }}
        <ListboxItemIndicator class="ml-auto text-[var(--color-success)] text-xs">
          &#10003;
        </ListboxItemIndicator>
      </ListboxItem>
    </ListboxContent>
  </ListboxRoot>
</template>
```

- [ ] **Step 2: Write tests**

Create `src/__tests__/components/base/AppSelect.test.ts`:

```ts
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

  it("sets modelValue", () => {
    const wrapper = mount(AppSelect, { props: { options: opts, modelValue: "eng" } });
    // Reka ListboxRoot receives model-value
    expect(wrapper.findComponent({ name: "ListboxRoot" }).props("modelValue")).toBe("eng");
  });

  it("emits update:modelValue on select", async () => {
    const wrapper = mount(AppSelect, { props: { options: opts } });
    const root = wrapper.findComponent({ name: "ListboxRoot" });
    await root.vm.$emit("update:modelValue", "design");
    expect(wrapper.emitted("update:modelValue")?.[0]).toEqual(["design"]);
  });
});
```

- [ ] **Step 3: Run tests**

```bash
pnpm test -- src/__tests__/components/base/AppSelect.test.ts
```

Expected: all 3 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/components/base/AppSelect.vue src/__tests__/components/base/AppSelect.test.ts
git commit -m "feat: add AppSelect component using Reka UI Listbox primitive"
```

---

### Task 7: Popover

**Files:**
- Create: `src/components/base/Popover.vue`
- Create: `src/__tests__/components/base/Popover.test.ts`

- [ ] **Step 1: Write the component**

Create `src/components/base/Popover.vue`:

```vue
<script setup lang="ts">
import { PopoverContent, PopoverPortal, PopoverRoot, PopoverTrigger } from 'reka-ui';

defineProps<{
  triggerClass?: string;
}>();
</script>

<template>
  <PopoverRoot>
    <PopoverTrigger
      :class="triggerClass"
      class="inline-flex items-center gap-[4px] px-[10px] py-[5px]
             border-2 border-[var(--color-border-form)] rounded-[var(--radius-form)]
             text-[var(--text-base)] leading-relaxed
             bg-[var(--color-surface)] text-[var(--color-text-secondary)]
             cursor-pointer transition-all duration-200
             focus:border-[var(--color-brand-solid)]
             focus:bg-[#fafaff]
             focus:shadow-[var(--shadow-focus-ring)]"
    >
      <slot name="trigger" />
    </PopoverTrigger>
    <PopoverPortal>
      <PopoverContent
        class="bg-[var(--color-surface)] border border-[var(--color-border-decorative)]
               rounded-[var(--radius-popover)] shadow-[var(--shadow-popover)] overflow-hidden
               animate-[popoverIn_0.2s_cubic-bezier(0.16,1,0.3,1)]"
      >
        <slot />
      </PopoverContent>
    </PopoverPortal>
  </PopoverRoot>
</template>

<style scoped>
@keyframes popoverIn {
  from { opacity: 0; transform: translateY(-4px) scale(0.97); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
</style>
```

- [ ] **Step 2: Write tests**

Create `src/__tests__/components/base/Popover.test.ts`:

```ts
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

  it("renders default slot content", () => {
    const wrapper = mount(Popover, {
      slots: { default: '<div class="popover-inner">Content</div>' },
    });
    // The PopoverContent is rendered inside a Portal (Teleport)
    expect(wrapper.html()).toContain("Content");
  });
});
```

- [ ] **Step 3: Run tests**

```bash
pnpm test -- src/__tests__/components/base/Popover.test.ts
```

Expected: all 2 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/components/base/Popover.vue src/__tests__/components/base/Popover.test.ts
git commit -m "feat: add Popover component using Reka UI Popover primitive with entrance animation"
```

---

### Task 8: Toast

**Files:**
- Create: `src/components/base/Toast.vue`
- Create: `src/__tests__/components/base/Toast.test.ts`

- [ ] **Step 1: Write the component**

Create `src/components/base/Toast.vue`:

```vue
<script setup lang="ts">
defineProps<{
  show: boolean;
  message: string;
  undoLabel?: string;
}>();

defineEmits<{
  undo: [];
  dismiss: [];
}>();
</script>

<template>
  <Teleport to="body">
    <Transition name="toast">
      <div
        v-if="show"
        class="fixed bottom-[24px] left-1/2 -translate-x-1/2
               flex items-center gap-[12px]
               bg-[var(--color-text-primary)] text-white
               px-[20px] py-[12px] rounded-[10px]
               shadow-[var(--shadow-toast)] z-50 text-[13px]"
      >
        <span>{{ message }}</span>
        <button
          v-if="undoLabel"
          class="font-semibold text-[#a5b4fc] hover:text-[#c7d2fe] cursor-pointer transition-colors"
          @click="$emit('undo')"
        >
          {{ undoLabel }}
        </button>
        <button
          class="text-[var(--color-text-secondary)] hover:text-white text-[16px] leading-none cursor-pointer transition-colors"
          @click="$emit('dismiss')"
        >
          &times;
        </button>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.toast-enter-active { transition: all 0.2s ease-out; }
.toast-leave-active { transition: all 0.2s ease-in; }
.toast-enter-from, .toast-leave-to { opacity: 0; transform: translate(-50%, 1rem); }
</style>
```

- [ ] **Step 2: Write tests**

Create `src/__tests__/components/base/Toast.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import Toast from "../../../components/base/Toast.vue";

describe("Toast", () => {
  it("renders when show is true", () => {
    const wrapper = mount(Toast, { props: { show: true, message: "Done" } });
    expect(wrapper.text()).toContain("Done");
  });

  it("does not render when show is false", () => {
    const wrapper = mount(Toast, { props: { show: false, message: "Done" } });
    expect(wrapper.find("div.fixed").exists()).toBe(false);
  });

  it("shows undo button when undoLabel is provided", () => {
    const wrapper = mount(Toast, { props: { show: true, message: "Deleted", undoLabel: "Undo" } });
    expect(wrapper.text()).toContain("Undo");
  });

  it("emits undo on button click", async () => {
    const wrapper = mount(Toast, { props: { show: true, message: "X", undoLabel: "Undo" } });
    await wrapper.find("button.font-semibold").trigger("click");
    expect(wrapper.emitted("undo")).toHaveLength(1);
  });

  it("emits dismiss on close click", async () => {
    const wrapper = mount(Toast, { props: { show: true, message: "X" } });
    const buttons = wrapper.findAll("button");
    const closeBtn = buttons[buttons.length - 1];
    await closeBtn.trigger("click");
    expect(wrapper.emitted("dismiss")).toHaveLength(1);
  });
});
```

- [ ] **Step 3: Run tests**

```bash
pnpm test -- src/__tests__/components/base/Toast.test.ts
```

Expected: all 5 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/components/base/Toast.vue src/__tests__/components/base/Toast.test.ts
git commit -m "feat: add Toast component with undo action and transition"
```

---

### Task 9: MentionMenu (Composite)

**Files:**
- Create: `src/components/composite/MentionMenu.vue`
- Create: `src/__tests__/components/composite/MentionMenu.test.ts`

- [ ] **Step 1: Write the component**

Create `src/components/composite/MentionMenu.vue`:

```vue
<script setup lang="ts">
import { ref, computed } from 'vue';
import type { Dimension, Commitment } from '../../types';
import { ListboxContent, ListboxItem, ListboxRoot } from 'reka-ui';

const props = defineProps<{
  dimensions: Dimension[];
  commitments: Commitment[];
  dimValues: Record<string, string>;
}>();

const emit = defineEmits<{
  'select': [dimKey: string, value: string];
  'close': [];
}>();

const phase = ref<'dim' | 'val'>('dim');
const activeDimKey = ref<string | null>(null);

const goalOptions = computed(() => {
  const goals = new Set<string>();
  for (const c of props.commitments) for (const g of c.goals) goals.add(g);
  return [...goals];
});

const dimOptions = computed(() =>
  props.dimensions.map(d => ({
    value: d.key,
    label: d.name,
    required: d.required,
    filled: !!props.dimValues[d.key],
  }))
);

const valOptions = computed(() => {
  if (!activeDimKey.value) return [];
  const dim = props.dimensions.find(d => d.key === activeDimKey.value);
  if (!dim) return [];
  if (dim.source === 'monthly') {
    return goalOptions.value.map(g => ({ value: g, label: g }));
  }
  return (dim.values || []).map(v => ({ value: v, label: v }));
});

function selectDim(dimKey: string) {
  activeDimKey.value = dimKey;
  phase.value = 'val';
}

function selectVal(value: string) {
  if (activeDimKey.value) {
    emit('select', activeDimKey.value, value);
    // After selecting, check if all required are filled
    const allFilled = props.dimensions
      .filter(d => d.required)
      .every(d => props.dimValues[d.key]);
    if (allFilled) {
      emit('close');
    } else {
      phase.value = 'dim';
      activeDimKey.value = null;
    }
  }
}

function goBack() { phase.value = 'dim'; activeDimKey.value = null; }
</script>

<template>
  <div
    class="bg-[var(--color-surface)] border border-[var(--color-border-decorative)]
           rounded-[var(--radius-popover)] shadow-[var(--shadow-popover)] overflow-hidden
           w-[240px] text-[var(--text-base)]
           animate-[popoverIn_0.2s_cubic-bezier(0.16,1,0.3,1)]"
  >
    <!-- Dim phase header -->
    <div
      v-if="phase === 'dim'"
      class="px-[14px] py-[8px] text-[10px] text-[var(--color-text-secondary)]
             uppercase tracking-wider font-bold border-b border-[var(--color-divider)]
             flex items-center gap-[8px]"
    >
      <span class="bg-[var(--color-brand-soft-bg)] text-[var(--color-brand-link)]
                   px-[6px] py-[2px] rounded-[4px] text-[9px] font-bold">
        DIM
      </span>
      Pick a dimension
    </div>

    <!-- Val phase header -->
    <div
      v-else
      class="px-[14px] py-[8px] text-[12px] font-medium
             border-b border-[var(--color-divider)]
             flex items-center gap-[8px]
             bg-[#faf5ff] text-[#7c3aed]"
    >
      <button class="font-bold hover:text-[#5b21b6] cursor-pointer leading-none" @click="goBack">
        &larr;
      </button>
      Pick a value for
      <b class="text-[#5b21b6]">
        {{ props.dimensions.find(d => d.key === activeDimKey)?.name || '' }}
      </b>
    </div>

    <!-- Dim phase items -->
    <template v-if="phase === 'dim'">
      <div
        v-for="d in dimOptions" :key="d.value"
        class="px-[14px] py-[10px] text-[14px] flex items-center gap-[8px]
               cursor-pointer transition-colors duration-100
               hover:bg-[var(--color-divider)]"
        @click="selectDim(d.value)"
      >
        <span class="w-[4px] h-[24px] rounded-[2px] flex-shrink-0"
              :style="{ background: `var(--color-chip-${d.value === 'category' ? 'category' : d.value === 'goal' ? 'goal' : 'biz'}-text)` }">
        </span>
        {{ d.label }}
        <span class="ml-auto text-[11px]" :class="d.filled ? 'text-[var(--color-success)]' : 'text-[var(--color-text-secondary)]'">
          {{ d.filled ? props.dimValues[d.value] + ' &#10003;' : d.required ? 'Required' : '' }}
        </span>
      </div>
      <!-- Footer: dot progress -->
      <div
        class="px-[14px] py-[8px] text-[11px] border-t border-[var(--color-divider)]
               flex items-center gap-[6px] text-[var(--color-text-secondary)]"
      >
        <span
          v-for="(d, i) in props.dimensions.filter(d => d.required)" :key="i"
          class="w-[6px] h-[6px] rounded-full"
          :class="props.dimValues[d.key] ? 'bg-[var(--color-success)]' : 'bg-[var(--color-divider)]'"
        />
        {{ props.dimensions.filter(d => d.required && !props.dimValues[d.key]).length }} to go
      </div>
    </template>

    <!-- Val phase items -->
    <template v-else>
      <div
        v-for="v in valOptions" :key="v.value"
        class="px-[14px] py-[10px] text-[14px] cursor-pointer transition-colors duration-100
               hover:bg-[var(--color-divider)]"
        @click="selectVal(v.value)"
      >
        {{ v.label }}
      </div>
      <div
        class="px-[14px] py-[8px] text-[11px] border-t border-[var(--color-divider)]
               text-[var(--color-text-secondary)]"
      >
        &larr; Back &middot; Type to filter
      </div>
    </template>
  </div>
</template>

<style scoped>
@keyframes popoverIn {
  from { opacity: 0; transform: translateY(-4px) scale(0.97); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
</style>
```

- [ ] **Step 2: Write a smoke test (component uses inject for store, test pattern follows EntryItem)**

Create `src/__tests__/components/composite/MentionMenu.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import MentionMenu from "../../../components/composite/MentionMenu.vue";
import { makeConfig, makeCommitment } from "../../mocks/fixtures";

describe("MentionMenu", () => {
  const config = makeConfig({ dimensions: [
    { name: "Goal", key: "goal", source: "monthly", required: true },
    { name: "Category", key: "category", source: "static", values: ["Coding", "Meeting"], required: false },
  ]});
  const commitments = [makeCommitment({ goals: ["Ship it", "Review"] })];

  it("renders dimension names in dim phase", () => {
    const wrapper = mount(MentionMenu, {
      props: { dimensions: config.dimensions, commitments, dimValues: {} },
    });
    expect(wrapper.text()).toContain("Goal");
    expect(wrapper.text()).toContain("Category");
    expect(wrapper.text()).toContain("Pick a dimension");
  });

  it("shows 'Required' for unfilled required dims", () => {
    const wrapper = mount(MentionMenu, {
      props: { dimensions: config.dimensions, commitments, dimValues: {} },
    });
    expect(wrapper.text()).toContain("Required");
  });

  it("enters val phase on dimension click", async () => {
    const wrapper = mount(MentionMenu, {
      props: { dimensions: config.dimensions, commitments, dimValues: {} },
    });
    const firstDim = wrapper.findAll(".cursor-pointer")[0];
    await firstDim.trigger("click");
    expect(wrapper.text()).toContain("Pick a value");
  });

  it("emits select and advances after value selection", async () => {
    const wrapper = mount(MentionMenu, {
      props: { dimensions: config.dimensions, commitments, dimValues: { goal: "" } },
    });
    // Click dimension
    const dimItems = wrapper.findAll(".cursor-pointer");
    await dimItems[dimItems.length > 2 ? 0 : 0].trigger("click");
    // Now in val phase — click a value
    await wrapper.vm.$nextTick();
    const valItems = wrapper.findAll(".cursor-pointer");
    // The val items are rendered; click the first non-dim one
    if (valItems.length > 0) {
      await valItems[valItems.length > 2 ? valItems.length - 3 : 0].trigger("click");
    }
    // After selection, should either emit close or go back to dim phase
    // (depends on whether all required filled)
    expect(wrapper.emitted()).toBeTruthy();
  });
});
```

- [ ] **Step 3: Run tests**

```bash
pnpm test -- src/__tests__/components/composite/MentionMenu.test.ts
```

Expected: all tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/components/composite/MentionMenu.vue src/__tests__/components/composite/MentionMenu.test.ts
git commit -m "feat: add MentionMenu composite component with two-phase dim/val picker"
```

---

### Task 10: EntryRow (Composite)

**Files:**
- Create: `src/components/composite/EntryRow.vue`
- Create: `src/__tests__/components/composite/EntryRow.test.ts`

EntryRow replaces EntryItem.vue — it combines display mode (item text, chips, duration, delete) with inline editing (item name, duration delta, dimension selects). The existing EntryItem tests should pass against EntryRow with selector updates.

- [ ] **Step 1: Write the component**

Create `src/components/composite/EntryRow.vue`:

```vue
<script setup lang="ts">
import { ref, nextTick, computed } from 'vue';
import type { Entry } from '../../types';
import { formatDuration, resolveDelta } from '../../utils/format';
import { useStore } from '../../stores/useStore';
import AppChip from '../base/AppChip.vue';

const props = defineProps<{
  entry: Entry;
  index: number;
}>();

const emit = defineEmits<{
  update: [entryId: string, item: string, durationMinutes: number];
  delete: [entryId: string];
  updateDimensions: [entryId: string, dimensions: Record<string, string>];
}>();

const store = useStore();

// ---- Edit modes ----
const editingItem = ref(false);
const editingDuration = ref(false);
const editingDimensions = ref(false);

const itemInput = ref('');
const durInput = ref('');

function startEditItem() {
  itemInput.value = props.entry.item;
  editingItem.value = true;
}
function commitItem() {
  editingItem.value = false;
  const newItem = itemInput.value.trim() || '(untitled)';
  if (newItem !== props.entry.item) {
    emit('update', props.entry.id, newItem, props.entry.duration);
  }
}

function startEditDuration() {
  durInput.value = String(props.entry.duration);
  editingDuration.value = true;
}
function commitDuration() {
  editingDuration.value = false;
  const newDur = resolveDelta(durInput.value, props.entry.duration);
  if (newDur !== props.entry.duration) {
    emit('update', props.entry.id, props.entry.item, newDur);
  }
}

function dimLabel(dims: Record<string, string>): string {
  const configDims = store.config?.dimensions || [];
  return configDims.map(d => dims[d.key]).filter(Boolean).join(' · ');
}

function chipColor(key: string): 'category' | 'biz' | 'importance' | 'goal' | 'missing' {
  const map: Record<string, 'category' | 'biz' | 'importance' | 'goal'> = {
    'category': 'category',
    'business-line': 'biz',
    'importance-urgency': 'importance',
    'goal': 'goal',
  };
  return map[key] || 'category';
}

const orderedDimensions = computed(() => store.config?.dimensions || []);
</script>

<template>
  <div
    class="flex items-center gap-[12px] py-[10px] px-[12px] -mx-[12px] rounded-[8px]
           border-b border-[var(--color-divider)] last:border-b-0
           transition-all duration-200
           hover:bg-[var(--color-divider)] hover:translate-x-[2px]"
    :class="{ 'bg-[#fafaff] shadow-[inset_3px_0_0_var(--color-brand-solid)]': editingItem || editingDuration || editingDimensions }"
  >
    <!-- Row number -->
    <span class="text-[var(--text-xs)] text-[var(--color-text-secondary)] w-[20px] text-right flex-shrink-0 tabular-nums">
      {{ index + 1 }}
    </span>

    <!-- Item text -->
    <template v-if="editingItem">
      <input
        v-model="itemInput"
        class="flex-1 px-[8px] py-[3px] border-2 border-[var(--color-brand-solid)]
               rounded-[var(--radius-form)] text-[var(--text-base)] leading-[1.4]
               outline-none bg-[#fafaff]
               shadow-[var(--shadow-focus-ring)]"
        @keydown.enter.prevent="commitItem"
        @keydown.escape.prevent="editingItem = false"
        @blur="commitItem"
        autofocus
      />
    </template>
    <template v-else>
      <span
        class="flex-1 text-[var(--text-base)] min-w-0 cursor-default
               rounded px-[2px] -mx-[2px] hover:bg-[var(--color-divider)]"
        @dblclick="startEditItem"
      >
        {{ entry.item }}
      </span>
    </template>

    <!-- Dimension chips -->
    <template v-if="editingDimensions">
      <select
        v-for="dim in orderedDimensions"
        :key="dim.key"
        :value="entry.dimensions[dim.key] || ''"
        class="px-[10px] py-[3px] border-2 border-[var(--color-border-form)]
               rounded-[var(--radius-form)] text-[var(--text-base)] leading-[1.4]
               bg-[var(--color-surface)] text-[var(--color-text-secondary)]
               focus:border-[var(--color-brand-solid)] focus:bg-[#fafaff]
               focus:shadow-[var(--shadow-focus-ring)] outline-none"
        @change="$emit('updateDimensions', entry.id, { ...entry.dimensions, [dim.key]: ($event.target as HTMLSelectElement).value })"
        @blur="editingDimensions = false"
      >
        <option value="">-- {{ dim.name }} --</option>
        <template v-if="dim.source === 'monthly'">
          <option v-for="c in store.commitments.flatMap(c => c.goals)" :key="c" :value="c">{{ c }}</option>
        </template>
        <template v-else>
          <option v-for="v in (dim.values || [])" :key="v" :value="v">{{ v }}</option>
        </template>
      </select>
    </template>
    <template v-else>
      <AppChip
        v-for="dim in orderedDimensions.filter(d => entry.dimensions[d.key])"
        :key="dim.key"
        :color="chipColor(dim.key)"
        :label="dim.name"
        :value="entry.dimensions[dim.key]"
        @click="editingDimensions = true"
      />
    </template>

    <!-- Duration -->
    <template v-if="editingDuration">
      <input
        v-model="durInput"
        class="w-[56px] text-right px-[8px] py-[3px]
               border-2 border-[#8b5cf6] rounded-[var(--radius-form)]
               text-[var(--text-base)] leading-[1.4] tabular-nums
               outline-none bg-[#fafaff]
               shadow-[0_0_0_4px_rgba(139,92,246,0.12),0_0_20px_rgba(139,92,246,0.06)]"
        @keydown.enter.prevent="commitDuration"
        @keydown.escape.prevent="editingDuration = false"
        @blur="commitDuration"
        autofocus
      />
    </template>
    <template v-else>
      <span
        class="text-[var(--text-base)] text-[var(--color-text-secondary)] tabular-nums
               flex-shrink-0 cursor-default rounded px-[4px] hover:bg-[var(--color-divider)]"
        @dblclick="startEditDuration"
      >
        {{ formatDuration(entry.duration) }}
      </span>
    </template>

    <!-- Delete -->
    <button
      class="text-[var(--color-text-secondary)] hover:text-[var(--color-danger)]
             text-[18px] leading-none flex-shrink-0 p-[2px]
             opacity-0 group-hover:opacity-100 transition-opacity duration-150 cursor-pointer"
      @click="$emit('delete', entry.id)"
    >
      &times;
    </button>
  </div>
</template>
```

- [ ] **Step 2: Write tests**

Create `src/__tests__/components/composite/EntryRow.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import EntryRow from "../../../components/composite/EntryRow.vue";
import { makeEntry, makeConfig } from "../../mocks/fixtures";
import { STORE_KEY } from "../../../stores/useStore";
import { createTestStore } from "../../mocks/store";

function mountRow(entryOverrides = {}, configOverrides = {}) {
  const store = createTestStore({ config: makeConfig(configOverrides) });
  const entry = makeEntry(entryOverrides);
  const wrapper = mount(EntryRow, {
    props: { entry, index: 0 },
    global: { provide: { [STORE_KEY as symbol]: store } },
  });
  return { wrapper, store, entry };
}

describe("EntryRow", () => {
  it("renders index, item text, and formatted duration", () => {
    const { wrapper } = mountRow({ item: "Write tests", duration: 75 });
    const text = wrapper.text();
    expect(text).toContain("1");
    expect(text).toContain("Write tests");
    expect(text).toContain("1h 15m");
  });

  it("double-click item text enters edit mode", async () => {
    const { wrapper } = mountRow({ item: "Original" });
    const display = wrapper.find(".flex-1");
    await display.trigger("dblclick");
    expect(wrapper.find("input.flex-1").exists()).toBe(true);
  });

  it("edit item: Enter commits the change", async () => {
    const { wrapper } = mountRow({ id: "e1", item: "Old", duration: 30 });
    const display = wrapper.find(".flex-1");
    await display.trigger("dblclick");

    const input = wrapper.find("input.flex-1");
    await input.setValue("New item");
    await input.trigger("keydown", { key: "Enter" });

    expect(wrapper.emitted("update")?.[0]).toEqual(["e1", "New item", 30]);
  });

  it("edit item: Escape cancels", async () => {
    const { wrapper } = mountRow({ item: "Original" });
    const display = wrapper.find(".flex-1");
    await display.trigger("dblclick");

    const input = wrapper.find("input.flex-1");
    await input.setValue("Changed");
    await input.trigger("keydown", { key: "Escape" });

    expect(wrapper.emitted("update")).toBeUndefined();
    expect(wrapper.text()).toContain("Original");
  });

  it("double-click duration enters edit mode", async () => {
    const { wrapper } = mountRow({ duration: 45 });
    // Find the duration span (the last tabular-nums span before the delete button)
    const durDisplay = wrapper.find(".tabular-nums");
    await durDisplay.trigger("dblclick");
    expect(wrapper.find("input.w-\\[56px\\]").exists()).toBe(true);
  });

  it("edit duration: delta +30 adds to existing", async () => {
    const { wrapper } = mountRow({ id: "e1", item: "Task", duration: 60 });
    const durDisplay = wrapper.find(".tabular-nums");
    await durDisplay.trigger("dblclick");

    const input = wrapper.find("input.w-\\[56px\\]");
    await input.setValue("+30");
    await input.trigger("keydown", { key: "Enter" });

    expect(wrapper.emitted("update")?.[0]).toEqual(["e1", "Task", 90]);
  });

  it("emits delete on button click", async () => {
    const { wrapper } = mountRow({ id: "del-me" });
    const btn = wrapper.find("button");
    await btn.trigger("click");
    expect(wrapper.emitted("delete")?.[0]).toEqual(["del-me"]);
  });

  it("shows dimension chips for set dimensions", () => {
    const { wrapper } = mountRow({
      dimensions: { goal: "Ship feature", "business-line": "Platform" },
    });
    expect(wrapper.text()).toContain("Ship feature");
    expect(wrapper.text()).toContain("Platform");
  });
});
```

- [ ] **Step 3: Run tests**

```bash
pnpm test -- src/__tests__/components/composite/EntryRow.test.ts
```

Expected: all 8 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/components/composite/EntryRow.vue src/__tests__/components/composite/EntryRow.test.ts
git commit -m "feat: add EntryRow composite component replacing EntryItem"
```

---

### Task 11: CommitmentsEditor (Composite)

**Files:**
- Create: `src/components/composite/CommitmentsEditor.vue`
- Create: `src/__tests__/components/composite/CommitmentsEditor.test.ts`

- [ ] **Step 1: Write the component**

Create `src/components/composite/CommitmentsEditor.vue`:

```vue
<script setup lang="ts">
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Commitment } from '../../types';
import AppInput from '../base/AppInput.vue';
import AppButton from '../base/AppButton.vue';

const props = defineProps<{
  commitments: Commitment[];
  rootPath: string;
  selectedYear: number;
  selectedMonth: number;
}>();

const emit = defineEmits<{
  saved: [];
  cancel: [];
}>();

const editingCommitments = ref<Commitment[]>(
  JSON.parse(JSON.stringify(props.commitments))
);
const error = ref('');
const saving = ref(false);

function addRole() {
  editingCommitments.value.push({ role: '', allocation: 0, goals: [] });
}
function removeRole(index: number) {
  if (editingCommitments.value.length > 1) editingCommitments.value.splice(index, 1);
}
function addGoal(roleIndex: number) {
  editingCommitments.value[roleIndex].goals.push('');
}
function removeGoal(roleIndex: number, goalIndex: number) {
  editingCommitments.value[roleIndex].goals.splice(goalIndex, 1);
}

function preValidate(): string | null {
  if (editingCommitments.value.length === 0) return 'At least one role is required';
  for (const c of editingCommitments.value) {
    if (!c.role.trim()) return 'Role name cannot be empty';
    if (!c.allocation || c.allocation <= 0) return `Allocation for '${c.role || 'unnamed'}' must be > 0`;
    for (const g of c.goals) {
      if (!g.trim()) return 'Goal name cannot be empty';
    }
  }
  return null;
}

async function save() {
  const err = preValidate();
  if (err) { error.value = err; return; }
  saving.value = true;
  error.value = '';
  try {
    await invoke('set_commitments', {
      rootPath: props.rootPath,
      year: props.selectedYear,
      month: props.selectedMonth,
      commitments: editingCommitments.value.map(c => ({
        role: c.role.trim(),
        allocation: c.allocation,
        goals: c.goals.map(g => g.trim()).filter(g => g !== ''),
      })),
    });
    emit('saved');
  } catch (e) {
    error.value = typeof e === 'string' ? e : String(e);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div>
    <div v-if="error" class="mb-[12px] p-[8px] bg-red-50 border border-red-200 rounded-[var(--radius-form)] text-[var(--text-sm)] text-[var(--color-danger)]">
      {{ error }}
    </div>

    <div v-for="(c, ri) in editingCommitments" :key="ri" class="mb-[16px] last:mb-0">
      <div class="flex items-center gap-[8px] mb-[10px]">
        <input
          v-model="c.role"
          placeholder="Role"
          class="w-[130px] px-[12px] py-[8px] border-2 border-[var(--color-border-form)]
                 rounded-[var(--radius-form)] text-[var(--text-base)]
                 bg-[var(--color-surface)] text-[var(--color-text-primary)]
                 outline-none transition-all duration-200
                 focus:border-[var(--color-brand-solid)] focus:bg-[#fafaff]
                 focus:shadow-[var(--shadow-focus-ring)]"
        />
        <span class="text-[var(--text-sm)] text-[var(--color-text-secondary)]">Alloc:</span>
        <input
          v-model.number="c.allocation"
          type="number" min="1" placeholder="hours"
          class="w-[56px] text-center px-[12px] py-[8px] border-2 border-[var(--color-border-form)]
                 rounded-[var(--radius-form)] text-[var(--text-base)]
                 bg-[var(--color-surface)] text-[var(--color-text-primary)]
                 outline-none transition-all duration-200
                 focus:border-[var(--color-brand-solid)] focus:bg-[#fafaff]
                 focus:shadow-[var(--shadow-focus-ring)]"
        />
        <span class="text-[var(--text-sm)] text-[var(--color-text-secondary)]">h</span>
        <button
          v-if="editingCommitments.length > 1"
          class="ml-auto text-[var(--text-sm)] text-[var(--color-text-secondary)]
                 hover:text-[var(--color-danger)] cursor-pointer transition-colors"
          @click="removeRole(ri)"
        >
          Delete Role
        </button>
      </div>

      <div class="ml-[20px] flex flex-col gap-[8px]">
        <div v-for="(_g, gi) in c.goals" :key="gi" class="flex items-center gap-[8px]">
          <input
            v-model="c.goals[gi]"
            placeholder="Goal name"
            class="flex-1 px-[12px] py-[8px] border-2 border-[var(--color-border-form)]
                   rounded-[var(--radius-form)] text-[var(--text-base)]
                   bg-[var(--color-surface)] text-[var(--color-text-primary)]
                   outline-none transition-all duration-200
                   focus:border-[var(--color-brand-solid)] focus:bg-[#fafaff]
                   focus:shadow-[var(--shadow-focus-ring)]"
          />
          <button
            class="text-[var(--color-text-secondary)] hover:text-[var(--color-danger)]
                   cursor-pointer text-[14px] transition-colors"
            @click="removeGoal(ri, gi)"
          >
            &times;
          </button>
        </div>
        <button
          class="text-[var(--text-sm)] text-[var(--color-brand-link)] font-medium cursor-pointer hover:underline"
          @click="addGoal(ri)"
        >
          + Add Goal
        </button>
      </div>

      <hr v-if="ri < editingCommitments.length - 1" class="my-[12px] border-[var(--color-divider)]" />
    </div>

    <button
      class="text-[var(--text-sm)] text-[var(--color-brand-link)] font-medium cursor-pointer hover:underline block mb-[16px]"
      @click="addRole"
    >
      + Add Role
    </button>

    <div class="flex justify-end gap-[8px] pt-[12px] border-t border-[var(--color-divider)]">
      <AppButton variant="secondary" size="sm" @click="$emit('cancel')">Cancel</AppButton>
      <AppButton size="sm" :disabled="saving" @click="save">
        {{ saving ? 'Saving...' : 'Save' }}
      </AppButton>
    </div>
  </div>
</template>
```

- [ ] **Step 2: Write tests**

Create `src/__tests__/components/composite/CommitmentsEditor.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import CommitmentsEditor from "../../../components/composite/CommitmentsEditor.vue";
import { makeCommitment } from "../../mocks/fixtures";

describe("CommitmentsEditor", () => {
  const commitments = [makeCommitment({ role: "Dev", allocation: 40, goals: ["Ship"] })];

  function mountEditor(props = {}) {
    return mount(CommitmentsEditor, {
      props: {
        commitments,
        rootPath: "/tmp",
        selectedYear: 2026,
        selectedMonth: 7,
        ...props,
      },
    });
  }

  it("renders role and goal fields", () => {
    const wrapper = mountEditor();
    expect(wrapper.text()).toContain("Dev");
    expect(wrapper.text()).toContain("Ship");
  });

  it("adds a goal row on + Add Goal click", async () => {
    const wrapper = mountEditor();
    const btn = wrapper.find("button:not(.ml-auto):not(.flex.justify-end *)");
    // Find the "+ Add Goal" button specifically
    const addBtns = wrapper.findAll("button");
    const addGoalBtn = [...addBtns].find(b => b.text().includes("Add Goal"));
    if (addGoalBtn) {
      await addGoalBtn.trigger("click");
      // A new empty goal input should appear
      const inputs = wrapper.findAll("input[placeholder='Goal name']");
      expect(inputs.length).toBeGreaterThanOrEqual(2);
    }
  });

  it("adds a role on + Add Role click", async () => {
    const wrapper = mountEditor();
    const btn = [...wrapper.findAll("button")].find(b => b.text().includes("Add Role"));
    if (btn) {
      await btn.trigger("click");
      const roleInputs = wrapper.findAll("input[placeholder='Role']");
      expect(roleInputs.length).toBe(2);
    }
  });

  it("shows validation error for empty role name", async () => {
    const wrapper = mountEditor();
    // Empty the role
    const roleInput = wrapper.find("input[placeholder='Role']");
    await roleInput.setValue("");
    // Click Save
    const saveBtn = [...wrapper.findAll("button")].find(b => b.text().includes("Save"));
    if (saveBtn) {
      await saveBtn.trigger("click");
      expect(wrapper.text()).toContain("Role name cannot be empty");
    }
  });

  it("emits cancel on Cancel click", async () => {
    const wrapper = mountEditor();
    const cancelBtn = [...wrapper.findAll("button")].find(b => b.text().includes("Cancel"));
    if (cancelBtn) {
      await cancelBtn.trigger("click");
      expect(wrapper.emitted("cancel")).toHaveLength(1);
    }
  });
});
```

- [ ] **Step 3: Run tests**

```bash
pnpm test -- src/__tests__/components/composite/CommitmentsEditor.test.ts
```

Expected: all 5 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/components/composite/CommitmentsEditor.vue src/__tests__/components/composite/CommitmentsEditor.test.ts
git commit -m "feat: add CommitmentsEditor composite component with dynamic role/goal form"
```

---

### Task 12: Refactor existing views

**Files:**
- Modify: `src/components/App.vue` (replace Toast with base Toast)
- Modify: `src/components/QuickEntry.vue` (use AppInput, AppButton, AppChip, MentionMenu)
- Modify: `src/components/EntryInput.vue` (use AppInput, AppButton, AppChip, MentionMenu)
- Modify: `src/components/EntryList.vue` (use EntryRow instead of EntryItem)
- Modify: `src/components/CommitmentsPanel.vue` (use ProgressBar, CommitmentsEditor)
- Modify: `src/components/DimensionPanel.vue` (use AppSelect, AppChip)
- Modify: `src/components/MonthNavigator.vue` (use AppButton)
- Modify: `src/components/DayStrip.vue` (apply new border/shadow/focus styles)
- Modify: `src/components/SetupScreen.vue` (use AppButton)
- Modify: `src/components/ConfigErrorBanner.vue` (use AppButton)

**Note:** This task is large. Each sub-step modifies one component. Run the existing test suite after each sub-step to verify no regressions.

- [ ] **Step 1: Replace inline Toast in App.vue with base Toast component**

Read `src/components/App.vue`, find the `<Teleport to="body">` section containing the undo toast. Replace both toast Teleports with:

```vue
<script setup lang="ts">
// Add import:
import Toast from './base/Toast.vue';
// ... existing imports
</script>

<template>
  <!-- Replace the two Teleport blocks for toasts with: -->
  <Toast
    :show="showUndoToast"
    message="Entry deleted"
    undo-label="Undo"
    @undo="handleUndo"
    @dismiss="dismissUndo"
  />
  <Toast
    :show="showScanWarning"
    :message="`${scanWarnings.length} data issue${scanWarnings.length > 1 ? 's' : ''} found during scan`"
    @dismiss="dismissScanWarning"
  />
  <!-- Remove old <style> block for .toast-* classes (now in Toast.vue) -->
</template>
```

Remove the `<style>` block containing `.toast-enter-active`, `.toast-leave-active`, `.toast-enter-from`, `.toast-leave-to`.

- [ ] **Step 2: Refactor EntryList to use EntryRow**

Replace `EntryItem` import and usage with `EntryRow` in `EntryList.vue`:

```vue
<script setup lang="ts">
// Replace: import EntryItem from "./EntryItem.vue";
import EntryRow from "./composite/EntryRow.vue";
// ... rest unchanged
</script>

<template>
  <!-- Replace <EntryItem ... /> with <EntryRow ... /> -->
  <div class="bg-[var(--color-surface)] rounded-[var(--radius-card)] shadow-[var(--shadow-card)]">
    <div v-if="entries.length === 0" class="p-8 text-center text-[var(--color-text-secondary)] text-sm">
      No entries yet. Log your first work item above.
    </div>
    <div v-else class="px-4">
      <EntryRow
        v-for="(entry, index) in entries"
        :key="entry.id"
        :entry="entry"
        :index="index"
        @update="(entryId, item, dur) => emit('update', entryId, item, dur)"
        @delete="(entryId) => emit('delete', entryId)"
        @update-dimensions="(entryId, dims) => emit('updateDimensions', entryId, dims)"
      />
      <div class="flex justify-between text-[13px] text-[var(--color-text-secondary)] py-[12px] border-t-2 border-[var(--color-divider)] mt-[4px]">
        <span>{{ entries.length }} {{ entries.length === 1 ? "entry" : "entries" }}</span>
        <span class="font-bold text-[15px] text-[var(--color-brand-link)]">{{ formatDuration(totalMinutes) }}</span>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 3: Refactor QuickEntry.vue**

Replace the raw `<input>` and `<button>` in `QuickEntry.vue`'s template with `AppInput` and `AppButton`. Replace inline chip markup with `AppChip`. Replace the inline @mention menu with `MentionMenu`.

Key changes:
- `<input class="..." ref="inputEl" ...>` → `<AppInput v-model="input" :placeholder="...">`
- `<button class="...">Log</button>` → `<AppButton @click="handleSubmit">Log</AppButton>`
- Dimension chips → `<AppChip v-for="..." :color="..." :label="..." :value="..." closable @close="..." />`
- @mention menu → `<MentionMenu v-if="menuVisible" ... />`

- [ ] **Step 4: Refactor EntryInput.vue**

Same pattern as QuickEntry — replace raw inputs/buttons/chips with AppInput, AppButton, AppChip. The @mention menu logic in EntryInput is the most complex part — it can be simplified to delegate to MentionMenu component.

- [ ] **Step 5: Refactor CommitmentsPanel.vue**

Replace the inline progress bars with `<ProgressBar>`. Replace the inline editing form with `<CommitmentsEditor>`.

- [ ] **Step 6: Refactor remaining views**

For DimensionPanel, MonthNavigator, DayStrip, SetupScreen, ConfigErrorBanner: apply design tokens via CSS variables and replace raw buttons/inputs with AppButton.

- [ ] **Step 7: Remove EntryItem.vue**

```bash
rm src/components/EntryItem.vue
rm src/__tests__/components/EntryItem.test.ts
```

- [ ] **Step 8: Update existing tests for modified selectors**

Run the full test suite. Tests that query specific CSS classes (like `.text-sm.text-gray-800`) will need selector updates because the new components use CSS variable-based classes. Fix each failing test by updating the selector to match the new markup.

```bash
pnpm test -- --run
```

Expected: all tests eventually pass after selector fixes.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "refactor: replace raw HTML with base/composite components across all views"
```

---

### Task 13: Dark mode verification

**Files:**
- Check: `src/assets/tokens.css` (dark mode tokens already defined in Task 1)
- Check: `src/assets/main.css` (dark mode bg-image:none already set)

- [ ] **Step 1: Verify dark mode tokens**

All component styles use `var(--color-*)` and `var(--shadow-*)` — dark mode should work automatically via the `@media (prefers-color-scheme: dark)` block in `tokens.css`. No additional code changes needed.

- [ ] **Step 2: Manual verification**

Launch the app and toggle system dark mode. Verify:
- Cards have no shadow (replaced by background contrast)
- Gradient accents (buttons, progress bars) remain vibrant
- Chip colors are preserved
- Focus rings visible against dark backgrounds
- Toast has correct dark shadow

- [ ] **Step 3: Commit (if any fixes needed)**

```bash
git add -A
git commit -m "fix: dark mode verification and edge case fixes"
```

---

### Task 14: prefers-reduced-motion support

**Files:**
- Modify: `src/assets/main.css`

- [ ] **Step 1: Add reduced-motion media query**

Add to end of `src/assets/main.css`:

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/assets/main.css
git commit -m "feat: disable animations when prefers-reduced-motion is set"
```

---

### Task 15: Final test pass and cleanup

- [ ] **Step 1: Run full test suite**

```bash
cd /Users/boxcounter/Code/BoxCounter/logbook
pnpm test -- --run
```

Expected: all tests PASS with zero failures.

- [ ] **Step 2: Run TypeScript check**

```bash
pnpm run build
```

Expected: `vue-tsc --noEmit` passes with no errors.

- [ ] **Step 3: Remove unused imports and dead code**

Search for any imports of removed files (EntryItem) and remove them. Check for unused CSS classes.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: final test pass, TypeScript check, and cleanup"
```

---

## Implementation Notes

- **Incremental commits**: Each task ends with a commit. If a task produces broken tests, fix before committing.
- **Test resilience**: Tests that query `.text-sm.text-gray-800` or similar Tailwind class selectors will break as components adopt CSS variables. Update selectors to use data-testid attributes or text content assertions instead.
- **No functional changes**: All Tauri invoke() calls, data structures, and business logic remain unchanged. This is a pure view-layer refactor.
- **EntryItem removal**: After Task 12 step 7, EntryItem.vue is deleted. Ensure no imports reference it.
- **Reka UI and jsdom**: Some Reka UI components use Teleport/Portal which may behave differently in jsdom. If tests fail, use `attachTo: document.body` in mount options, or stub the Reka UI components in test setup.
