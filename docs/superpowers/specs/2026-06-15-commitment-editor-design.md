# Commitment Editor — Design Spec

> 日期：2026-06-15 | 状态：design-approved

## 背景

Logbook 的 Commitment（月度承诺）当前存储在 `_monthly.md` YAML frontmatter 中，通过 `open_in_editor` 在系统编辑器中手动编辑。两个痛点：

1. **YAML 格式易出错**：缩进、格式错误导致解析失败
2. **上下文切换**：离开 App → 开编辑器 → 找文件 → 改完保存

本设计为 CommitmentsPanel 增加内联编辑能力，无需手写 YAML、无需离开 Logbook。

## 范围

- Role 增删、Allocation 修改、Goal 增删改名
- 编辑态内联在 CommitmentsPanel 中，整面板编辑 + 显式保存/取消
- Goal 改名时批量更新当月 entry；Goal 删除如有 entry 引用则拒绝
- 前端预校验 + 后端完整校验

## 不在范围

- 从上一月复制 commitments（未来迭代）
- Goal 重排/排序
- 多角色之间移动 Goal

## 数据流

```
CommitmentsPanel (展示态)
  → 点击"编辑"
  → CommitmentsPanel (编辑态) — 整面板可编辑
  → 增删改 role/goal/allocation
  → 保存
  → invoke('set_commitments', { root_path, year, month, commitments })
  → Rust:
      1. 校验（role 非空、allocation > 0、goal 非空、同 role 下 goal 唯一）
      2. 检测 goal 改名 → 批量更新当月 entry 的 dimensions.goal
      3. 检测 goal 删除 → 有 entry 引用则拒绝
      4. 原子写入 _monthly.md
      5. 返回 Vec<Commitment>
  → 前端更新 store
  → 回到展示态
```

取消时恢复编辑前快照，不调后端。

## 新增 Rust Command

```
set_commitments(root_path: String, year: i32, month: u32, commitments: Vec<Commitment>)
  → Result<Vec<Commitment>, String>
```

### 校验规则

| 条件 | 错误信息 |
|------|---------|
| role 名为空 | `"Role name cannot be empty"` |
| allocation 为 0 | `"Allocation for '{role}' must be greater than 0"` |
| goal 名为空 | `"Goal name cannot be empty"` |
| 同 role 下 goal 重复 | `"Goal '{goal}' already exists in '{role}'"` |
| 删除有 entry 的 goal | `"Cannot delete goal '{goal}': used by N entries this month"` |
| commitments 为空数组 | `"At least one role is required"` |

### Goal 改名逻辑

保存时 Rust 端先读取当前 `_monthly.md` 获取旧 commitments，对比新旧，检测 goal 文本变更（旧 goal 名 → 新 goal 名，非新增、非删除 → 改名）。然后遍历当月所有 day file，替换匹配的 `dimensions.goal` 值。

## 组件设计

CommitmentsPanel.vue 增加编辑态，两种状态共用同一组件。

### 展示态（现有逻辑）

- 每个 role：allocation 进度条 + goal 列表及耗时
- 底部增加 `✏️ 编辑` 按钮（小号、低调）

### 编辑态（新增）

```
┌─ Commitments ──────────────────────────┐
│                                         │
│  Role: [Developer        ]  Alloc: [80] │
│    Goal: [Slax Reader 功能开发    ✕]    │
│    Goal: [代码审查                ✕]    │
│    + 添加 Goal                          │
│  ─────────────────────────────────────  │
│  Role: [Tech Lead        ]  Alloc: [40] │
│    Goal: [1:1                    ✕]    │
│    Goal: [架构评审                ✕]    │
│    + 添加 Goal                          │
│                                         │
│  + 添加 Role                            │
│                                         │
│  [取消]                    [保存]       │
└─────────────────────────────────────────┘
```

- Role 最少 1 个，删到仅剩 1 个时不显示删除按钮
- 前端预校验：空 role、零 allocation、空 goal → 保存前拦截
- 保存中按钮 disabled + loading
- 后端错误通过 toast 展示

### 外部修改冲突

编辑态期间文件监听可能检测到外部变更。如果 `commitments-changed` 事件传入的 commitments 与 `last_saved_commitments` 快照不同 → 提示"文件已被外部修改"，刷新编辑态。

## 测试策略

### Rust 单元测试（`#[cfg(test)]`，无 IO）

- 校验逻辑：空 role、零 allocation、空 goal、重复 goal、空数组
- Goal 改名检测：对比新旧 Vec<Commitment>
- Entry 批量替换：给定 mock DayFile 验证替换结果

### Rust 集成测试（`tests/`，真实文件系统）

- 写入 `_monthly.md` → 读取验证
- Goal 改名 → 当月 entry 的 `dimensions.goal` 被更新
- Goal 删除被拒（有 entry 引用时）
- Goal 删除成功（无 entry 引用时）
- 空 commitments 拒绝
- 空 `_monthly.md` 写入首批 commitments

### 前端组件测试（Vitest + vue-test-utils）

- 展示态 → 编辑态切换
- 增删 role、增删 goal、改 allocation
- 前端预校验：空值拦截
- 取消恢复快照
- 保存中 loading/disabled
- 后端错误 toast
- 外部修改冲突提示
