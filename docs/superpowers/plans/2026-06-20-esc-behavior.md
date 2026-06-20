# Esc 行为改进 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 给 EntryRowEdit、TwoLineInput、Day note、QuickJumpPopover 补齐 esc（退出/取消/清空/关闭）处理，行为按组件作用域隔离。

**Architecture:** esc 处理挂在各组件自身根元素的 `@keydown` 上（靠事件冒泡捕获聚焦子元素），不新增全局监听，从而把作用域限定在当前聚焦的组件，避免多个编辑器同时响应同一次 esc。`DimensionPopover` 维持现状（最内层、capture-phase + stopPropagation 优先处理）。

**Tech Stack:** Vue 3 Composition API + TypeScript，测试 Vitest + @vue/test-utils。

测试命令：`npm test`（= `vitest run`）。单文件：`npx vitest run <path>`。

参考设计：`docs/superpowers/specs/2026-06-20-esc-behavior-design.md`

---

## File Structure

- `src/components/composite/EntryRowEdit.vue` — 加 `isDirty` / `confirming` 状态、esc handler、就地确认条
- `src/components/TwoLineInput.vue` — 在 `onKeydown` 加 esc 清空分支
- `src/components/QuickJumpPopover.vue` — 加 `close` emit 与根 div `@keydown.esc`
- `src/components/HeatmapCalendar.vue` — 接 `@close` 关闭 jump 弹层
- `src/components/MonthView.vue` — Day note 加 `@focus` 快照 + `@keydown.esc` 还原
- 对应测试文件：`src/__tests__/components/composite/EntryRowEdit.test.ts`、`TwoLineInput.test.ts`、`QuickJumpPopover.test.ts`、`HeatmapCalendar.test.ts`、`MonthView.test.ts`

---

## Task 1: EntryRowEdit —— dirty 感知 + 就地确认条

**Files:**
- Modify: `src/components/composite/EntryRowEdit.vue`
- Test: `src/__tests__/components/composite/EntryRowEdit.test.ts`

- [ ] **Step 1: 写失败测试**

追加到 `src/__tests__/components/composite/EntryRowEdit.test.ts` 的 `describe("EntryRowEdit", ...)` 块内（`mountEdit` / `dimensions` / `fullDims` 已在文件顶部定义）：

```ts
  it("Esc with no changes emits cancel", async () => {
    const wrapper = mountEdit();
    await wrapper.trigger("keydown", { key: "Escape" });
    expect(wrapper.emitted("cancel")).toBeTruthy();
  });

  it("Esc with unsaved changes shows the discard confirm bar and does NOT cancel", async () => {
    const wrapper = mountEdit();
    await wrapper.findAll("input")[0].setValue("Changed item");
    await wrapper.trigger("keydown", { key: "Escape" });
    expect(wrapper.find("[data-test='discard-prompt']").exists()).toBe(true);
    expect(wrapper.emitted("cancel")).toBeFalsy();
  });

  it("Esc again on the confirm bar discards (emits cancel)", async () => {
    const wrapper = mountEdit();
    await wrapper.findAll("input")[0].setValue("Changed item");
    await wrapper.trigger("keydown", { key: "Escape" });
    await wrapper.trigger("keydown", { key: "Escape" });
    expect(wrapper.emitted("cancel")).toBeTruthy();
  });

  it("clicking Keep-editing leaves edit mode active (confirm bar gone, no cancel)", async () => {
    const wrapper = mountEdit();
    await wrapper.findAll("input")[0].setValue("Changed item");
    await wrapper.trigger("keydown", { key: "Escape" });
    await wrapper.find("[data-test='keep-editing']").trigger("click");
    expect(wrapper.find("[data-test='discard-prompt']").exists()).toBe(false);
    expect(wrapper.emitted("cancel")).toBeFalsy();
    expect(wrapper.find("[data-test='save']").exists()).toBe(true);
  });

  it("clicking Discard emits cancel", async () => {
    const wrapper = mountEdit();
    await wrapper.findAll("input")[0].setValue("Changed item");
    await wrapper.trigger("keydown", { key: "Escape" });
    await wrapper.find("[data-test='discard']").trigger("click");
    expect(wrapper.emitted("cancel")).toBeTruthy();
  });

  it("Esc does nothing while the DimensionPopover is open", async () => {
    const wrapper = mountEdit();
    await wrapper.findAll("input")[0].setValue("Changed item");
    await wrapper.find("[data-test='add-tag']").trigger("click"); // open popover
    await wrapper.trigger("keydown", { key: "Escape" });
    expect(wrapper.emitted("cancel")).toBeFalsy();
    expect(wrapper.find("[data-test='discard-prompt']").exists()).toBe(false);
  });
```

