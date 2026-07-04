# Dimension Value Add Hint

## 范围

`src/components/composite/DimensionEditorModal.vue` 一处添加。

## 改动

在虚线框输入行（第 455 行 `</div>` 结束）之后，添加一行条件提示：

```html
<p v-if="newValue.trim()" class="text-micro text-[var(--color-text-muted)] mt-xs">
  Press Enter or click + to add
</p>
```

## 行为

- `newValue` 有非空白内容时显示提示
- `newValue` 为空时隐藏
- `addValue` 执行后 `newValue` 被清空，提示自动消失
- 仅对 `source === 'static'` 且 `!deleted` 的维度有效（输入框自身受同一条件保护）

## 不变

- 输入框、`+` 按钮、`Enter` 快捷键的行为完全不变
- 不新增 props/state/事件
- 不涉及其他文件
