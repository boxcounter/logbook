# Hide Default Template Indicator for Future Months

## 范围

`src/components/MonthView.vue` 一处修改。

## 改动

第 186 行，在 `v-if` 条件中添加 `&& !isFutureMonth`：

```html
<p v-if="store.usingDefaultDimensions && !isFutureMonth" class="mb-sm text-micro text-[var(--color-text-disabled)]">
  Using default template (no custom dimensions this month)
</p>
```

新增 computed `isFutureMonth`：当用户当前查看的月份晚于今天所在的月份时为 `true`。利用已有的 `selectedYear`/`selectedMonth` 与 `new Date()` 的年月比较得出。

## 行为

- 未来月份：不显示提示
- 当前月份及历史月份：`store.usingDefaultDimensions` 为 true 时照常显示

## 不变

- `store.usingDefaultDimensions` 的值和语义
- 其他组件无影响
