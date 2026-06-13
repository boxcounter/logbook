# Required Dimensions — Design Spec

**Date**: 2026-06-13
**Status**: Approved

## Overview

Entry dimensions 分为必填（required）和选填（optional）。必填维度在录入 Entry 时必须设值，否则阻止提交并提示用户。

## 数据模型

### config.yaml

```yaml
dimensions:
  - name: Business line
    key: business-line
    source: static
    values: [Slax Reader, Community]
    required: true          # 新增字段
  - name: Category
    key: category
    source: static
    values: [Meeting, Deep Work, Review]
    required: true
  - name: Goal
    key: goal
    source: monthly
    # required 省略，默认 false
```

### Rust `Dimension` (models.rs)

```rust
pub struct Dimension {
    pub name: String,
    pub key: String,
    pub source: String,
    pub values: Option<Vec<String>>,
    #[serde(default)]  // false
    pub required: bool,
}
```

### TypeScript `Dimension` (types.ts)

```ts
interface Dimension {
  name: string;
  key: string;
  source: "static" | "monthly";
  values?: string[];
  required: boolean;  // default false from serde
}
```

### 向后兼容

`required` 默认 `false`（serde `default`），现有 config.yaml 无需修改。

## 后端校验

### append_entry

`commands.rs` 的 `append_entry` 在 parse 之后、写文件之前校验：

- newEntry.dimensions 必须包含所有 `required: true` 的维度 key
- 缺失时返回 `Err("Missing required dimension: Business line")`
- 前端在 try/catch 中捕获，显示在输入框下方的红色 error 区（现有逻辑，无需新 UI）

### update_entry

仅当 `update.dimensions` 为 `Some` 时校验。不改 dimensions 时不拦。

### 不新增 config 校验

现有 `MissingValues` / `ValuesEmpty` 校验已覆盖配置错误场景，无需针对 `required` 新增校验项。

## 前端交互

### @ 菜单循环

**当前**：`@ → 维度列表 → 选维度 → 值列表 → 选值 → 关闭菜单`

**改为**：选值后回到维度列表，而非关闭菜单。

```
@ → 维度列表
  → 选 "Business line" → 值列表（Slax Reader / Community）
    → 选 "Slax Reader" → 回到维度列表（Business line ✓）
  → 选 "Category" → 值列表（Meeting / Deep Work / Review）
    → 选 "Meeting" → 回到维度列表（Category ✓）
  → Enter → 关闭菜单（全部必填已填）
```

**维度列表的状态显示：**

| 状态 | 显示 |
|------|------|
| 已填值 | 绿色 `值 ✓` |
| 未填 + required | 红色 `required` |
| 未填 + optional | 无标记 |

**底部 footer：**

- 全部必填已填 → 绿色 `All required ✓ · Enter to confirm`
- 有必填未填 → 灰色 `N required remaining`

**键盘操作（不变）：**

- `1-9` 快速选中
- `↑↓` / Ctrl+N,P 在当前列表内移动高亮
- `Enter`：维度层 → 进值列表；值列表层 → 选值并回到维度列表；全部必填已填时 → 关闭菜单
- `Esc` 随时关闭（不管必填是否齐全 —— 关闭后走缺失 chip 流程）
- `Backspace`（值列表层 filter 为空时）回到维度列表

**细节：**

- 已填维度可重新选（覆盖旧值）
- 点击 chip × 清除值后，下次 @ 打开时该维度恢复未填状态
- 选填维度仍出现在列表中，可选择但非强制

### 缺失必填维度反馈

提交时（点 Log / Enter 且不在菜单中）如果必填维度缺失：

1. 输入框下方出现红色虚线 chip，格式：`+ DimensionName`
2. 点击 chip → 弹出 @ 菜单，直接进入该维度的**值选择层**（跳过维度列表）
3. 选值后：
   - 如还有缺失 → 菜单回到维度列表（继续循环）
   - 全部填完 → 菜单关闭
4. Chip 自动消失（dimValues 更新后 reactive 掉）
5. 用户再次 Enter/Log 提交

### DimensionPanel

- 必填维度 label 旁加红色 `*`（例：`Business line *`）
- 底部一行图例：`* required`
- Chips 行、下拉框交互、展开/收起逻辑不变

## 不变的部分

- `update_entry` dialog 不做 required 校验（entry 可能在 required 机制引入前创建）
- `EntryItem.vue`、`SummaryBar`、`CommitmentsPanel` 不变
- `lastDimensions` 行为不变（记录上次提交的值，下次自动回填）
- 文件格式不变

## 涉及文件

| 文件 | 变更 |
|------|------|
| `src-tauri/src/models.rs` | Dimension 加 `required` 字段 |
| `src-tauri/src/commands.rs` | `append_entry` / `update_entry` 加 required 校验 |
| `src-tauri/src/config.rs` | 测试用例适配（加 `required: false`） |
| `src/types.ts` | Dimension interface 加 `required` |
| `src/components/EntryInput.vue` | @ 菜单循环、缺失 chip、红色虚线 chip UI、footer 状态 |
| `src/components/DimensionPanel.vue` | 必填 label 加 `*`、底部图例 |

## 测试要点

- Rust: required 维度缺失时 `append_entry` 返回 error
- Rust: required 维度齐全时正常写入
- Rust: `update_entry` dimensions 为 Some 时校验，None 时不校验
- 前端: @ 菜单选值后回到维度列表，非关闭
- 前端: 全部必填填满后 Enter 关闭菜单
- 前端: 缺失必填时点 Log 显示红色虚线 chip
- 前端: 点击红色 chip 弹出值选择层
- 前端: DimensionPanel 显示 `*` 和图例