- [ ] **Step 2: 运行测试，确认失败**

Run: `npx vitest run src/__tests__/components/composite/EntryRowEdit.test.ts`
Expected: 新增用例 FAIL（找不到 `discard-prompt` / cancel 未触发等）。

- [ ] **Step 3: 实现 —— script 部分**

在 `src/components/composite/EntryRowEdit.vue` 的 `<script setup>` 中，`popoverUp` 声明之后加入状态与 handler（`resolveDelta` 已 import）：

```ts
const confirming = ref(false);

const isDirty = computed(() =>
  item.value !== props.entry.item ||
  resolveDelta(durText.value, props.entry.duration) !== props.entry.duration ||
  JSON.stringify(dimValues.value) !== JSON.stringify(props.entry.dimensions)
);

function onEsc() {
  if (popoverOpen.value) return;            // popover owns esc
  if (confirming.value || !isDirty.value) { // 2nd esc, or nothing to lose
    emit("cancel");
    return;
  }
  confirming.value = true;                  // dirty: ask before discarding
}

// Enter normally saves; while confirming it means "keep editing".
function onEnter() {
  if (confirming.value) { confirming.value = false; return; }
  save();
}
```

- [ ] **Step 4: 实现 —— template 部分**

4a. 根 `<div ref="rootEl" ...>` 上加 esc 监听（追加到已有属性，不影响 class）：

```html
  <div
    ref="rootEl"
    @keydown.esc="onEsc"
    class="bg-[var(--color-surface)] border border-[var(--color-brand-solid)] rounded-[var(--radius-form-lg)]
           shadow-[var(--shadow-focus-ring)] px-[14px] py-[9px] flex flex-col gap-[4px] relative"
  >
```

4b. 两个 input 的 `@keydown.enter.prevent="save"` 改为 `onEnter`，并加 `@input="confirming = false"`（打字即取消确认态）：

```html
      <input
        v-model="item"
        class="flex-1 text-[length:var(--app-text-base)] font-medium text-[var(--color-text-primary)] border-none outline-none bg-transparent py-[1px]"
        @keydown.enter.prevent="onEnter"
        @input="confirming = false"
      />
```

```html
      <input
        v-model="durText"
        class="mono w-[56px] text-right text-[length:var(--app-text-sm)] text-[var(--color-text-primary)]
               border border-[var(--color-border-form)] rounded-[var(--radius-form)] px-[8px] py-[2px]
               outline-none focus:border-[var(--color-brand-solid)]"
        @keydown.enter.prevent="onEnter"
        @input="confirming = false"
      />
```

4c. 把操作按钮行整段替换为「正常态 / 确认态」二选一：

原：

```html
    <div class="flex gap-[8px] mt-[4px] items-center">
      <button data-test="save" class="text-[length:var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] bg-[var(--color-brand-solid)] text-white hover:bg-[var(--color-brand-link)]" @click="save">Save</button>
      <button data-test="cancel" class="text-[length:var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)]" @click="emit('cancel')">Cancel</button>
      <button data-test="delete" class="text-[length:var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] text-[var(--color-text-disabled)] hover:text-[var(--color-danger)] ml-auto" @click="emit('delete')">Delete</button>
    </div>
```

改为：

```html
    <div class="flex gap-[8px] mt-[4px] items-center">
      <template v-if="!confirming">
        <button data-test="save" class="text-[length:var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] bg-[var(--color-brand-solid)] text-white hover:bg-[var(--color-brand-link)]" @click="save">Save</button>
        <button data-test="cancel" class="text-[length:var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)]" @click="emit('cancel')">Cancel</button>
        <button data-test="delete" class="text-[length:var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] text-[var(--color-text-disabled)] hover:text-[var(--color-danger)] ml-auto" @click="emit('delete')">Delete</button>
      </template>
      <template v-else>
        <span data-test="discard-prompt" class="text-[length:var(--app-text-micro)] text-[var(--color-text-secondary)]">放弃修改？</span>
        <button data-test="discard" class="text-[length:var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] text-[var(--color-danger)] hover:underline" @click="emit('cancel')">放弃</button>
        <button data-test="keep-editing" class="text-[length:var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] bg-[var(--color-brand-solid)] text-white hover:bg-[var(--color-brand-link)]" @click="confirming = false">继续编辑</button>
      </template>
    </div>
```

