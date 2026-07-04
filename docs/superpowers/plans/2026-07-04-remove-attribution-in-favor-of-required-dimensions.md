# Remove Attribution — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Delete the attribution system (`Attribution` enum, `compute_attribution`, amber warnings, warning bar) and replace with `role required: true` + cross-dimension validation.

**Architecture:** The attribution mechanism post-classified entries at read time as `Unattributed`/`Mismatch`/`Ok` to drive amber UI. Replaced by two simpler mechanisms: (1) role dimension set to `required: true` blocks entry creation without a role; (2) new `validate_cross_dimension_constraints` function blocks CLI entries where goal doesn't belong to role. Both run at write time, making read-time classification unnecessary.

**Tech Stack:** Rust (Tauri commands), TypeScript (Vue 3 SFC), YAML config.

---

### Task 1: Add cross-dimension validation function + unit tests

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Write the validation function**

Insert after the `validate_required_dimensions` function (after line 137), before the `append_entry` function:

```rust
/// Validate cross-dimension constraints: if entry has both role and goal,
/// the goal must be declared under that role in commitments.yaml.
fn validate_cross_dimension_constraints(
    dimensions: &BTreeMap<String, String>,
    role_key: &str,
    goal_key: &str,
    role_to_goals: &std::collections::HashMap<String, Vec<String>>,
) -> Result<(), String> {
    let role = dimensions.get(role_key);
    let goal = dimensions.get(goal_key);
    if let (Some(r), Some(g)) = (role, goal) {
        if let Some(goals) = role_to_goals.get(r.as_str()) {
            if !goals.contains(g) {
                return Err(format!(
                    "Goal '{}' is not declared under role '{}'",
                    g, r
                ));
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Write unit tests**

Insert at the end of the test module (before the closing `}` of `#[cfg(test)] mod tests`):

```rust
    #[test]
    fn test_validate_cross_dimension_ok_matching() {
        let mut dims = BTreeMap::new();
        dims.insert("role".to_string(), "Eng".to_string());
        dims.insert("goal".to_string(), "ShipIt".to_string());
        let mut role_to_goals = std::collections::HashMap::new();
        role_to_goals.insert("Eng".to_string(), vec!["ShipIt".to_string()]);
        assert!(validate_cross_dimension_constraints(
            &dims, "role", "goal", &role_to_goals
        ).is_ok());
    }

    #[test]
    fn test_validate_cross_dimension_reject_mismatch() {
        let mut dims = BTreeMap::new();
        dims.insert("role".to_string(), "Eng".to_string());
        dims.insert("goal".to_string(), "OtherGoal".to_string());
        let mut role_to_goals = std::collections::HashMap::new();
        role_to_goals.insert("Eng".to_string(), vec!["ShipIt".to_string()]);
        let result = validate_cross_dimension_constraints(
            &dims, "role", "goal", &role_to_goals
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not declared under role"));
    }

    #[test]
    fn test_validate_cross_dimension_ok_no_goal() {
        let mut dims = BTreeMap::new();
        dims.insert("role".to_string(), "Eng".to_string());
        // no goal key present
        let role_to_goals = std::collections::HashMap::new();
        assert!(validate_cross_dimension_constraints(
            &dims, "role", "goal", &role_to_goals
        ).is_ok());
    }

    #[test]
    fn test_validate_cross_dimension_ok_role_not_in_map() {
        let mut dims = BTreeMap::new();
        dims.insert("role".to_string(), "UnknownRole".to_string());
        dims.insert("goal".to_string(), "SomeGoal".to_string());
        let role_to_goals = std::collections::HashMap::new(); // empty
        assert!(validate_cross_dimension_constraints(
            &dims, "role", "goal", &role_to_goals
        ).is_ok());
    }
```

- [ ] **Step 3: Run the new tests**

