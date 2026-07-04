# Dimensions Edit Refine

## 范围

三个小改动，均落在 `src/components/composite/DimensionEditorModal.vue`，无新增依赖，无接口变更。

---

## 改动点

### 1. Save as template 过滤已删除维度

**现状**：`saveAsTemplate()` 将 `draft.value`（含 `deleted: true` 的维度）直接传给 Rust 端 `save_dimensions_template`，导致模板中残留已删除维度。

**改动**：调用 `invoke` 前过滤：

```ts
const active = draft.value.filter(d => !d.deleted);
await invoke("save_dimensions_template", { rootPath: props.rootPath, dimensions: active });
```

Rust 端不改，校验逻辑不变。

### 2. "Save as template" 按钮与分隔符间距

**现状**：`|` 分隔符与按钮紧贴（第 232-233 行）。

**改动**：按钮加 `ml-2xs`（使用 `--spacing-2xs` token）：

```html
<button
  data-test="save-as-template"
  class="ml-2xs text-secondary font-semibold ..."
```

### 3. Role 维度右侧面板信息卡片

**现状**：`commitments:goals` 在分割线下有信息卡片（第 458-461 行），`commitments:role` 没有，留空。

**改动**：在 `commitments:goals` 卡片 `</template>` 后（第 462 行之后），添加镜像块：

```html
<template v-if="selectedDimension.source === 'commitments:role'">
  <div class="border border-[var(--color-border-form)] rounded-[var(--radius-form-lg)] bg-[var(--color-surface-muted)] p-md">
    <p class="text-secondary text-[var(--color-text-muted)]">Values are derived from commitment roles.</p>
  </div>
</template>
```

结构与 `commitments:goals` 完全一致，仅文字替换 `goals` -> `roles`。

---

## 边界情况

- 模板保存时如果所有维度都已删除（`active` 为空数组），Rust 端序列化 `Template { dimensions: [] }` 写入 `dimensions.template.yaml`——这是合法的空模板。
- `ml-2xs` 仅影响视觉间距，不改变任何交互逻辑。
- Role 信息卡片不影响已有的 `commitments:goals` 卡片和 `static` 的 values 编辑区；三者通过各自的 `v-if` 互斥。

## 不改变

- Rust 端 `save_dimensions_template` 签名和行为
- 前端 store、types、其他组件
- 已有测试逻辑
