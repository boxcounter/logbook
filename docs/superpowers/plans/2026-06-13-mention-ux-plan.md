# @-mention 维度选择 UX 改进 — 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 @-mention 菜单的 5 个 UX 问题：dim/val 阶段区分不清、required 标签刺眼、数字序号无用、返回入口缺失、自动串联未跳转未填维度。

**Architecture:** 所有改动集中在 `src/components/EntryInput.vue`。不改交互骨架（两阶段菜单 + 自动串联保留），只改呈现和行为。纯 Tailwind 4 工具类，不新增 CSS 文件。

**Tech Stack:** Vue 3 (composition API) + TypeScript + Tailwind CSS 4

**Spec:** `docs/superpowers/specs/2026-06-13-mention-ux-design.md`

---

## 文件结构

| 文件 | 职责 | 变更类型 |
|------|------|----------|
| `src/components/EntryInput.vue` | 输入框 + @-mention 菜单 + 键盘导航 + chip 行 | 修改（6 处脚本 + 模板重写） |

---

### Task 1: 新增辅助函数和计算属性

**Files:**
- Modify: `src/components/EntryInput.vue`（`<script setup>` 区域，插入新代码）

- [ ] **Step 1: 添加 `dimBarColor` 函数**

在 `chipClass` 函数（约第 83 行）之后插入：

```typescript
// ---- Color bar for dim phase rows ----
const dimBarColor = (key: string): string => {
  const map: Record<string, string> = {
    goal: 'bg-blue-500',
    'business-line': 'bg-amber-500',
    'importance-urgency': 'bg-pink-500',
    category: 'bg-green-500',
  };
  return map[key] || 'bg-gray-400';
};
```

- [ ] **Step 2: 添加 `totalRequiredDims` 计算属性**

在 `requiredRemaining` 计算属性（约第 67 行）之后插入：

```typescript
const totalRequiredDims = computed(() =>
  props.dimensions.filter(d => d.required).length
);
```

- [ ] **Step 3: 添加 `getValueCount` 函数**

插入在 `dimBarColor` 函数之后：

```typescript
const getValueCount = (key: string): number => {
  const dim = props.dimensions.find(d => d.key === key);
  if (!dim) return 0;
  if (dim.source === 'monthly') return goalOptions.value.length;
  return dim.values?.length || 0;
};
```

- [ ] **Step 4: 添加 `goBackToDim` 函数**

在 `closeMenu` 函数（约第 182 行）之后插入：

```typescript
/// Go back from val phase to dim phase (reverse of replaceMentionWithDimKey)
function goBackToDim() {
  const val = input.value;
  const cursorPos = inputEl.value?.selectionStart ?? val.length;
  const textBefore = val.slice(0, cursorPos);
  const lastAt = textBefore.lastIndexOf('@');
  if (lastAt === -1) return;
  const afterAt = val.slice(lastAt);
  const spaceIdx = afterAt.indexOf(' ');
  if (spaceIdx !== -1) {
    // Remove dimKey and space: "@dimKey rest" → "@rest"
    input.value = val.slice(0, lastAt) + '@' + afterAt.slice(spaceIdx + 1);
  }
  menuPhase.value = 'dim';
  activeDimKey.value = null;
  filterText.value = '';
  selectedIndex.value = 0;
}
```

- [ ] **Step 5: 添加 `firstUnfilledRequiredIndex` 函数**

在 `goBackToDim` 函数之后插入：

```typescript
/// Return index of first unfilled required dimension in getMenuItems(), or 0
function firstUnfilledRequiredIndex(): number {
  const items = getMenuItems();
  const idx = items.findIndex(
    item => item.required && item.key && !dimValues.value[item.key]
  );
  return idx === -1 ? 0 : idx;
}
```