```bash
cd src-tauri && cargo test validate_cross_dimension
```
Expected: 4 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: add validate_cross_dimension_constraints with tests"
```

---

### Task 2: Integrate cross-dimension validation into append_entry and update_entry

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: In `append_entry`, add cross-dimension validation after required validation**

In `append_entry` (around line 551), after `validate_required_dimensions(&dims, &entry.dimensions)?;`, insert:

```rust
    // Cross-dimension validation
    {
        let commitments = crate::files::read_commitments_file(root, year, month).unwrap_or_default();
        let goal_key = goal_dim_key(root, year, month)?;
        let role_key = role_dim_key(root, year, month)?;
        let (_, role_to_goals) = build_commitment_maps(&commitments);
        validate_cross_dimension_constraints(&entry.dimensions, &role_key, &goal_key, &role_to_goals)?;
    }
```

- [ ] **Step 2: In `update_entry`, add cross-dimension validation when dimensions are updated**

In `update_entry` (around line 608-610), inside the `if let Some(ref dims) = update.dimensions` block, after `validate_required_dimensions(&effective, dims)?;`, insert:

```rust
            {
                let commitments = crate::files::read_commitments_file(root, year, month).unwrap_or_default();
                let goal_key = goal_dim_key(root, year, month)?;
                let role_key = role_dim_key(root, year, month)?;
                let (_, role_to_goals) = build_commitment_maps(&commitments);
                validate_cross_dimension_constraints(dims, &role_key, &goal_key, &role_to_goals)?;
            }
```

- [ ] **Step 3: Run tests to verify compilation**

```bash
cd src-tauri && cargo test
```
Expected: all existing tests pass (new validation only fires when both role + goal present, doesn't break existing tests).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: integrate cross-dimension validation into append_entry and update_entry"
```

---

### Task 3: Remove Attribution from models.rs

**Files:**
- Modify: `src-tauri/src/models.rs`

- [ ] **Step 1: Delete the Attribution enum and its Default impl**

Delete lines 53-69 (the entire `Attribution` enum block including `impl Default`).

- [ ] **Step 2: Remove `attribution` field from `Entry` struct**

Change the `Entry` struct from:
```rust
pub struct Entry {
    pub id: String,
    pub item: String,
    pub duration: u32,
    #[serde(default)]
    pub dimensions: BTreeMap<String, String>,
    #[serde(default)]
    pub attribution: Attribution,
}
```
to:
```rust
pub struct Entry {
    pub id: String,
    pub item: String,
    pub duration: u32,
    #[serde(default)]
    pub dimensions: BTreeMap<String, String>,
}
```

- [ ] **Step 3: Simplify `CommitmentProgressResult` to remove warning fields**

Change from:
```rust
pub struct CommitmentProgressResult {
    pub roles: Vec<CommitmentProgress>,
    pub unattributed_count: u32,
    pub unattributed_total_minutes: u32,
    pub mismatch_count: u32,
}
```
to — delete the struct entirely; `get_commitment_progress` will return `Vec<CommitmentProgress>` directly (handled in Task 5).

- [ ] **Step 4: Add `CommitmentProgressResult` type alias for smooth transition**

In models.rs, add after the `CommitmentProgress` struct:
```rust
/// Return type for get_commitment_progress (simplified: was a wrapper, now just the role list).
pub type CommitmentProgressResult = Vec<CommitmentProgress>;
```
This keeps the IPC type name stable, avoiding a breaking change in the frontend's `invoke` type parameter. The frontend will still `invoke<CommitmentProgressResult>` but it resolves to `CommitmentProgress[]`.

Wait — Tauri serialization won't work with a type alias as a command return type. The `#[tauri::command]` return type needs to be a concrete type, not an alias to `Vec<T>`. Let me reconsider.

Actually, rename it: delete the old struct, and define:
```rust
/// get_commitment_progress 返回值
pub type CommitmentProgressResult = Vec<CommitmentProgress>;
```

Tauri commands can return `Vec<T>` directly. The frontend type `CommitmentProgressResult` in TypeScript will change from `{ roles: ..., unattributed_count: ..., ... }` to `CommitmentProgress[]`. We update all references.

Let me just delete the struct and in Task 5 change `get_commitment_progress` return type to `Vec<CommitmentProgress>`. The frontend will be updated accordingly in the frontend tasks.