- [ ] **Step 5: 运行测试，确认通过**

Run: `npx vitest run src/__tests__/components/composite/EntryRowEdit.test.ts`
Expected: 全部 PASS（含原有用例）。

- [ ] **Step 6: 提交**

```bash
git add src/components/composite/EntryRowEdit.vue src/__tests__/components/composite/EntryRowEdit.test.ts
git commit -m "feat(ui): esc cancels EntryRowEdit; confirm before discarding unsaved changes"
```

---

## Task 2: TwoLineInput —— 有内容时 esc 清空

**Files:**
- Modify: `src/components/TwoLineInput.vue:65-76`
- Test: `src/__tests__/components/TwoLineInput.test.ts`

- [ ] **Step 1: 写失败测试**

追加到 `src/__tests__/components/TwoLineInput.test.ts` 的 describe 块内。若文件顶部已有 `dimensions` / `commitments` / mount 辅助，复用之；否则用下面的自包含 mount：

```ts
  it("Esc clears typed text without emitting submit", async () => {
    const wrapper = mount(TwoLineInput, {
      props: { dimensions: [], commitments: [], initialValues: {} },
    });
    const input = wrapper.find("input");
    await input.setValue("draft work 1h");
    await input.trigger("keydown", { key: "Escape" });
    expect((input.element as HTMLInputElement).value).toBe("");
    expect(wrapper.emitted("submit")).toBeFalsy();
  });

  it("Esc on an empty input does nothing", async () => {
    const wrapper = mount(TwoLineInput, {
      props: { dimensions: [], commitments: [], initialValues: {} },
    });
    const input = wrapper.find("input");
    await input.trigger("keydown", { key: "Escape" });
    expect((input.element as HTMLInputElement).value).toBe("");
    expect(wrapper.emitted("submit")).toBeFalsy();
  });
```

文件顶部需 `import TwoLineInput from "../../components/TwoLineInput.vue";` 与 `import { mount } from "@vue/test-utils";`（多数已存在，缺则补）。

- [ ] **Step 2: 运行测试，确认失败**

Run: `npx vitest run src/__tests__/components/TwoLineInput.test.ts`
Expected: "Esc clears typed text..." FAIL（值仍为 `draft work 1h`，因为 esc 当前不被处理）。

- [ ] **Step 3: 实现**

在 `src/components/TwoLineInput.vue` 的 `onKeydown` 中，`@` 分支之前插入 esc 分支：

```ts
function onKeydown(e: KeyboardEvent) {
  // Esc when the popover is closed: clear the in-progress entry. While the
  // popover is open its capture-phase listener owns Esc (back/close), and its
  // stopPropagation means this handler won't even see the event.
  if (e.key === "Escape") {
    if (popoverOpen.value) return;
    const hasContent =
      text.value.trim() !== "" ||
      JSON.stringify(dimValues.value) !== JSON.stringify(props.initialValues);
    if (!hasContent) return;
    e.preventDefault();
    text.value = "";
    dimValues.value = { ...props.initialValues };
    submitAttempted.value = false;
    return;
  }
  // Esc is owned by the popover (capture-phase window listener).
  if (e.key === "@") { e.preventDefault(); popoverOpen.value = true; return; }
  // Enter must never be blocked (spec §5.2): submit even with the popover open
  // or required dimensions unfilled. Close the popover first if it is open.
  if (e.key === "Enter") {
    e.preventDefault();
    if (popoverOpen.value) closePopover();
    handleSubmit();
    return;
  }
}
```

- [ ] **Step 4: 运行测试，确认通过**

Run: `npx vitest run src/__tests__/components/TwoLineInput.test.ts`
Expected: 全部 PASS。

- [ ] **Step 5: 提交**

```bash
git add src/components/TwoLineInput.vue src/__tests__/components/TwoLineInput.test.ts
git commit -m "feat(ui): esc clears the TwoLineInput add bar when it has content"
```

