# CommitmentsModal 两栏布局重构

## 动机

当前 CommitmentsModal 单列布局在角色/目标较多时存在两个问题：

1. **滚动开销**：目标多的长列表需要频繁滚动，特别是跨角色对比分配额度时视线不断跳动。
2. **数据丢失焦虑**：视口外的未保存内容多，用户心理安全感不足——虽然脏检保证了技术安全。

两栏 master-detail 解决这两个问题：左侧角色列表固定不动提供总览 + 导航，右侧编辑当前选中角色，一屏可见无需滚动。

## 设计

### 布局

与 DimensionEditorModal 一致的两栏结构：

```
┌─────────────────────────────────────────────┐
│  Header: "Edit Commitments" | month label    │
│          Committed Xh | Logged Xh             │
├────────────┬────────────────────────────────┤
│            │                                │
│ Left panel │  Right panel                   │
│ ~200px     │  flex-1                        │
│            │                                │
│ Role 1     │  Role name input               │
│ Role 2  ✓  │  Allocation stepper (+/- 5h)   │
│ Role 3     │                                │
│            │  Goal list:                     │
│ + Add Role │    Goal 1  [     ] Xh logged    │
│            │    Goal 2  [     ] Xh logged    │
│            │    + Add Goal                   │
├────────────┴────────────────────────────────┤
│  Footer: [Cancel] [Save]                     │
└─────────────────────────────────────────────┘
```

### 左侧面板

- **角色列表**：每行显示角色名 + `Xh / Yh`（已记录 / 已分配），和当前 `CommitmentsPanel` 侧栏一致。
- **选中态**：高亮选中行，`selectedIndex` 驱动右侧面板内容。
- **拖拽排序**：`VueDraggable`，handle 为整行。
- **"+ Add Role" 按钮**：点击后追加空角色到列表末 → 自动选中 → 焦点进右侧面板角色名输入框。
- **删除**：带已记录目标（`logged > 0`）的角色，删除置灰不可点（语义不变）。
- **键盘导航**：左侧面板持有焦点时 `↑`/`↓` 切换选中角色（循环）。右侧输入框获得焦点时 `↑`/`↓` 被输入框消费，不冒泡。
- **空状态**：角色列表为空时右侧面板显示 "No roles yet. Add a role to get started."。

### 右侧面板

选中角色时：

- **角色名**：可编辑文本输入框，面板顶部。
- **分配额度**：步进器 `+`/`-`（步长 5h），显示格式 `Xh allocated`。
- **目标列表**：
  - 每行：目标名称输入框 + 已记录时长（只读 `Xh logged`）。
  - 输入框内按 `Enter`（IME-safe：`isComposing` 守卫）追加新空白目标在下方，焦点进新输入框。
  - 独立拖拽排序（`VueDraggable`，handle 为拖拽 icon），与左侧角色拖拽互不干扰。
  - 删除：已记录时长 > 0 的目标，删除置灰不可点（逻辑不变）。
- **"+ Add Goal" 链接按钮**：列表底部，追加空目标。

### 数据流

- **统一草案**：`draft` 为深拷贝的 `commitments` 数组，`reactive()`。左侧面板和右侧面板共享同一个 `draft`。
- **切换角色**：只改变 `selectedIndex`，无副作用。右侧输入框 `v-model` 绑定的数据已在 `draft` 中，切换后切回来内容仍保留。
- **保存**：`Cmd/Ctrl+Enter` 整体校验 + 提交。切换选中角色不触发任何保存。
- **脏检**：`isDirty = JSON.stringify(draft) !== JSON.stringify(original)`，和当前一致。
- **关闭**：`Esc` / 点击遮罩 / Cancel 按钮 → `requestClose()` → 脏时弹出确认「Discard changes? / Keep editing / Discard」，不脏直接关。

### 校验（不变）

- 角色名不能为空
- 角色名不能重复
- 目标名全局唯一（跨所有角色）
- 空目标且 logged == 0 → 保存时静默剔除

### 组件架构

- `RoleCard` 不再需要（删除）。
- CommitmentsModal 不拆分独立 `RoleEditor` 子组件——所有模板逻辑内联在 CommitmentsModal 中。`selectedIndex` 和 `draft` 在同一个组件作用域内管理，避免跨组件传参引入双向绑定复杂度和 dirty 追踪问题。

### 与 DimensionEditorModal 的差异（保留）

| 方面 | CommitmentsModal | DimensionEditorModal |
|------|-----------------|---------------------|
| 删除模型 | Hard delete（splice） | Soft delete（`deleted: true` + toggle） |
| Save as template | 无 | 有 |
| Source 类型切换 | 无 | 有（static / commitments:goals / commitments:role） |
| 子组件 | 无（全内联） | 无（全内联） |

## 不纳入范围

- 外部文件变更检测（不一致）
- 自动保存 / per-role 保存（明确排除）
- 过渡动画（两个 Modal 均无）

## 影响范围

- **修改**：`src/components/composite/CommitmentsModal.vue`
- **删除**：`src/components/composite/RoleCard.vue`（如无其他引用）
- **不涉及**：`CommitmentsPanel.vue`（调用方接口不变）、Rust 后端、IPC 协议

## 备选方案评估

备选方案「保持单列」因无法解决上述两个核心痛点（滚动 + 心理安全感）被排除。

备选方案「per-role 自动保存 + 两栏」因增加保存语义复杂性（部分保存 vs 整体保存的冲突、失败回滚、外部文件覆盖检测等）被排除——用户选择整体保存。
