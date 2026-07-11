# DimensionPopover 自动关闭：改为「所有可填维度填完」

日期：2026-07-11
状态：待实现

## 背景与根因

用户现象：3 必选 + 1 可选维度全填完后，`DimensionPopover` 不自动关闭。

根因（已用单元测试 + 用户真实配置双重确认，非推测）：

`DimensionPopover.vue` `selectVal`（L130-144）的 auto-close 判断用：

```ts
const allFilled = props.dimensions
  .filter(d => d.required)                                    // ← 只过滤 required
  .every(d => d.key === justFilledKey || props.dimValues[d.key]);
```

这里 `props.dimensions.filter(d => d.required)` **未排除 `deleted: true` 的维度**。而 UI 渲染用的是 `visibleDims`（L23，`props.dimensions.filter(d => !d.deleted)`），两者过滤口径不一致。

用户真实配置（`2026/07/dimensions.yaml`）中有 3 个 `deleted: true` 且 `required: true` 的维度（Importance-Urgency、Business Line、Category）。这些维度：

- 在 UI 不显示，用户看不见、无法填写；
- 但 `allFilled` 仍然把它们计入判断，且它们永远没有值 → `allFilled` 永远为 false → popover 永不自动关闭。

同类缺陷：一个 `required: true` 但可选值列表为空的维度（例如 commitments 未配置 goals 时的 `commitments:role:goals` 源维度），同样会永远阻断关闭，因为用户无法为它选值。

## 目标

1. 修复 deleted-required / 空值-required 阻断 auto-close 的 bug。
2. 将 auto-close 语义从「所有 **required** 维度填完」改为「所有**可见且可选值非空**的维度填完」——符合用户「全部填完才关」的意图。

## 非目标

- 不改父组件（EntryComposer / EntryRowEdit）对 `close` 事件的处理逻辑。
- 不改 popover 的打开触发、键盘导航、阶段切换、Esc/click-outside 关闭路径。
- 不改 `firstUnfilledIndex`（它已正确使用 `visibleDims` 口径）。
- 不改维度配置 schema 或校验（不禁止 deleted+required 组合，这是合法的向后兼容字段）。

## 设计

改动集中在 `DimensionPopover.vue` 单文件，只动 `selectVal` 的判断条件。

### 判定口径变更

**新增**一个纯函数，封装「该维度是否有可选值」的判定。判定按 source 分三种，与 `activeValues` computed（L67-100）的取值来源保持一致：

| source | 有可选值当且仅当 |
|--------|------------------|
| `commitments:role`，或伪 `role` key（无 role 维度时） | `props.commitments.length > 0` |
| `commitments:role:goals` | `goalOptions.value.length > 0`（复用已有 computed，L47-51） |
| 其他（含 `static`） | `(d.values ?? []).length > 0` |

「可选值为空的维度」不参与 auto-close 计数——用户无法为它选值，若计入会永远阻断关闭。

### selectVal 改动

```ts
function selectVal(value: string) {
  if (!selectedDimKey.value) return;
  const justFilledKey = selectedDimKey.value;
  emit("select", justFilledKey, value);

  const allFillableFilled = visibleDims.value
    .filter(d => d.key !== justFilledKey && !props.dimValues[d.key]) // 还没填的可见维度
    .every(d => !hasFillableValues(d));                              // 每个未填项都无可选值 → 无需再填

  if (allFillableFilled) {
    emit("close");
  } else {
    stage.value = "dim";
    selectedDimKey.value = null;
    highlightedIndex.value = firstUnfilledIndex(justFilledKey);
  }
}
```

变更点：
- `props.dimensions.filter(d => d.required)` → `visibleDims.value.filter(...)`：口径与 UI 一致，排除 deleted。
- `.every(d => d.required ...)` → `.every(d => !hasFillableValues(d))`：从「只看 required」改为「看是否还有可选值未填」。所有可见维度（含 optional）都要填完才关，且无可选值的维度不计入。
- `justFilledKey` 仍通过 filter 排除的方式视为已填（与原实现一致，因为 emit 后 prop 尚未更新）。

### hasFillableValues 辅助函数

```ts
// 是否有可选值。与 activeValues 的取值来源口径一致，仅用于 auto-close 判定，
// 不负责实际渲染值列表。
function hasFillableValues(d: Dimension): boolean {
  if (d.source === "commitments:role") {
    return props.commitments.length > 0;
  }
  if (d.source === "commitments:role:goals") {
    return goalOptions.value.length > 0;
  }
  return (d.values ?? []).length > 0;
}
```

边界说明：`allFillableFilled` 遍历 `visibleDims`（真实 `Dimension` 对象），不包含「无 role 维度时的伪 role 条目」（它在模板里单独渲染，`data-test="dim-role"`，不在 `visibleDims` 内）。因此伪 role 的填充状态不参与 auto-close 判定——这与 `firstUnfilledIndex` 的口径一致（它也只看 `visibleDims`）。伪 role 始终是 optional，不阻断关闭属于可接受行为。

## 行为变化（tradeoff）

| 场景 | 改前 | 改后 |
|------|------|------|
| deleted-required 阻断 | 永不自动关闭（bug） | 不再计入，正常关闭 |
| 空可选值 required 阻断 | 永不自动关闭（bug） | 不再计入，正常关闭 |
| 只填必填、optional 未填 | 必填填完即关 | **不关，需手动 Esc** |
| 全部可见维度填完 | 若 required 已齐即关（optional 不影响） | 填完最后一个可见维度即关 |

第三行是用户明确接受的行为变化。

## 测试计划（TDD，先红后绿）

在 `src/__tests__/components/DimensionPopover.test.ts` 新增以下用例（现有用例全部应保持通过，其中 L51「emits close once all required dimensions are filled」需改为反映新语义——见下）：

1. **deleted-required 不阻断关闭**：dimensions 含 4 个 required（非 deleted）+ 3 个 deleted-required + 1 个 optional；填完所有可见 required 后，再填 optional → emit close。
2. **空可选值维度不阻断关闭**：dimensions 含一个 `values: []` 的 optional + 正常 required；填完 required 即关（空值 optional 不计入）。
3. **optional 未填时不关**（锁定新行为，防回归）：dimensions 含 1 required（有值）+ 1 optional（有值）；只填 required → 不 emit close；再填 optional → emit close。
4. **现有 L51 用例的语义已变化**：原用例 `category`（required）+ `goal`（required）两个必填，选完 goal 后两者都填完即关——新语义下行为一致（都是 required 且都填完），用例仍应通过，无需改动。

实现顺序：先写用例 1-3（用例 3 此时应为红——改前必填填完即关，与断言矛盾），跑红 → 改 `selectVal` → 跑绿 → 跑全量测试确认无回归。

## 影响范围

- 改动文件：`src/components/DimensionPopover.vue`（`selectVal` + 新增 `hasFillableValues`）
- 测试文件：`src/__tests__/components/DimensionPopover.test.ts`（新增 3 个用例）
- 不触碰：EntryComposer.vue、EntryRowEdit.vue、useClickOutside.ts、Rust 端、配置 schema