**Step 4 (revised):** Simply delete the `CommitmentProgressResult` struct (lines 72-78). No alias needed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models.rs
git commit -m "refactor: remove Attribution enum, Entry.attribution field, and CommitmentProgressResult struct"
```

---

### Task 4: Remove attribution injection from commands.rs

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Delete `compute_attribution` function**

Delete lines 824-857 (the entire function).

- [ ] **Step 2: Delete `annotate_day_file` function**

Delete lines 879-889 (the entire function).

- [ ] **Step 3: Remove attribution injection from `append_entry`**

Delete lines 578-585 (the `// Inject attribution for the new entry` block). Also remove the `let mut entry =` and change to `let entry =` since it's no longer mutable:

Change:
```rust
    let mut entry = Entry {
        id: entry_id,
        item: entry.item,
        duration,
        dimensions: entry.dimensions,
        attribution: crate::models::Attribution::default(),
    };

    // Inject attribution for the new entry
    {
        let commitments = crate::files::read_commitments_file(root, year, month).unwrap_or_default();
        let goal_key = goal_dim_key(root, year, month)?;
        let role_key = role_dim_key(root, year, month)?;
        let (goal_to_role, role_to_goals) = build_commitment_maps(&commitments);
        entry.attribution = compute_attribution(&entry.dimensions, &role_key, &goal_key, &goal_to_role, &role_to_goals);
    }
```
to:
```rust
    let entry = Entry {
        id: entry_id,
        item: entry.item,
        duration,
        dimensions: entry.dimensions,
    };
```

- [ ] **Step 4: Remove attribution injection from `update_entry`**

Replace the attribution injection block (lines 640-656) — delete the `// Inject attribution` comment and its entire block:

Delete:
```rust
    // Inject attribution
    if let Ok(ref mut day_file) = result {
        let commitments = crate::files::read_commitments_file(root, year, month).unwrap_or_default();
        let goal_key = goal_dim_key(root, year, month)?;
        let role_key = role_dim_key(root, year, month)?;
        let (goal_to_role, role_to_goals) = build_commitment_maps(&commitments);
        annotate_day_file(day_file, &role_key, &goal_key, &goal_to_role, &role_to_goals);
    }
```

- [ ] **Step 5: Remove attribution injection from `delete_entry`**

