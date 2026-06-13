# @-mention 维度选择 UX 改进

**日期**：2026-06-13
**状态**：design-approved

## 问题

当前输入框按 `@` 弹出维度选择菜单存在 5 个 UX 问题：

1. 选择维度（dim phase）和选择值（val phase）的菜单视觉几乎一样，用户分不清当前在哪个阶段
2. 红色 "required" 文字太抢眼，有催促感；右侧灰色 badge 重复左侧已有的维度名，信息冗余
3. 值列表有数字序号，但按数字键不会筛选、而是直接选中该项，与直觉不符
4. val phase 无法回到 dim phase（只有空 filter 时 Backspace 可用，无可见入口）
5. 选完一个值回到 dim phase 时，选中项总是第 0 项（已填的维度），而非下一个未填的维度

## 设计

不改变两阶段菜单交互骨架，做 6 个就地修改。

### 改动 1：dim vs val 视觉区分

**dim phase**：
- 顶栏：深色背景（`bg-gray-800`），左侧放 "DIM" 小标签（`bg-gray-600` 圆角 badge），右侧文字 "Pick a dimension"
- 每行：左侧 3px 宽颜色条（每个维度固定颜色：Goal 蓝、Business Line 橙、Category 绿、Import/Urg 粉）+ 维度名 + 右侧辅助信息

**val phase**：
- 顶栏：浅蓝背景（`bg-blue-50`），左侧 `←` 返回按钮（加粗，可点击），右侧文字 "Pick a value for **DimensionName**"
- 每行：纯文字，左侧留 4px padding，无颜色条、无序号

### 改动 2：去掉数字序号、去掉右侧重复名称

- 删除所有数字序号 badge（`[1] [2] [3]...`）
- dim phase 用左侧颜色条替代序号的视觉锚点作用
- 右侧灰色 badge 改为显示有用信息：未填时显示值数量（如 "12 values"），已填时显示绿色值名 + ✓（保留现有逻辑）
- 不再重复显示维度名

### 改动 3：required 弱化

去掉红色 "required" 文字。必填信息通过三层安静的方式传达：

1. **左侧颜色条** = 这个维度存在（所有维度都有），存在即暗示需要关注
2. **绿色 ✓** = 已填，表示这个维度搞定了
3. **底部灰色圆点** = 进度指示器，如 `●●○` 表示 4 个必填中填了 2 个（替代文字 "2 required remaining"）

### 改动 4：val phase 返回入口

- val phase 顶栏左侧加 `←` 按钮，点击回到 dim phase
- val phase 底部 footer 加提示 "← Back to dimensions · Type to filter"
- 保留现有 Backspace（空 filter 时）回到 dim phase 的键盘行为
- 鼠标和键盘用户各有一条回到上层的路径

### 改动 5：数字键不再快捷选择

- 删除 keydown handler 中 1-9 数字键的快捷选择逻辑
- 数字输入正常流入 filter 文字，与其他字符一致
- 既然去掉了数字序号 badge，数字键的特殊行为就没有视觉依据了

### 改动 6：自动串联时选中第一个未填维度

- 选完一个维度的值后，`openDimMenu()` 自动将 `selectedIndex` 定位到第一个未填的 required 维度
- 已填的维度按 dim phase 列表顺序跳过
- 用户可连续按 Enter 完成所有必填维度，无需手动 ArrowDown

## 不改的部分

- 自动串联：选完一个维度的值 → 自动弹出下一个未填维度
- Enter / Tab / Ctrl+J 确认选择
- Escape / Ctrl+[ 关闭菜单并清除 @mention
- Arrow 键、Ctrl+N/P 导航
- 点击菜单外关闭
- 输入框下方的 chip 行（已填 chip + 未填虚线 chip）
- 两个菜单项之间的过滤和选择逻辑（`getMenuItems()`、`extractFilterFromInput()` 等）

## 实现要点

主要修改文件：`src/components/EntryInput.vue`

### 模板变更

- dim phase header：`bg-gray-800 text-gray-200`，加 "DIM" badge
- val phase header：`bg-blue-50 text-blue-600`，加 `←` 按钮，点击调用 `goBackToDim()`
- 菜单行：删除序号 `<span>`，dim phase 加颜色条 `<span>`（3px 宽，按维度 key 映射颜色）
- 右侧信息：删除 `item.sub` badge；未填时显示 `<dim.values.length> values`
- footer：text 版改为圆点版

### 脚本变更

- 新增 `goBackToDim()`：从 val phase 回到 dim phase
- 修改 `openDimMenu()`：接受可选参数 `skipFilled?: boolean`，为 true 时 `selectedIndex` 跳到第一个未填的 required 维度
- 删除 keydown handler 中 1-9 数字键分支
- 保留 Backspace 回退逻辑不变
- 确认选中后调用 `openDimMenu({ skipFilled: true })` 而非 `openDimMenu()`

### 样式

继续使用 Tailwind 4 工具类，不新增 CSS 文件。颜色映射参考现有 `chipClass()` 中的维度颜色：

```ts
const dimColors: Record<string, string> = {
  goal: 'bg-blue-500',
  'business-line': 'bg-amber-500',
  category: 'bg-green-500',
  'importance-urgency': 'bg-pink-500',
};
```