- [ ] **Step 6: 编译验证**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook && npm run build 2>&1 | tail -5
```

Expected: BUILD SUCCESS（无报错）

- [ ] **Step 7: Commit**

```bash
git add src/components/EntryInput.vue
git commit -m "feat(mention-ux): add dimBarColor, goBackToDim, firstUnfilledRequiredIndex helpers"
```

---

### Task 2: 修改 `openDimMenu` 支持 `skipFilled`，更新调用点

**Files:**
- Modify: `src/components/EntryInput.vue`（`openDimMenu` 函数 + 3 处调用点）

- [ ] **Step 1: 修改 `openDimMenu` 签名和逻辑**

将第 127-133 行的：

```typescript
function openDimMenu() {
  menuPhase.value = "dim";
  activeDimKey.value = null;
  selectedIndex.value = 0;
  filterText.value = "";
  menuVisible.value = true;
}
```

替换为：

```typescript
function openDimMenu(skipFilled: boolean = false) {
  menuPhase.value = 'dim';
  activeDimKey.value = null;
  filterText.value = '';
  menuVisible.value = true;
  selectedIndex.value = skipFilled ? firstUnfilledRequiredIndex() : 0;
}
```

- [ ] **Step 2: 更新 `confirmSelection` 中 val phase 的调用（第 204 行）**

将：

```typescript
      openDimMenu();      // loop back to dimension list
```

改为：

```typescript
      openDimMenu(true);  // loop back, skip to next unfilled
```

- [ ] **Step 3: 更新 `selectByIndex` 中 val phase 的调用（第 223 行）**

将：

```typescript
      openDimMenu();
```

改为：

```typescript
      openDimMenu(true);  // loop back, skip to next unfilled
```

- [ ] **Step 4: 编译验证**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook && npm run build 2>&1 | tail -5
```

Expected: BUILD SUCCESS

- [ ] **Step 5: Commit**

```bash
git add src/components/EntryInput.vue
git commit -m "feat(mention-ux): openDimMenu skipFilled, auto-jump to next unfilled dim"
```

---

### Task 3: 删除数字键快捷选择、更新模板 header

**Files:**
- Modify: `src/components/EntryInput.vue`（keydown handler + template header）

- [ ] **Step 1: 删除 keydown handler 中数字键分支**

删除第 387-392 行：

```typescript
  // Number keys: quick-select (1-9)
  if (e.key >= "1" && e.key <= "9" && !e.ctrlKey && !e.metaKey) {
    e.preventDefault();
    selectByIndex(parseInt(e.key) - 1);
    return;
  }
```

（保留后续的 Enter/Tab/Escape/Backspace 处理不变）

- [ ] **Step 2: 替换 dim phase header（第 487-488 行）**

将：

```html
        <div class="px-3 py-1.5 text-[10px] text-gray-400 uppercase tracking-wide border-b border-gray-100">
          <template v-if="menuPhase === 'dim'">Pick a dimension</template>
          <template v-else>Pick a value for <b>{{ activeDimKey ? (DIM_ALIASES[activeDimKey] || activeDimKey) : '' }}</b></template>
        </div>
```

替换为：

```html
        <!-- dim phase header -->
        <div
          v-if="menuPhase === 'dim'"
          class="px-3 py-1.5 text-[10px] uppercase tracking-wide border-b border-gray-100 bg-gray-800 text-gray-200 flex items-center gap-2"
        >
          <span class="bg-gray-600 px-1.5 py-0.5 rounded text-[9px] font-medium">DIM</span>
          Pick a dimension
        </div>
        <!-- val phase header -->
        <div
          v-else
          class="px-3 py-1.5 text-[10px] border-b border-gray-100 bg-blue-50 text-blue-600 flex items-center gap-2"
        >
          <button type="button" class="font-bold text-xs hover:text-blue-800 leading-none" @click="goBackToDim">&larr;</button>
          <span>Pick a value for <b class="text-blue-800">{{ activeDimKey ? (DIM_ALIASES[activeDimKey] || activeDimKey) : '' }}</b></span>
        </div>
```