---

## Task 3: QuickJumpPopover / HeatmapCalendar —— esc 关闭跳转弹层

**Files:**
- Modify: `src/components/QuickJumpPopover.vue`
- Modify: `src/components/HeatmapCalendar.vue:115-120`
- Test: `src/__tests__/components/QuickJumpPopover.test.ts`、`src/__tests__/components/HeatmapCalendar.test.ts`

- [ ] **Step 1: 写失败测试（QuickJumpPopover）**

追加到 `src/__tests__/components/QuickJumpPopover.test.ts` 的 describe 块内（`mountPop` 已定义）：

```ts
  it("Esc emits close", async () => {
    const wrapper = mountPop();
    await wrapper.trigger("keydown", { key: "Escape" });
    expect(wrapper.emitted("close")).toBeTruthy();
  });
```

- [ ] **Step 2: 运行测试，确认失败**

Run: `npx vitest run src/__tests__/components/QuickJumpPopover.test.ts`
Expected: "Esc emits close" FAIL（无 close 事件）。

- [ ] **Step 3: 实现 QuickJumpPopover**

3a. `<script setup>` 中扩展 emits：

```ts
const emit = defineEmits<{ jump: [{ year: number; month: number }]; close: [] }>();
```

3b. template 根 `<div>` 上加 `@keydown.esc`（追加属性，class 不变）：

```html
  <div
    @keydown.esc="emit('close')"
    class="flex gap-[8px] items-center bg-[var(--color-surface)] border border-[var(--color-border-form)]
           rounded-[var(--radius-form-lg)] shadow-[var(--shadow-quickjump)] px-[12px] py-[10px]"
  >
```

- [ ] **Step 4: 运行 QuickJumpPopover 测试，确认通过**

Run: `npx vitest run src/__tests__/components/QuickJumpPopover.test.ts`
Expected: 全部 PASS。

- [ ] **Step 5: 写失败测试（HeatmapCalendar 接线）**

追加到 `src/__tests__/components/HeatmapCalendar.test.ts` 的 describe 块内。Props 名称参考文件已有的 `mount` 辅助；若无辅助，用自包含 mount：

```ts
  it("closing the jump popover (its close event) hides it", async () => {
    const wrapper = mount(HeatmapCalendar, {
      props: {
        year: 2026, month: 6, selectedDate: "2026-06-10",
        monthEntries: {}, availableMonths: [{ year: 2026, month: 6 }],
      },
    });
    await wrapper.find("[data-test='month-label']").trigger("click"); // open jump
    const jump = wrapper.findComponent({ name: "QuickJumpPopover" });
    expect(jump.exists()).toBe(true);
    jump.vm.$emit("close");
    await wrapper.vm.$nextTick();
    expect(wrapper.findComponent({ name: "QuickJumpPopover" }).exists()).toBe(false);
  });
```

文件顶部需有 `import HeatmapCalendar from "../../components/HeatmapCalendar.vue";` 与 `import { mount } from "@vue/test-utils";`（缺则补）。

- [ ] **Step 6: 运行测试，确认失败**

Run: `npx vitest run src/__tests__/components/HeatmapCalendar.test.ts`
Expected: 新用例 FAIL（popover 未因 close 隐藏，因为 `@close` 尚未接线）。

- [ ] **Step 7: 实现 HeatmapCalendar 接线**

`src/components/HeatmapCalendar.vue` template 中的 `<QuickJumpPopover>` 加 `@close`：

```html
    <QuickJumpPopover
      v-if="showJump && availableMonths !== null"
      :year="year" :month="month" :available-months="availableMonths"
      class="mb-[8px]"
      @jump="onJump"
      @close="showJump = false"
    />
```

- [ ] **Step 8: 运行测试，确认通过**

Run: `npx vitest run src/__tests__/components/HeatmapCalendar.test.ts src/__tests__/components/QuickJumpPopover.test.ts`
Expected: 全部 PASS。

- [ ] **Step 9: 提交**

```bash
git add src/components/QuickJumpPopover.vue src/components/HeatmapCalendar.vue src/__tests__/components/QuickJumpPopover.test.ts src/__tests__/components/HeatmapCalendar.test.ts
git commit -m "feat(ui): esc closes the QuickJump month popover"
```

---

