# Unify Value Input to Inline Editing

## 范围

`src/components/composite/DimensionEditorModal.vue` — 删除 transient value 输入框，改为 inline 编辑。`CommitmentsModal.vue` 不改。

本 spec 替代 `2026-07-04-dimension-value-add-hint-design.md`（其添加的 hint 对应被删除的 transient 输入框）。

## 动机

DimensionsModal 现有两套 value 编辑模式并存：已有 value 是 inline `v-model` 绑定，新增 value 却走 transient 输入框（输入 → Enter/+ → 提交到数组）。不一致且多一步无意义的提交动作。CommitmentsModal goal 已用纯 inline 模式，行为对齐即可。

## 改动

### 1. 删除 transient 输入框

删除以下代码块（模板 440-458 行附近）：

```html
<!-- New value input (hidden when deleted) -->
<template v-if="!selectedDimension.deleted">
  <div class="flex items-center gap-sm mt-sm">
    <span ...>⠿</span>
    <input v-model="newValue" ... @keydown.enter.exact.prevent="addValue" />
    <button data-test="add-value" @click="addValue">+</button>
  </div>
  <p v-if="newValue.trim()" ...>Press Enter or click + to add</p>
</template>
```

### 2. 删除关联 script

- `const newValue = ref("")`
- `function addValue() { ... }`
- `newValue.value = ""` 重置（watch 内）

### 3. 修改 `+ Add Value` 按钮行为

当前有一个 `+ Add Value` 按钮？查看模板——没有，只有 transient 输入框内的 `+` 按钮。改为：在 `VueDraggable` 下方、`</template>` 结束之前，新增一个按钮，点击直接 push 空字符串到 `selectedDimension.values`：

```html
<button
  v-if="!selectedDimension.deleted"
  data-test="add-value-btn"
  class="self-start mt-sm text-secondary font-medium text-[var(--color-brand-link)] cursor-pointer hover:underline"
  @click="selectedDimension.values = [...selectedDimension.values, '']"
>+ Add Value</button>
```

（样式对齐 CommitmentsModal 的 `+ Add Goal` 按钮）

### 4. Enter 在 value input 上插入新空行

给每个 value input 添加 `@keydown.enter.exact.prevent="onValueEnter(i)"`，实现与 `onGoalEnter` 一致的逻辑：

```ts
function onValueEnter(index: number) {
  if (!selectedDimension.value?.values) return;
  const values = selectedDimension.value.values;
  if (index === values.length - 1 && values[index].trim() === "") return;
  values.splice(index + 1, 0, "");
}
```

### 5. 保存时过滤空 value

在 `save()` 中，提交前过滤掉空字符串：

```ts
// 在 invoke 之前，对每个 static 维度的 values 过滤空字符串
const cleaned = draft.value.map(d => {
  if (d.source === 'static' && d.values) {
    return { ...d, values: d.values.filter(v => v.trim() !== '') };
  }
  return d;
});
```

### 6. Enter 插入空行同步应用到 CommitmentsModal（已有）

`CommitmentsModal` 的 `onGoalEnter` 已有 guard + `toCommitments` 的 filter 双重防线，不动。

## 不变

- value 列表的 inline 编辑（已有 `updateValue`）不改变
- `VueDraggable`、拖拽、删除 value 行为不变
- `source !== 'static'` 的维度不显示 value 区域，不受影响
- `selectedDimension.deleted` 时不显示 value 区域，不受影响
- CommitmentsModal 完全不变
- 交互原则合规：不静默丢失输入、Esc 消解、键盘优先，均不涉及新增交互路径

## 清理

删除本 spec 替代的文件：`docs/superpowers/specs/2026-07-04-dimension-value-add-hint-design.md`

其对应的 hint 代码（`<p v-if="newValue.trim()" ...>Press Enter or click + to add</p>`）已在本次删除的 transient 输入框块中一同移除。

## 测试关注点

- 新增 value 后 Enter 插入新空行，最后一个空行不产生新空行（guard）
- 保存后 .yaml 文件不包含空字符串 value
- `data-test="add-value-btn"` 替代已删除的 `data-test="add-value"`，同步更新 `src/__tests__/components/composite/DimensionEditorModal.test.ts` 中 4 处引用（L124、L149、L168、L531），以及 "hides add-value section" 测试用例调整为验证 `delete` 状态下按钮不可见
- 已有 value 的 inline 编辑、拖拽、删除行为不受影响