- [ ] **Step 3: 编译验证**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook && npm run build 2>&1 | tail -5
```

Expected: BUILD SUCCESS

- [ ] **Step 4: Commit**

```bash
git add src/components/EntryInput.vue
git commit -m "feat(mention-ux): remove number key select, redesign phase headers with visual distinction"
```

---

### Task 4: 重写菜单行（去掉序号、加颜色条、改右侧信息）

**Files:**
- Modify: `src/components/EntryInput.vue`（menu item 行模板，第 491-505 行）

- [ ] **Step 1: 替换菜单行模板**

将第 491-505 行：

```html
        <template v-for="(item, i) in getMenuItems()" :key="i">
          <div
            class="mention-item flex items-center gap-2 px-3 py-1.5 cursor-pointer"
            :class="{ 'bg-blue-50': i === selectedIndex }"
            :data-idx="i"
          >
            <span
              class="text-[10px] rounded w-[18px] h-[18px] inline-flex items-center justify-center flex-shrink-0 tabular-nums"
              :class="i === selectedIndex ? 'bg-blue-600 text-white' : 'bg-gray-100 text-gray-400'"
            >{{ i + 1 }}</span>
            <span class="flex-1">{{ item.label }}</span>
            <span v-if="menuPhase === 'dim' && item.required && !dimValues[item.key || '']" class="text-[10px] text-red-400">required</span>
            <span v-else-if="menuPhase === 'dim' && item.required && dimValues[item.key || '']" class="text-[10px] text-green-500">{{ dimValues[item.key || ''] }} ✓</span>
            <span v-if="menuPhase === 'dim' && item.sub" class="text-[10px] text-gray-400 bg-gray-100 px-1.5 py-0.5 rounded">{{ item.sub }}</span>
          </div>
        </template>
```

替换为：

```html
        <template v-for="(item, i) in getMenuItems()" :key="i">
          <div
            class="mention-item flex items-center gap-2 px-3 py-1.5 cursor-pointer"
            :class="{ 'bg-blue-50': i === selectedIndex }"
            :data-idx="i"
          >
            <!-- Dim phase: colored bar; Val phase: no bar, text indented -->
            <span
              v-if="menuPhase === 'dim'"
              class="w-[3px] h-[22px] rounded-full flex-shrink-0"
              :class="dimBarColor(item.key || '')"
            ></span>
            <span
              class="flex-1"
              :class="{ 'pl-1': menuPhase === 'val' }"
            >{{ item.label }}</span>
            <!-- Dim phase right-side info -->
            <span
              v-if="menuPhase === 'dim' && item.required && !dimValues[item.key || '']"
              class="text-[10px] text-gray-400"
            >{{ getValueCount(item.key || '') }} values</span>
            <span
              v-else-if="menuPhase === 'dim' && item.required && dimValues[item.key || '']"
              class="text-[10px] text-green-500"
            >{{ dimValues[item.key || ''] }} ✓</span>
          </div>
        </template>
```

- [ ] **Step 2: 编译验证**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook && npm run build 2>&1 | tail -5
```

Expected: BUILD SUCCESS

- [ ] **Step 3: Commit**

```bash
git add src/components/EntryInput.vue
git commit -m "feat(mention-ux): replace number badges with color bars, show value counts, remove sub alias badge"
```

---

### Task 5: 重写 footer（圆点进度指示器）+ val phase footer

**Files:**
- Modify: `src/components/EntryInput.vue`（footer 模板，第 510-517 行）

- [ ] **Step 1: 替换 footer 模板**

将第 510-517 行：

```html
        <div
          v-if="menuPhase === 'dim'"
          class="px-3 py-1 text-[10px] border-t border-gray-100"
          :class="allRequiredFilled ? 'text-green-600' : 'text-gray-400'"
        >
          <template v-if="allRequiredFilled">All required ✓ · Enter to confirm</template>
          <template v-else>{{ requiredRemaining }} required remaining</template>
        </div>
```

替换为：

```html
        <!-- Dim phase footer: dot progress indicator -->
        <div
          v-if="menuPhase === 'dim' && totalRequiredDims > 0"
          class="px-3 py-1.5 text-[10px] border-t border-gray-100 flex items-center gap-1"
          :class="allRequiredFilled ? 'text-green-600' : 'text-gray-400'"
        >
          <template v-if="allRequiredFilled">
            All required ✓ · Enter to confirm
          </template>
          <template v-else>
            <span
              v-for="n in totalRequiredDims"
              :key="n"
              class="inline-block w-[6px] h-[6px] rounded-full"
              :class="n <= (totalRequiredDims - requiredRemaining) ? 'bg-green-400' : 'bg-gray-300'"
            ></span>
            <span class="ml-1">{{ requiredRemaining }} to go</span>
          </template>
        </div>
        <!-- Val phase footer: navigation hint -->
        <div
          v-if="menuPhase === 'val'"
          class="px-3 py-1.5 text-[10px] text-gray-400 border-t border-gray-100"
        >
          &larr; Back to dimensions · Type to filter
        </div>
```

