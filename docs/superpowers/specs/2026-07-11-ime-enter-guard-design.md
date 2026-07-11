# IME Enter 守卫：EntryRowEdit + EntryComposer

**日期**：2026-07-11
**状态**：待实现
**类型**：Bug fix

## 问题

中文（及其他 IME）输入法弹出候选词列表时，用 Enter 选词会误触发编辑完成 / 提交。

**根因**：`EntryRowEdit.vue` 和 `EntryComposer.vue` 的 Enter 处理没有 `e.isComposing` 守卫。

- `EntryRowEdit`：两个 `<input>`（item、duration）都用 `@keydown.enter.prevent="onEnter"`，`onEnter()` 是无参裸函数，直接调用 `save()`。
- `EntryComposer`：`onKeydown` 的 Enter 分支（约第 90 行）直接调用 `handleSubmit()`。

两处都未检查 `e.isComposing`，导致输入法组合期间的 Enter 被当作提交。

**已正确处理的其他入口**（对比基准）：

| 文件 | 行 | 守卫 |
|------|----|------|
| `src/composables/useDayNote.ts` | 61 | `if (e.isComposing) return;` |
| `src/components/DimensionPopover.vue` | 156 | `if (e.isComposing) return;` |
| `src/components/composite/CommitmentsModal.vue` | 197 | `if (e?.isComposing) return;` |
| `src/components/composite/DimensionEditorModal.vue` | 102 | `if (e?.isComposing) return;` |

`docs/interaction-principles.md` 第 4 条（第 46 行）已明确规定此规则，本次修复是补齐遗漏。

## 方案

用 `e.isComposing` 守卫，与项目现有 4 处保持一致。不引入 `compositionstart`/`compositionend` 监听 + ref 标志位——`isComposing` 已验证可用，额外机制违背 interaction-principles「不发明新机制」。

### 改动 1：`EntryRowEdit.vue`

**当前**：

```vue
<input ... @keydown.enter.prevent="onEnter" />
```

```ts
// Enter normally saves; while confirming it means "keep editing".
function onEnter() {
  if (confirming.value) { confirming.value = false; return; }
  save();
}
```

**改为**：

```vue
<input ... @keydown.enter="onEnter" />
```

（移除 `.prevent` 修饰符，prevent 逻辑移入函数内，仅对真正要提交的 Enter 生效。）

```ts
// Enter normally saves; while confirming it means "keep editing".
// Guard against IME composition (e.g. Chinese pinyin candidate selection).
function onEnter(e: KeyboardEvent) {
  if (e.isComposing) return;
  e.preventDefault();
  if (confirming.value) { confirming.value = false; return; }
  save();
}
```

两个 `<input>`（item、duration）都做此改动。

### 改动 2：`EntryComposer.vue`

**当前**（`onKeydown` 内 Enter 分支，约第 90 行）：

```ts
if (e.key === "Enter") {
  e.preventDefault();
  if (popoverOpen.value) closePopover();
  handleSubmit();
  return;
}
```

**改为**：

```ts
if (e.key === "Enter") {
  // Guard against IME composition (e.g. Chinese pinyin candidate selection).
  if (e.isComposing) return;
  e.preventDefault();
  if (popoverOpen.value) closePopover();
  handleSubmit();
  return;
}
```

### 为什么 `.prevent` 要从模板移到函数内（EntryRowEdit）

Vue 的 `@keydown.enter.prevent` 修饰符在事件匹配 `.enter` 时无条件 `preventDefault()`。加 `isComposing` 守卫后，如果保留模板上的 `.prevent`，输入法选词的 Enter 也会被 `preventDefault()`。

实测中 WebKit 在 IME 组合期间的 `preventDefault()` 不阻断选词，所以保留 `.prevent` 功能上不会出错。但移到函数内有好处：

1. **语义清晰**：prevent 只对真正要提交的 Enter 生效，不产生"阻止了但又不处理"的语义模糊。
2. **与守卫逻辑一致**：守卫说"这个 Enter 不归我管"，那就不应该对它做任何事——包括 prevent。

这是干净性改进，不是行为修正。`EntryComposer` 已经是函数内 prevent，无需调整。

## `isComposing` 可靠性

`KeyboardEvent.isComposing` 是 [W3C UI Events 规范](https://www.w3.org/TR/uievents/#dom-keyboardevent-iscomposing)标准属性，所有现代浏览器（Chrome / WebKit / Firefox）支持。Tauri 2.x 在 macOS 上用系统 WebView（WebKit），对 IME 事件的 `isComposing` 支持完整。

项目已有 4 处用同一机制工作正常，本次修复只是补齐遗漏的两处。

## 不做的事

- **不引入 `compositionstart`/`compositionend` + ref 标志位**。`isComposing` 不可靠时的 fallback 方案，但本项目已验证可用，加 ref 是多余复杂度。
- **不改 Esc / `@` 的处理**。Esc 不是输入法常用键；`@` 在中文输入法下通常用 Shift+2 触发，不经过候选词列表。
- **不改其他 4 处已有守卫的代码**。

## 测试

### 单元测试（vitest）

为 `EntryRowEdit` 和 `EntryComposer` 各加一个测试用例：构造 `isComposing: true` 的 keydown Enter 事件，断言不触发 save / submit emit。

参考现有测试模式：模拟 keydown 事件，检查 emit 的 payload。

### 手动验证

1. `pnpm tauri dev` 启动
2. 切到中文拼音输入法
3. **EntryRowEdit**：双击 entry 进入编辑，输入拼音，在候选词列表用 Enter 选词 → 验证不触发保存
4. **EntryComposer**：在录入框输入拼音，在候选词列表用 Enter 选词 → 验证不触发提交
5. 英文直接 Enter 仍正常提交

## 影响范围

- `src/components/composite/EntryRowEdit.vue`：`onEnter` 签名 + 模板 `.prevent`
- `src/components/EntryComposer.vue`：`onKeydown` Enter 分支
- 对应测试文件