In `delete_entry` (around lines 690-703), delete the same attribution injection block (identical structure to update_entry's).

- [ ] **Step 6: Remove attribution injection from `get_entries`**

Delete lines 518-537 (the `// Inject attribution` block in `get_entries`).

- [ ] **Step 7: Remove attribution injection from `get_month_entries`**

In `get_month_entries`:
- Delete lines 460-463 (reading commitments + building maps).
- Delete lines 491-493 (the `annotate_day_file` call inside the loop).
- Change the doc comment on line 443-444 from `/// Batch-read all day files for a month, injecting attribution from\n/// commitments.yaml (read once). Returns entries keyed by YYYY-MM-DD date.` to `/// Batch-read all day files for a month. Returns entries keyed by YYYY-MM-DD date.`

- [ ] **Step 8: Remove attribution injection from `load_root_state`**

Delete lines 264-299 (the entire `// Inject attribution into today's entries` block, including all lines between `{` and `}`).

- [ ] **Step 9: Remove `goal_dim_key` and `role_dim_key` from `compute_attribution` unit tests**

Delete lines 2543-2635 (all `test_compute_attribution_*` test functions).

- [ ] **Step 10: Build check**

```bash
cd src-tauri && cargo check
```
Expected: compiles without errors. Fix any remaining references to `attribution` field or `Attribution` enum.

- [ ] **Step 11: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "refactor: remove all attribution injection from commands.rs"
```

---

### Task 5: Simplify get_commitment_progress

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Change return type and remove unattributed/mismatch tracking**

Change the function signature from:
```rust
) -> Result<crate::models::CommitmentProgressResult, String> {
```
to:
```rust
) -> Result<Vec<crate::models::CommitmentProgress>, String> {
```

- [ ] **Step 2: Remove unattributed/mismatch counters**

Delete lines 918-920:
```rust
    let mut unattributed_count: u32 = 0;
    let mut unattributed_total: u32 = 0;
    let mut mismatch_count: u32 = 0;
```

- [ ] **Step 3: Simplify the entry aggregation loop**

Replace lines 973-1017 (the `match attr { ... }` block) with:

```rust
                        for e in &day_file.entries {
                            if let Some(role) = e.dimensions.get(&role_key) {
                                if let Some(goal_val) = e.dimensions.get(&goal_key) {
                                    *role_goal_spent.entry(role.clone()).or_insert(0) += e.duration;
                                    *goal_spent.entry(goal_val.clone()).or_insert(0) += e.duration;
                                } else {
                                    *role_general_spent.entry(role.clone()).or_insert(0) += e.duration;
                                }
                            }
                        }
```

- [ ] **Step 4: Update early-return values**

In the `goal_key` missing branch (lines 935-941), change:
```rust
            return Ok(crate::models::CommitmentProgressResult {
                roles: vec![],
                unattributed_count: 0,
                unattributed_total_minutes: 0,
                mismatch_count: 0,
            });
```
to:
```rust
            return Ok(vec![]);
```

In the `role_key` missing branch (lines 947-953), same change.

- [ ] **Step 5: Update the final Ok return**

Change lines 1048-1053:
```rust
    Ok(crate::models::CommitmentProgressResult {
        roles,
        unattributed_count,
        unattributed_total_minutes: unattributed_total,
        mismatch_count,
    })
```
to:
```rust
    Ok(roles)
```

- [ ] **Step 6: Run tests**

```bash
cd src-tauri && cargo test
```
Expected: tests pass. Any test that constructed `CommitmentProgressResult` with the old fields will need updating.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "refactor: simplify get_commitment_progress to return Vec<CommitmentProgress>"
```

---

### Task 6: Remove attribution from files.rs and operation_log.rs

**Files:**
- Modify: `src-tauri/src/files.rs`
- Modify: `src-tauri/src/operation_log.rs`
- Modify: `src-tauri/tests/operation_log_integration.rs`

- [ ] **Step 1: Remove attribution from `append_new_entry` in files.rs**

Change line 170 from:
```rust
        attribution: crate::models::Attribution::default(),
```
to — remove the line entirely (remove the trailing comma from the previous line too).

- [ ] **Step 2: Remove attribution from all test Entry constructions in files.rs**

Remove `attribution: crate::models::Attribution::default(),` from lines 511, 671, 692, 699, 726, 733.

- [ ] **Step 3: Remove attribution from operation_log.rs**

Line 227 — remove `attribution: crate::models::Attribution::default(),`

Line 472 — remove `attribution: crate::models::Attribution::default(),`

- [ ] **Step 4: Remove attribution from operation_log_integration.rs**

Line 34 — remove `attribution: tauri_app_lib::models::Attribution::default(),`

- [ ] **Step 5: Build check and run tests**

```bash
cd src-tauri && cargo test
```
Expected: compiles and all tests pass (any test with old `Attribution::default()` references will fail compilation — fix them).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/files.rs src-tauri/src/operation_log.rs src-tauri/tests/operation_log_integration.rs
git commit -m "refactor: remove Attribution::default() from files.rs, operation_log.rs, and integration tests"
```

---

### Task 7: Update configuration files

**Files:**
- Modify: `src-tauri/tests/fixtures/dimensions.template.yaml`

- [ ] **Step 1: Add required: true to Role dimension in test fixture**

In `src-tauri/tests/fixtures/dimensions.template.yaml`, change the Role dimension from:
```yaml
  - name: Role
    key: role
    source: commitments:role
```
to:
```yaml
  - name: Role
    key: role
    source: commitments:role
    required: true
```

- [ ] **Step 2: Run integration tests**

```bash
cd src-tauri && cargo test
```
Expected: tests that use this fixture pass. Any integration test that creates entries without a role dimension will now fail with "Missing required dimension: Role". Those tests need to add `"role"` to the entry's dimensions — fix them inline.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/fixtures/dimensions.template.yaml
git commit -m "config: set Role required: true in test fixture"
```

---

### Task 8: Run full Rust test suite, verify backend compiles

- [ ] **Step 1: Run all Rust tests**

```bash
cd src-tauri && cargo test
```
Expected: all tests pass. If any fail, fix and re-run.

- [ ] **Step 2: Commit any fixes**

---

### Task 9: Remove attribution from frontend types

**Files:**
- Modify: `src/types.ts`

- [ ] **Step 1: Delete the `Attribution` type**

Delete line 37:
```typescript
export type Attribution = "ok" | "unattributed" | "mismatch";
```

- [ ] **Step 2: Remove `attribution` field from `Entry` interface**

Change the `Entry` interface from:
```typescript
export interface Entry {
  id: string;
  item: string;
  duration: number;
  dimensions: Record<string, string>;
  attribution: Attribution;
}
```
to:
```typescript
export interface Entry {
  id: string;
  item: string;
  duration: number;
  dimensions: Record<string, string>;
}
```

- [ ] **Step 3: Simplify `CommitmentProgressResult`**

Delete the old interface:
```typescript
export interface CommitmentProgressResult {
  roles: CommitmentProgress[];
  unattributed_count: number;
  unattributed_total_minutes: number;
  mismatch_count: number;
}
```
Replace with a type alias:
```typescript
export type CommitmentProgressResult = CommitmentProgress[];
```

This keeps the name `CommitmentProgressResult` stable for the `invoke<>()` calls in `useMonthData.ts` and `useEntryActions.ts` — the resolved type is now `CommitmentProgress[]` directly.

- [ ] **Step 4: Commit**

```bash
git add src/types.ts
git commit -m "refactor: remove Attribution type and simplify CommitmentProgressResult in frontend types"
```

---

### Task 10: Remove amber from EntryRow.vue

**Files:**
- Modify: `src/components/composite/EntryRow.vue`

- [ ] **Step 1: Delete `isProblemEntry` computed**

Delete lines 35-37:
```typescript
const isProblemEntry = computed(() =>
  props.entry.attribution === "unattributed" || props.entry.attribution === "mismatch"
);
```

- [ ] **Step 2: Remove amber styling from the template**

The amber styling appears in three places in the `<div>` element and `<span>` elements. Remove all `isProblemEntry` references:

Change the outer `<div>` class binding from:
```html
  <div
    v-else
    data-test="entry-row"
    class="group flex justify-between items-start gap-sm px-md py-sm transition-colors"
    :class="[
      { 'just-added': justAdded },
      isProblemEntry
        ? 'bg-[var(--color-problem-entry-bg)] hover:bg-[var(--color-problem-entry-hover-bg)]'
        : 'hover:bg-[var(--color-surface-muted)]',
      index > 0 ? 'border-t border-[var(--color-divider)]' : '',
      isProblemEntry && index > 0 ? '!border-[var(--color-problem-entry-border)]' : '',
    ]"
```
to:
```html
  <div
    v-else
    data-test="entry-row"
    class="group flex justify-between items-start gap-sm px-md py-sm transition-colors"
    :class="[
      { 'just-added': justAdded },
      'hover:bg-[var(--color-surface-muted)]',
      index > 0 ? 'border-t border-[var(--color-divider)]' : '',
    ]"
```

- [ ] **Step 3: Remove the amber dot**

Delete the entire amber dot `<span>` (the `v-if="isProblemEntry"` span):
```html
    <span
      v-if="isProblemEntry"
      ...
    >●</span>
```

- [ ] **Step 4: Remove amber color from duration display**

Change:
```html
      :class="isProblemEntry ? '!text-[var(--color-problem-entry-text)] font-medium' : 'text-[var(--color-text-primary)]'"
```
to:
```html
      class="text-[var(--color-text-primary)]"
```

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/EntryRow.vue
git commit -m "refactor: remove amber problem-entry styling from EntryRow"
```

---

### Task 11: Remove warning bar from CommitmentsPanel.vue

**Files:**
- Modify: `src/components/CommitmentsPanel.vue`

- [ ] **Step 1: Remove `CommitmentProgressResult` import and `progressResult` prop**

In the `<script setup>`:
- Remove `CommitmentProgressResult` from the import line (line 3 — change to `import type { Commitment, CommitmentProgress } from "../types";`)
- Remove the `progressResult` prop (lines 13-14)
- Delete the `warningVisible`, `warningUnattributedMinutes`, `warningMismatchCount` computed properties (lines 17-28)

- [ ] **Step 2: Remove the warning bar template**

Delete lines 121-133 (the `<div v-if="warningVisible" data-test="warning-bar">...</div>` block).

- [ ] **Step 3: Commit**

```bash
git add src/components/CommitmentsPanel.vue
git commit -m "refactor: remove warning bar from CommitmentsPanel"
```

---

### Task 12: Clean up CSS tokens

**Files:**
- Modify: `src/assets/tokens.css`

- [ ] **Step 1: Remove problem-entry tokens**

In the light theme section, delete lines 36-40 (the `/* === Problem Entry ... === */` block):
```css
/* === Problem Entry (unattributed / mismatch) === */
--color-problem-entry-bg: #fffbeb;
--color-problem-entry-hover-bg: #fef3c7;
--color-problem-entry-border: #fde68a;
--color-problem-entry-text: #d97706;
```

- [ ] **Step 2: Remove warning-bar tokens from light theme**

Delete lines 42-44:
```css
/* === Warning Bar (CommitmentsPanel unattributed/mismatch) === */
--color-warning-bar-text: #92400e;
--color-warning-bar-hint: #b45309;
```

- [ ] **Step 3: Remove problem-entry and warning-bar tokens from dark theme**

Remove the corresponding dark theme tokens (lines 152-157):
```css
--color-problem-entry-bg: #422006;
--color-problem-entry-hover-bg: #5c2d0a;
--color-problem-entry-border: #78350f;
--color-problem-entry-text: #f59e0b;
--color-warning-bar-text: #facc15;
--color-warning-bar-hint: #fde68a;
```

- [ ] **Step 4: Commit**

```bash
git add src/assets/tokens.css
git commit -m "refactor: remove problem-entry and warning-bar CSS tokens"
```

---

### Task 13: Update store, composables, and MonthView to match new types

**Files:**
- Modify: `src/stores/useStore.ts`
- Modify: `src/composables/useMonthData.ts`
- Modify: `src/composables/useEntryActions.ts`
- Modify: `src/components/MonthView.vue`

- [ ] **Step 1: Remove `commitmentProgressResult` from useStore.ts**

- Delete `CommitmentProgressResult` from the import (line 2).
- Delete `commitmentProgressResult: CommitmentProgressResult | null;` (line 21).
- Delete `commitmentProgressResult: null,` (line 44).

- [ ] **Step 2: Update useMonthData.ts**

- Change `import type { Entry, DayFile, Commitment, CommitmentProgressResult, MonthDimensions }` to `import type { Entry, DayFile, Commitment, CommitmentProgressResult, MonthDimensions }` (unchanged — still imports `CommitmentProgressResult` but it's now `CommitmentProgress[]`).
- Delete line 12: `store.commitmentProgressResult = null;`
- Change lines 44-46 from:
```typescript
      const result = await invoke<CommitmentProgressResult>("get_commitment_progress", { rootPath: store.rootPath, year, month });
      store.commitmentProgress = result.roles;
      store.commitmentProgressResult = result;
```
to:
```typescript
      store.commitmentProgress = await invoke<CommitmentProgressResult>("get_commitment_progress", { rootPath: store.rootPath, year, month });
```
- Delete line 50: `store.commitmentProgressResult = null;`

- [ ] **Step 3: Update useEntryActions.ts**

- Change lines 32, 37-42 in `refreshProgress` from:
```typescript
      const result = await invoke<CommitmentProgressResult>("get_commitment_progress", {
        rootPath: store.rootPath,
        year: ym.year,
        month: ym.month,
      });
      store.commitmentProgress = result.roles;
      store.commitmentProgressResult = result;
```
to:
```typescript
      store.commitmentProgress = await invoke<CommitmentProgressResult>("get_commitment_progress", {
        rootPath: store.rootPath,
        year: ym.year,
        month: ym.month,
      });
```
- Delete line 42: `store.commitmentProgressResult = null;`

- [ ] **Step 4: Update MonthView.vue**

Remove the `:progress-result` binding from the CommitmentsPanel usage (line 147):
```html
      <CommitmentsPanel
        :progress="store.commitmentProgress"
        :commitments="store.commitments"
        :root-path="store.rootPath"
        :selected-year="selectedYear"
        :selected-month="selectedMonth"
        @saved="onCommitmentsSaved"
      />
```

- [ ] **Step 5: Type check**

```bash
pnpm vue-tsc --noEmit
```
Expected: no type errors.

- [ ] **Step 6: Commit**

```bash
git add src/stores/useStore.ts src/composables/useMonthData.ts src/composables/useEntryActions.ts src/components/MonthView.vue
git commit -m "refactor: remove commitmentProgressResult from store and wire simplified progress data"
```

---

### Task 14: Update frontend tests and fixtures

**Files:**
- Modify: `src/__tests__/mocks/fixtures.ts`
- Modify: `src/__tests__/components/EntryComposer.test.ts`
- Modify: `src/__tests__/components/composite/EntryRowEdit.test.ts`
- Modify: `src/__tests__/components/DimensionPopover.test.ts`
- Modify: `src/__tests__/tailwind-token-usage.test.ts` (if it references the removed tokens)

- [ ] **Step 1: Remove `attribution` from `makeEntry` in fixtures.ts**

Change the `makeEntry` function from:
```typescript
export function makeEntry(overrides?: Partial<Entry>): Entry {
  return {
    id: fakeUuid(),
    item: "Test entry",
    duration: 30,
    dimensions: {},
    attribution: "ok",
    ...overrides,
  };
}
```
to:
```typescript
export function makeEntry(overrides?: Partial<Entry>): Entry {
  return {
    id: fakeUuid(),
    item: "Test entry",
    duration: 30,
    dimensions: {},
    ...overrides,
  };
}
```

- [ ] **Step 2: Update EntryComposer.test.ts**

Remove any test cases that reference `attribution` or `isProblemEntry`. Search for and delete test cases referencing these concepts. If `makeEntry` is used with `attribution` override, remove the override.

- [ ] **Step 3: Update EntryRowEdit.test.ts**

Same — remove any tests referencing `attribution` or amber/problem-entry styling.

- [ ] **Step 4: Update DimensionPopover.test.ts**

Remove any tests that check the attribution-related behavior (e.g., tests checking that required-or-not dimensions set attribution correctly). The required/optional behavior in DimensionPopover is unchanged — only remove attribution-specific tests.

- [ ] **Step 5: Check tailwind-token-usage.test.ts**

```bash
cd src && npx vitest run src/__tests__/tailwind-token-usage.test.ts
```
If it fails because removed tokens were referenced in an allowlist, update the test's allowlist accordingly.

- [ ] **Step 6: Run all frontend tests**

```bash
pnpm test
```
Expected: all tests pass. Fix any failures.

- [ ] **Step 7: Commit**

```bash
git add src/__tests__/
git commit -m "test: remove attribution references from frontend tests and fixtures"
```

---

### Task 15: Final verification

- [ ] **Step 1: Run full project test suite**

```bash
pnpm test
```
Expected: all backend and frontend tests pass.

- [ ] **Step 2: Type check**

```bash
pnpm vue-tsc --noEmit && cd src-tauri && cargo check
```
Expected: no type errors.

- [ ] **Step 3: Fix any remaining issues and commit**

---

### Potential issues to watch for

1. **Integration tests that construct entries without `role` dimension**: With `required: true` on Role, integration tests that use the fixture and create entries without a role dimension will fail `validate_required_dimensions`. Those tests need `dimensions: { "role": "Dev" }` added to their entry input.

2. **`unused import` warnings for `BTreeMap` or `HashMap` in commands.rs**: After removing `compute_attribution` and other attribution code, some imports may become unused. Run `cargo check` and remove unused imports.

3. **Tailwind token test**: The test at `src/__tests__/tailwind-token-usage.test.ts` validates that only allowed design tokens are used. Removing `--color-problem-entry-*` and `--color-warning-bar-*` tokens from `tokens.css` may cause this test to fail if they're in an allowlist. Remove them from the allowlist if needed.