- [ ] **Step 2: 编译验证**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook && npm run build 2>&1 | tail -5
```

Expected: BUILD SUCCESS

- [ ] **Step 3: Commit**

```bash
git add src/components/EntryInput.vue
git commit -m "feat(mention-ux): dot progress indicator in dim footer, nav hint in val footer"
```

---

### Task 6: 手动验证

**Files:** 无（验证步骤）

启动应用并验证以下场景。

- [ ] **Step 1: 启动应用**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook && npm run tauri dev 2>&1 &
```

- [ ] **Step 2: 场景 1 — 初始 @ 触发 dim phase**

1. 清空输入框
2. 按 `@`
3. 验证：弹出的菜单顶栏为深色背景 + "DIM" badge + "PICK A DIMENSION"
4. 验证：每行左侧有颜色条（Goal 蓝、Business Line 橙、Category 绿等）
5. 验证：没有数字序号
6. 验证：右侧显示 "N values"（如 "12 values"）
7. 验证：没有红色 "required" 文字
8. 验证：底部显示灰色和绿色圆点进度 + "N to go"

- [ ] **Step 3: 场景 2 — 选择维度进入 val phase**

1. 在 dim phase 选中 Goal（Enter 或点击）
2. 验证：菜单切换为浅蓝顶栏 + `←` 按钮 + "Pick a value for **Goal**"
3. 验证：列表项没有颜色条、没有序号
4. 验证：底部显示 "← Back to dimensions · Type to filter"

- [ ] **Step 4: 场景 3 — 自动串联 skipFilled**

1. 在 val phase 选中一个 Goal 值（如 "Sprint planning"）
2. 验证：自动回到 dim phase
3. 验证：选中的是第一个**未填**的维度（如 Business Line），而非已填的 Goal
4. 连续按 Enter 选值，验证每次回到 dim phase 都跳到下一个未填维度

- [ ] **Step 5: 场景 4 — 返回按钮**

1. 在 val phase，点击顶栏 `←` 按钮
2. 验证：回到 dim phase，`@` 前缀恢复正常
3. 在 val phase，清空 filter 后按 Backspace
4. 验证：同样回到 dim phase

- [ ] **Step 6: 场景 5 — 数字键不再快捷选择**

1. 在 dim phase 或 val phase
2. 按数字键 `1`、`2` 等
3. 验证：数字作为 filter 文字出现在输入框中，不会触发选择
4. 验证：菜单列表根据输入的数字 filter 筛选

- [ ] **Step 7: 场景 6 — 已填 chip、未填虚线 chip、提交**

1. 选完 4 个必填维度的值
2. 验证：输入框下方 chip 行显示 4 个已填 chip（与现有行为一致）
3. 输入时间（如 "1.5h"）+ 按 Enter 提交
4. 验证：提交成功，无错误

验证全部通过后继续。

- [ ] **Step 8: Commit（如有微调）**

```bash
git add src/components/EntryInput.vue && git commit -m "chore(mention-ux): manual verification tweaks"
```

如果无微调，跳过此步骤。

---

## 验证清单

| # | 场景 | 预期 |
|---|------|------|
| 1 | 按 `@` 打开菜单 | 深色顶栏 + "DIM" badge，颜色条，无序号，无红色 "required" |
| 2 | 选维度进入 val | 浅蓝顶栏 + ← 按钮 + 维度名，无颜色条，纯文字列表 |
| 3 | 选值后自动串联 | 回到 dim phase，选中第一个未填维度 |
| 4 | 点 ← / Backspace 回退 | 从 val 回到 dim |
| 5 | 数字键输入 | 数字进入 filter，不触发选择 |
| 6 | 底部圆点进度 | 绿色 = 已填，灰色 = 未填 |
| 7 | 提交 | chip 行、提交行为不变 |
