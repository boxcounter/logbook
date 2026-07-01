# Dimension 交互修复

## 背景

Entry 编辑流程中的两个 dimension 相关 UX 问题：

1. **EntryRowEdit 展示统一的 "+ tag" 按钮**，而非像 EntryComposer 那样逐个展示缺失 dimension 的命名提示。"tag" 标签用词不当——Logbook 的概念是 dimension，不是 tag。
2. **DimensionPopover 展示了已删除的 dimension**。`deleted: true` 的 dimension 在展示链的各个环节都没有被过滤。

## 方案

### 问题 1：EntryRowEdit 维度展示

**目标**：EntryRowEdit 的维度展示与 EntryComposer 一致。

**当前**：
```
[已填充 chip] [+ tag]
```

**改为**：
```
[已填充 chip] [缺失必填提示] [+ 按钮（仅当存在未填充非必填 dimension 时出现）]
```

**`src/components/composite/EntryRowEdit.vue` 改动**：

1. **缺失必填 dimension 提示**——已填充 chip 之后，渲染每个缺失必填 dimension 为虚线命名提示（样式与 EntryComposer 第 192-203 行一致）。点击提示打开 DimensionPopover。原来第 183-187 行的 warning text（"Required: ..."）在提交被拦截时显示，改为视觉提示后成为冗余——移除。

2. **"+ tag" → "+"**——将硬编码的 "+ tag" 按钮替换为条件渲染的 "+" 按钮，仅在存在未填充**非必填** dimension 时显示（`props.dimensions.filter(d => !d.deleted && !d.required && !dimValues[d.key]).length > 0`）。标签仅 "+"，不缀 "tag"。样式保持不变（虚线边框），点击打开 DimensionPopover。

3. **移除 submit warning**（第 183-187 行）。逐 dimension 的视觉提示 + save 的 hard-block 已经足够表达"缺失必填 dimension"；独立的 warning text 不再需要。

### 问题 2：过滤已删除 dimension

**目标**：在所有面向用户的展示中排除 `deleted: true` 的 dimension，但保留 DimensionEditorModal 中的可见性（该组件有自己的 "Show deleted" 开关）。

**改动**：

| 文件 | 位置 | 改动 |
|---|---|---|
| `DimensionPopover.vue` | 第 147 行 `v-for` | 加 `v-if="!d.deleted"` |
| `EntryComposer.vue` | 第 32-33 行 `filledDims` / `missingRequired` | filter 加 `&& !d.deleted` |
| `EntryRowEdit.vue` | 第 103、107 行 `missingRequired` / `filled()` | filter 加 `&& !d.deleted` |

**为什么不在数据源过滤？** DimensionEditorModal 需要完整列表（含已删除 dimension）来实现 "Show deleted" 开关。在每个消费组件中显式过滤，意图清晰，出问题容易定位。当前只有 3 个消费点，不构成维护负担。

## 不改的部分

- `EntryRow.vue`（展示模式）`filledDims`——不修改。展示模式只显示已填充 dimension；已删除但旧 entry 仍有值的 dimension 会继续渲染。这是有意为之：不丢失历史数据可见性。
- `dimensionColor.ts`——已在第 18 行过滤已删除 dimension 用于颜色分配。无需修改。
- `DimensionEditorModal.vue`——已正确处理已删除 dimension 的显示/隐藏开关。无需修改。

## 测试要点

- **DimensionPopover**：已删除 dimension 不在列表中显示
- **EntryComposer**：已删除 dimension 不出现在 filled chip，也不作为缺失必填提示展示，不在 @ 菜单中可选
- **EntryRowEdit**：已删除 dimension 不出现在 filled chip，也不作为缺失必填提示展示；"+" 按钮的显示条件排除已删除 dimension（即仅统计未删除且未填充的非必填 dim）；所有非必填 dim 均已填充或删除时 "+" 隐藏
- **EntryRow（展示模式）**：历史 entry 中已删除 dimension 的值仍然显示
- **DimensionEditorModal**："Show deleted" 开关行为不变