## Task 4: Day note —— esc 静默还原

**Files:**
- Modify: `src/components/MonthView.vue`（Day note 区，script ~182-210 / template ~296-306）
- Test: `src/__tests__/components/MonthView.test.ts`

- [ ] **Step 1: 写失败测试**

追加到 `src/__tests__/components/MonthView.test.ts` 的 `describe("MonthView", ...)` 块内（`mountView` 已定义）：

```ts
  it("Esc on the day note reverts its content to the pre-edit snapshot", async () => {
    const wrapper = mountView();
    const note = wrapper.find("[contenteditable]");
    note.element.textContent = "original";
    await note.trigger("focus");          // snapshot taken here
    note.element.textContent = "edited away";
    await note.trigger("keydown", { key: "Escape" });
    expect(note.element.textContent).toBe("original");
  });
```

- [ ] **Step 2: 运行测试，确认失败**

Run: `npx vitest run src/__tests__/components/MonthView.test.ts`
Expected: 新用例 FAIL（textContent 仍为 `edited away`）。

- [ ] **Step 3: 实现 —— script 部分**

在 `src/components/MonthView.vue` 的 `// ---- Day note (inline) ----` 区块内，`saveNote` 之后加入快照与还原 handler：

```ts
let noteSnapshot = "";
function onNoteFocus() {
  noteSnapshot = noteRef.value?.textContent || "";
}
function onNoteEsc(e: KeyboardEvent) {
  e.preventDefault();
  if (noteRef.value) noteRef.value.textContent = noteSnapshot;
  noteRef.value?.blur(); // triggers saveNote with unchanged content (no-op write)
}
```

- [ ] **Step 4: 实现 —— template 部分**

note 的 `<div ref="noteRef" ...>` 加 `@focus` 与 `@keydown.esc`（追加到已有事件属性）：

```html
        <div
          ref="noteRef"
          class="text-[length:var(--app-text-xs)] italic text-[var(--color-text-secondary)] leading-[1.5] cursor-text px-[10px] py-[6px] rounded-[var(--radius-form-lg)] outline-none hover:bg-[var(--color-page-bg)]"
          contenteditable="true"
          data-placeholder="Add a note…"
          @focus="onNoteFocus"
          @keydown.esc="onNoteEsc"
          @blur="saveNote"
          @paste="onNotePaste"
          @input="onNoteInput"
        ></div>
```

- [ ] **Step 5: 运行测试，确认通过**

Run: `npx vitest run src/__tests__/components/MonthView.test.ts`
Expected: 全部 PASS。

- [ ] **Step 6: 提交**

```bash
git add src/components/MonthView.vue src/__tests__/components/MonthView.test.ts
git commit -m "feat(ui): esc reverts the day note to its pre-edit content"
```

---

## Task 5: 全量回归 + 文档同步

**Files:**
- 可能 Modify: `SPEC.md`（若其中有键盘快捷键章节，补 esc 行为；无则跳过）

- [ ] **Step 1: 跑全量测试**

Run: `npm test`
Expected: 全绿。若有失败，定位到上面对应 Task 修复，不要新增 esc 全局监听。

- [ ] **Step 2: 一致性检查**

调用 `/check-consistency`（项目规则：HANDOFF 撰写前 / Phase 结束时触发）。确认 SPEC.md 与新 esc 行为不矛盾；如 SPEC 有快捷键表，补一行 esc。

- [ ] **Step 3: 提交文档（若有改动）**

```bash
git add SPEC.md
git commit -m "docs: note esc behavior in keyboard section"
```

---

## Self-Review 记录

- **Spec 覆盖**：EntryRowEdit（Task 1）/ TwoLineInput（Task 2）/ QuickJump+Heatmap（Task 3）/ Day note（Task 4）逐一对应；架构「作用域绑定」体现在各 Task 用元素级 `@keydown` 而非 window 监听。
- **精度规则**：EntryRowEdit `onEsc` 首行 `if (popoverOpen.value) return`、TwoLineInput esc 分支 `if (popoverOpen.value) return`，对应 spec 精度规则。
- **类型一致**：QuickJumpPopover emits 扩展为 `{ jump; close }`，HeatmapCalendar 接 `@close`，名称一致。
- **排除项**：未触碰 DimensionPopover、CommitmentsEditor，符合 spec。
