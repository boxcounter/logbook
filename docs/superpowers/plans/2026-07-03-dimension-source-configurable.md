# Dimension Source Configurable + Delete _monthly.md Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make dimension source values configurable via template, add Role as a configurable dimension, and remove the legacy _monthly.md file.

**Architecture:** Mirror the existing `goal_dim_key()` pattern (dynamic key resolution from template dimensions by `source` field). Add `role_dim_key()` using the same pattern. Replace `_monthly.md` reads with `dimensions.yaml`-based `usingDefaultDimensions` detection. Rename `source: "monthly"` → `"commitments:goals"` and `from_template` → `usingDefaultDimensions`.

**Tech Stack:** Rust (Tauri 2.x), TypeScript, Vue 3, vitest, cargo test

**Spec:** `docs/superpowers/specs/2026-07-03-dimension-source-configurable-design.md`

---

## File Structure

| File | Responsibility | Action |
|------|---------------|--------|
| `src-tauri/src/models.rs` | Dimension, MonthDimensions, InitResult types | Modify (rename fields, delete MonthlyFile) |
| `src-tauri/src/config.rs` | Validation + watcher | Modify (validate_dimensions, delete validate_monthly, rename month_from_monthly_path) |
| `src-tauri/src/commands.rs` | Business logic + dimension key resolution | Modify (add role_dim_key, rename monthly_dim_key, update callers) |
| `src-tauri/src/files.rs` | File I/O | Modify (delete _monthly.md functions) |
| `src-tauri/src/scan.rs` | Data dir scanning | Modify (remove _monthly.md skip) |
| `src-tauri/src/operation_log.rs` | Operation log | Modify (remove _monthly.md skip) |
| `src-tauri/tests/fixtures/template.yaml` | Test fixture | Modify (source rename + add Role dim) |
| `src-tauri/tests/fixtures/2026/06/_monthly.md` | Test fixture | Delete |
| `src-tauri/tests/*.rs` | Integration tests | Modify (inline YAML source rename) |
| `src/types.ts` | Frontend types | Modify (source union, usingDefaultDimensions) |
| `src/stores/useStore.ts` | Reactive store | Modify (fromTemplate → usingDefaultDimensions) |
| `src/components/DimensionPopover.vue` | Dimension selection | Modify (roleKey, goalKey) |
| `src/components/composite/DimensionEditorModal.vue` | Dimension editing modal | Modify (source check) |
| `src/components/MonthView.vue` | Month view | Modify (fromTemplate → usingDefaultDimensions) |
| `src/App.vue` | App root | Modify (fromTemplate → usingDefaultDimensions) |
| `src/utils/applyInitResult.ts` | Init result handler | Modify (fromTemplate → usingDefaultDimensions) |
| `src/__tests__/**/*.test.ts` | Frontend tests | Modify (source + fromTemplate updates) |

---

### Task 1: Rename source "monthly" → "commitments:goals" in Rust

**Files:**
- Modify: `src-tauri/src/models.rs:151` (InitResult::Ready.from_template rename comes in Task 6 — skip for now, just source renames)

Wait — let me restructure. The renames need to be done in a specific order to avoid dead references. Let me trace dependencies:

1. First, rename source strings everywhere (backend + test fixtures + integration tests) — pure string replacements, no logic changes
2. Then add role_dim_key and update callers — logic changes  
3. Then delete _monthly.md — logic changes
4. Then rename from_template → usingDefaultDimensions — pure rename
5. Then frontend

Actually, from_template → usingDefaultDimensions can be done independently alongside the source renames. Let me reorder.

Let me rewrite the plan more carefully.

---

### Task 1: Rename source "monthly" → "commitments:goals" in config.rs

**Files:**
- Modify: `src-tauri/src/config.rs:55-122`

**Description:** Update `validate_dimensions` to use new source names and add role source validation.

- [ ] **Step 1: Update validate_dimensions match arms**

In `src-tauri/src/config.rs:55-122`, replace the validate_dimensions function:

```rust
pub fn validate_dimensions(dimensions: &[Dimension]) -> Vec<ConfigErrorDetail> {
    let mut errors = Vec::new();
    let mut goal_source_count = 0;
    let mut role_source_count = 0;

    for (i, dim) in dimensions.iter().enumerate() {
        if dim.name.is_empty() {
            errors.push(ConfigErrorDetail {
                kind: "MissingName".to_string(),
                message: format!("Dimension at index {}: name is required", i),
            });
        }
        if dim.key.is_empty() {
            errors.push(ConfigErrorDetail {
                kind: "MissingKey".to_string(),
                message: format!("Dimension at index {}: key is required", i),
            });
        } else if !is_valid_key(&dim.key) {
            errors.push(ConfigErrorDetail {
                kind: "KeyInvalidChars".to_string(),
                message: format!(
                    "Dimension '{}': key '{}' contains invalid characters (use a-z, 0-9, -, _)",
                    dim.name, dim.key
                ),
            });
        }
        match dim.source.as_str() {
            "static" => match &dim.values {
                None => errors.push(ConfigErrorDetail {
                    kind: "MissingValues".to_string(),
                    message: format!(
                        "Dimension '{}' (key: {}): source is 'static' but values is not set",
                        dim.name, dim.key
                    ),
                }),
                Some(vals) if vals.is_empty() => errors.push(ConfigErrorDetail {
                    kind: "ValuesEmpty".to_string(),
                    message: format!(
                        "Dimension '{}' (key: {}): values list is empty",
                        dim.name, dim.key
                    ),
                }),
                _ => {}
            },
            "commitments:goals" => {
                goal_source_count += 1;
                if goal_source_count > 1 {
                    errors.push(ConfigErrorDetail {
                        kind: "MultipleGoalSource".to_string(),
                        message: format!(
                            "Dimension '{}': only one dimension may have source: commitments:goals",
                            dim.name
                        ),
                    });
                }
            }
            "commitments:role" => {
                role_source_count += 1;
                if role_source_count > 1 {
                    errors.push(ConfigErrorDetail {
                        kind: "MultipleRoleSource".to_string(),
                        message: format!(
                            "Dimension '{}': only one dimension may have source: commitments:role",
                            dim.name
                        ),
                    });
                }
            }
            other => {
                errors.push(ConfigErrorDetail {
                    kind: "InvalidSource".to_string(),
                    message: format!(
                        "Dimension '{}': invalid source '{}' (expected 'static', 'commitments:goals', or 'commitments:role')",
                        dim.name, other
                    ),
                });
            }
        }
    }
    errors
}
```

- [ ] **Step 2: Run Rust tests to verify config.rs changes compile**

Run: `cargo test -p tauri_app_lib -- config::tests`
Workdir: `src-tauri`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/config.rs
git commit -m "refactor: rename source monthly → commitments:goals, add commitments:role validation"
```

---

### Task 2: Rename monthly_dim_key → goal_dim_key, add role_dim_key

**Files:**
- Modify: `src-tauri/src/commands.rs:642-657`

**Description:** Rename the function and change source lookup. Add role_dim_key.

- [ ] **Step 1: Rename and update the functions**

In `src-tauri/src/commands.rs`, replace lines 642-657:

```rust
/// The dimension key used to tag a commitment goal for this month. Finds the
/// dimension with source=="commitments:goals", falling back to "goal" when none found.
fn goal_dim_key(root: &std::path::Path, year: i32, month: u32) -> String {
    files::resolve_month_dimensions(root, year, month)
        .ok()
        .and_then(|dims| dims.into_iter().find(|d| d.source == "commitments:goals").map(|d| d.key))
        .unwrap_or_else(|| "goal".to_string())
}

fn role_dim_key(root: &std::path::Path, year: i32, month: u32) -> String {
    files::resolve_month_dimensions(root, year, month)
        .ok()
        .and_then(|dims| dims.into_iter().find(|d| d.source == "commitments:role").map(|d| d.key))
        .unwrap_or_else(|| "role".to_string())
}
```

- [ ] **Step 2: Update all references from monthly_dim_key to goal_dim_key**

Do a global find-and-replace in `src-tauri/src/commands.rs`: `monthly_dim_key` → `goal_dim_key`

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Workdir: `src-tauri`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "refactor: rename monthly_dim_key → goal_dim_key, add role_dim_key"
```

---

### Task 3: Update test data source: monthly → "commitments:goals" in Rust

**Files:**
- Modify: `src-tauri/src/commands.rs:1731` (make_config)
- Modify: `src-tauri/src/config.rs:442,510,518` (test Dimension constructors)
- Modify: `src-tauri/tests/fixtures/template.yaml`
- Modify: `src-tauri/tests/fixtures/2026/06/_monthly.md`

**Description:** Update all hardcoded `source: "monthly"` in test data. Add Role dimension to template fixture.

- [ ] **Step 1: Update make_config in commands.rs**

In `src-tauri/src/commands.rs:1731`, change:
```rust
source: "monthly".into(),
```
to:
```rust
source: "commitments:goals".into(),
```

- [ ] **Step 2: Update config.rs test data**

In `src-tauri/src/config.rs`, find all `source: "monthly".into()` (lines ~442, 510, 518) and change to `source: "commitments:goals".into()`. Also update `"monthly"` in test assertions for error messages.

- [ ] **Step 3: Update tests/fixtures/template.yaml**

Change `source: monthly` to `source: commitments:goals` and add Role dimension:

```yaml
dimensions:
  - name: Goal
    key: goal
    source: commitments:goals
  - name: Role
    key: role
    source: commitments:role
  - name: Biz
    key: biz
    source: static
    values:
      - Product
      - Marketing
      - Engineering
    required: false
```

- [ ] **Step 4: Update tests/fixtures/2026/06/_monthly.md**

Change `source: monthly` to `source: commitments:goals`.

- [ ] **Step 5: Run cargo test to verify**

Run: `cargo test`
Workdir: `src-tauri`

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/config.rs src-tauri/tests/fixtures/template.yaml src-tauri/tests/fixtures/2026/06/_monthly.md
git commit -m "test: update source monthly → commitments:goals in test data and fixtures"
```

---

### Task 4: Update all integration test inline YAML source strings

**Files:**
- Modify: `src-tauri/tests/cli_integration.rs:24`
- Modify: `src-tauri/tests/commitment_editor_integration.rs:14`
- Modify: `src-tauri/tests/scan_integration.rs:15`
- Modify: `src-tauri/tests/entry_crud_integration.rs:18,165,199,235`
- Modify: `src-tauri/tests/op_log_verify_integration.rs:21,87`
- Modify: `src-tauri/tests/commitment_progress_integration.rs:15,154`
- Modify: `src-tauri/tests/monthly_dimensions_integration.rs:19`
- Modify: `src-tauri/tests/recovery_category_integration.rs:8`
- Modify: `src-tauri/tests/dimension_editor_integration.rs:46,77,122,157,189,224`

**Description:** Global search `source: monthly` in YAML strings within test files and replace with `source: commitments:goals`.

- [ ] **Step 1: Replace all occurrences**

Run find-and-replace across all `src-tauri/tests/*.rs` files:
- `source: monthly` → `source: commitments:goals`

- [ ] **Step 2: Verify compilation**

Run: `cargo test --no-run`
Workdir: `src-tauri`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/
git commit -m "test: rename source monthly → commitments:goals in integration tests"
```

---

### Task 5: Update frontend types and DimensionPopover goalKey

**Files:**
- Modify: `src/types.ts:4`
- Modify: `src/components/DimensionPopover.vue:52-54,88`
- Modify: `src/components/composite/DimensionEditorModal.vue:130-131,441`

**Description:** Update TypeScript source union and Vue component references.

- [ ] **Step 1: Update types.ts**

In `src/types.ts:4`, change:
```typescript
source: "static" | "monthly";
```
to:
```typescript
source: "static" | "commitments:goals" | "commitments:role";
```

- [ ] **Step 2: Update DimensionPopover.vue goalKey**

In `src/components/DimensionPopover.vue:52-54`, change:
```typescript
const goalKey = computed(() => {
  const monthly = props.dimensions.find(d => d.source === "monthly");
  return monthly?.key ?? "goal";
});
```
to:
```typescript
const goalKey = computed(() => {
  const monthly = props.dimensions.find(d => d.source === "commitments:goals");
  return monthly?.key ?? "goal";
});
```

In `src/components/DimensionPopover.vue:88`, change:
```typescript
if (d.source === "monthly") return goalOptions.value;
```
to:
```typescript
if (d.source === "commitments:goals") return goalOptions.value;
```

- [ ] **Step 3: Update DimensionEditorModal.vue**

In `src/components/composite/DimensionEditorModal.vue:130`, change:
```typescript
if (newDimSource.value === "monthly" && draft.value.some(d => d.source === "monthly" && !d.deleted)) {
  return "Only one monthly-source dimension allowed";
}
```
to:
```typescript
if (newDimSource.value === "commitments:goals" && draft.value.some(d => d.source === "commitments:goals" && !d.deleted)) {
  return "Only one commitments:goals source dimension allowed";
}
```

In `src/components/composite/DimensionEditorModal.vue:441`, change:
```html
<template v-if="selectedDimension.source === 'monthly'">
```
to:
```html
<template v-if="selectedDimension.source === 'commitments:goals'">
```

- [ ] **Step 4: Update frontend test data**

In all `src/__tests__/**/*.test.ts` files, replace:
- `source: "monthly"` → `source: "commitments:goals"`
- `'monthly'` → `'commitments:goals'` (in string assertions)

- [ ] **Step 5: Run frontend tests**

Run: `pnpm vitest run`
Expected: All tests pass (some may fail until Task 6 is complete)

- [ ] **Step 6: Commit**

```bash
git add src/types.ts src/components/DimensionPopover.vue src/components/composite/DimensionEditorModal.vue src/__tests__/
git commit -m "refactor: rename source monthly → commitments:goals in frontend"
```

---

### Task 6: Add roleKey to DimensionPopover and remove pseudo-dimension injection

**Files:**
- Modify: `src/components/DimensionPopover.vue`

**Description:** Add `roleKey` computed property. Replace all hardcoded `"role"` with `roleKey.value`. Handle fallback for when no role dimension exists in the template but commitments are present.

- [ ] **Step 1: Add roleKey computed**

In `src/components/DimensionPopover.vue`, add after `goalKey` computed (~line 55):

```typescript
const roleKey = computed(() => {
  const role = props.dimensions.find(d => d.source === "commitments:role");
  return role?.key ?? "role";
});

const hasRoleDimension = computed(() =>
  props.dimensions.some(d => d.source === "commitments:role")
);
```

- [ ] **Step 2: Update presentDimKeys to use roleKey**

Find where `presentDimKeys` is defined (used for displaying dimension labels) and update to use `roleKey.value` instead of hardcoded `"role"`.

- [ ] **Step 3: Update activeValues computed**

In the `activeValues` computed (~line 61):
```typescript
if (selectedDimKey.value === "role") {
```
→
```typescript
if (selectedDimKey.value === roleKey.value) {
```

- [ ] **Step 4: Update cross-filter reference**

In `activeValues` computed (~line 77):
```typescript
const existingRole = props.dimValues["role"];
```
→
```typescript
const existingRole = props.dimValues[roleKey.value];
```

- [ ] **Step 5: Update valHeaderName computed**

In `valHeaderName` (~line 94):
```typescript
if (selectedDimKey.value === "role") return "Role";
```
→ Delete this line — the role dimension name should come from the dimension's `name` field now that it's in `visibleDims`.

- [ ] **Step 6: Update Enter key handler**

In `onKeydown` (~line 164):
```typescript
selectDim("role");
```
→
```typescript
selectDim(roleKey.value);
```

- [ ] **Step 7: Update the template pseudo-dimension section**

In the template (~lines 231-260), the hardcoded Role `<div>` with `data-test="dim-role"` needs to be replaced. The logic:

- If `hasRoleDimension` is true: the role dimension is already rendered by the `v-for` loop over `visibleDims`. Remove the standalone `<div>` entirely.
- If `hasRoleDimension` is false and `commitments.length > 0`: keep the old pseudo-injection as fallback, but use `roleKey.value` for data binding.

Change the standalone `v-if="commitments.length > 0"` block to:
```html
<div
  v-if="!hasRoleDimension && commitments.length > 0"
  data-test="dim-role"
  ...
>
```

Replace all `dimValues['role']` with `dimValues[roleKey]` in this block.
Replace `selectDim('role')` with `selectDim(roleKey)` in this block.
Replace `barColor('role')` with `barColor(roleKey)` in this block.

- [ ] **Step 8: Update DimensionPopover tests**

In `src/__tests__/components/DimensionPopover.test.ts`, update:
1. The test fixture `dimensions` array to include a role dimension when testing the integrated path
2. Update any assertion strings from `"role"` to use the dynamic key
3. Keep tests for the fallback path (no role dimension in array, commitments present)

- [ ] **Step 9: Run frontend tests**

Run: `pnpm vitest run src/__tests__/components/DimensionPopover.test.ts`

- [ ] **Step 10: Commit**

```bash
git add src/components/DimensionPopover.vue src/__tests__/components/DimensionPopover.test.ts
git commit -m "feat: add roleKey computed, replace hardcoded role in DimensionPopover"
```

---

### Task 7: Update compute_attribution and annotate_day_file with role_key parameter

**Files:**
- Modify: `src-tauri/src/commands.rs:652-714`

**Description:** Add `role_key` parameter to `compute_attribution` and `annotate_day_file`. Update all call sites.

- [ ] **Step 1: Update compute_attribution signature**

In `src-tauri/src/commands.rs:652-684`, change the function to accept `role_key`:

```rust
fn compute_attribution(
    dimensions: &BTreeMap<String, String>,
    role_key: &str,
    goal_key: &str,
    goal_to_role: &std::collections::HashMap<String, String>,
    role_to_goals: &std::collections::HashMap<String, Vec<String>>,
) -> crate::models::Attribution {
    use crate::models::Attribution;
    let role = dimensions.get(role_key);
    let goal = dimensions.get(goal_key);
    // ... rest unchanged
```

- [ ] **Step 2: Update annotate_day_file signature**

In `src-tauri/src/commands.rs:706-714`, change:

```rust
fn annotate_day_file(
    day_file: &mut crate::models::DayFile,
    role_key: &str,
    goal_key: &str,
    goal_to_role: &std::collections::HashMap<String, String>,
    role_to_goals: &std::collections::HashMap<String, Vec<String>>,
) {
    for entry in &mut day_file.entries {
        entry.attribution = compute_attribution(&entry.dimensions, role_key, goal_key, goal_to_role, role_to_goals);
    }
}
```

- [ ] **Step 3: Update call site in load_root_state (line ~275)**

Add `let role_key = role_dim_key(root, now.year(), now.month());` after the `let goal_key = ...` line, and pass `&role_key` as the second argument to `annotate_day_file`.

- [ ] **Step 4: Update call site in get_entries (line ~398)**

Add `let role_key = role_dim_key(root, year, month);` after the `let goal_key = ...` line, and pass it to `annotate_day_file`.

- [ ] **Step 5: Update call site in append_entry (line ~452)**

Add `let role_key = role_dim_key(root, year, month);` after the `let goal_key = ...` line, and pass it to `compute_attribution`.

- [ ] **Step 6: Update call site in update_entry (line ~513)**

Add `let role_key = role_dim_key(root, year, month);` after the `let goal_key = ...` line, and pass it to `annotate_day_file`.

- [ ] **Step 7: Update call site in delete_entry (line ~563)**

Add `let role_key = role_dim_key(root, year, month);` after the `let goal_key = ...` line, and pass it to `annotate_day_file`.

- [ ] **Step 8: Update tests (lines ~2266-2359)**

In all `compute_attribution` test calls, add `"role"` as the second argument:
```rust
let result = compute_attribution(&dims, "role", "goal", &goal_to_role, &role_to_goals);
```

For `test_compute_attribution_dynamic_goal_key` (line ~2351), use `"role"` for the role_key:
```rust
let result = compute_attribution(&dims, "role", "objective", &goal_to_role, &role_to_goals);
```

- [ ] **Step 9: Run cargo test**

Run: `cargo test`
Workdir: `src-tauri`

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: add role_key param to compute_attribution and annotate_day_file"
```

---

### Task 8: Update get_commitment_progress with dynamic role key

**Files:**
- Modify: `src-tauri/src/commands.rs:757,783,816`

**Description:** Replace hardcoded `"role"` in `get_commitment_progress` with `role_dim_key()`.

- [ ] **Step 1: Add role_key variable**

After `let goal_key = goal_dim_key(root, year, month);` (~line 757), add:
```rust
let role_key = role_dim_key(root, year, month);
```

- [ ] **Step 2: Replace hardcoded references**

In the scan loop (~lines 783, 816), change:
```rust
e.dimensions.get("role")
```
to:
```rust
e.dimensions.get(&role_key)
```

- [ ] **Step 3: Run cargo test**

Run: `cargo test -p tauri_app_lib -- test_get_commitment_progress`
Workdir: `src-tauri`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: use dynamic role key in get_commitment_progress"
```

---

### Task 9: Update set_commitments role rename/delete with dynamic role key

**Files:**
- Modify: `src-tauri/src/commands.rs:935-936,970-971`

**Description:** Replace hardcoded `"role"` in set_commitments role propagation with dynamic key.

- [ ] **Step 1: Add role_key variable**

In `set_commitments`, add after obtaining `root`:
```rust
let role_key = role_dim_key(root, year, month);
```

- [ ] **Step 2: Replace in role rename section (~lines 935-936)**

Change:
```rust
e.dimensions.get("role").map(|r| r == old_name).unwrap_or(false)
```
to:
```rust
e.dimensions.get(&role_key).map(|r| r == old_name).unwrap_or(false)
```

Change:
```rust
e.dimensions.insert("role".to_string(), new_name.to_string());
```
to:
```rust
e.dimensions.insert(role_key.clone(), new_name.to_string());
```

- [ ] **Step 3: Replace in role delete section (~lines 970-971)**

Change:
```rust
e.dimensions.get("role").map(|r| r == *role_name).unwrap_or(false)
```
to:
```rust
e.dimensions.get(&role_key).map(|r| r == *role_name).unwrap_or(false)
```

Change:
```rust
e.dimensions.remove("role");
```
to:
```rust
e.dimensions.remove(&role_key);
```

- [ ] **Step 4: Run cargo test**

Run: `cargo test`
Workdir: `src-tauri`

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: use dynamic role key in set_commitments role propagation"
```

---

### Task 10: Resolve new LSP/compiler todo about pending renames

**Files:**
- Modify: `src-tauri/src/commands.rs:275,398,452,513,563` (annotate_day_file + compute_attribution call sites if missed in Task 7)

**Description:** After Tasks 7-9, ensure all call sites pass the `role_key` parameter correctly. Run full cargo test to catch any remaining issues.

- [ ] **Step 1: Run full Rust test suite**

Run: `cargo test`
Workdir: `src-tauri`

Expected: All tests pass

- [ ] **Step 2: Fix any remaining compilation/test errors**

Check each error and fix individually. All hardcoded `"role"` in `dimensions.get()` calls should now use the dynamic key.

- [ ] **Step 3: Commit if any fixes needed**

```bash
git add src-tauri/
git commit -m "fix: remaining role key dynamic resolution fixes"
```

---

### Task 11: Rename from_template → usingDefaultDimensions in Rust

**Files:**
- Modify: `src-tauri/src/models.rs:151,178-181`
- Modify: `src-tauri/src/commands.rs:263-268,281,623-639`
- Modify: `src-tauri/src/config.rs:442,510,518` (test data)

**Description:** Pure rename of the field name.

- [ ] **Step 1: Update models.rs**

In `src-tauri/src/models.rs:151` (`InitResult::Ready.from_template`):
```rust
from_template: bool,
```
→
```rust
usingDefaultDimensions: bool,
```

In `src-tauri/src/models.rs:178-181` (`MonthDimensions.from_template`):
```rust
pub from_template: bool,
```
→
```rust
pub usingDefaultDimensions: bool,
```

- [ ] **Step 2: Update all references in models.rs tests**

In `src-tauri/src/models.rs`, search for `from_template` in test code (lines ~243, 270, 397) and change to `usingDefaultDimensions`.

- [ ] **Step 3: Update commands.rs references**

In `src-tauri/src/commands.rs`:
- Line 263: `let from_template = ...` → `let usingDefaultDimensions = ...`
- Line 264: `let dimensions = if from_template` → `let dimensions = if usingDefaultDimensions`
- Line 281: `from_template,` → `usingDefaultDimensions,`
- Line 623: `let from_template = ...` → `let usingDefaultDimensions = ...`
- Line 637: `from_template={}` → `usingDefaultDimensions={}`
- Line 639: `MonthDimensions { dimensions, from_template }` → `MonthDimensions { dimensions, usingDefaultDimensions }`

- [ ] **Step 4: Run cargo test**

Run: `cargo test`
Workdir: `src-tauri`

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/commands.rs
git commit -m "refactor: rename from_template → usingDefaultDimensions in Rust"
```

---

### Task 12: Replace _monthly.md reads with dimensions.yaml in load_root_state

**Files:**
- Modify: `src-tauri/src/commands.rs:208-268`

**Description:** Remove `read_monthly_file_safe` call and `validate_monthly` call from `load_root_state`. Compute `usingDefaultDimensions` from `read_dimensions_file`.

- [ ] **Step 1: Replace the _monthly.md read block**

In `src-tauri/src/commands.rs`, read the current code ~lines 208-268 and replace the monthly read block:

```rust
    let all_errors = validate_dimensions(&template.dimensions);

    let now = chrono::Local::now();

    let monthly_dims = files::read_dimensions_file(root, now.year(), now.month()).unwrap_or_default();
    let usingDefaultDimensions = monthly_dims.is_empty();

    // Read commitments from commitments.yaml
    let commitments = match files::read_commitments_file(root, now.year(), now.month()) {
        // ... (keep existing commitments read code)
        // Remove the validate_monthly(&monthly) line from this section if present
    };
```

Specifically:
- Delete lines ~208-217 (the `read_monthly_file_safe` call and `monthly` variable)
- Delete line ~218 (`all_errors.extend(validate_monthly(&monthly));`)
- Replace line ~263 (`let from_template = monthly.dimensions.is_empty();`) with the new dimensions-based logic
- Replace lines ~264-268 (dimensions assignment) with `let dimensions = files::resolve_month_dimensions(root, now.year(), now.month()).unwrap_or_default();`

- [ ] **Step 2: Run cargo test**

Run: `cargo test -- load_root_state`
Workdir: `src-tauri`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "refactor: replace _monthly.md reads with dimensions.yaml in load_root_state"
```

---

### Task 13: Replace _monthly.md read in get_month_dimensions

**Files:**
- Modify: `src-tauri/src/commands.rs:620-639`

**Description:** Remove `read_monthly_file` call, use `read_dimensions_file` to determine `usingDefaultDimensions`.

- [ ] **Step 1: Replace the function body**

In `src-tauri/src/commands.rs:620-639`, replace the `get_month_dimensions` function:

```rust
pub fn get_month_dimensions(
    root_path: String,
    year: i32,
    month: u32,
) -> Result<MonthDimensions, String> {
    error_log::log_command_enter("get_month_dimensions", &format!("{}-{:02}", year, month));
    let root = std::path::Path::new(&root_path);
    let usingDefaultDimensions = files::read_dimensions_file(root, year, month)
        .map(|d| d.is_empty())
        .unwrap_or(true);
    let dimensions = files::resolve_month_dimensions(root, year, month)?;
    error_log::log_command_exit(
        "get_month_dimensions",
        true,
        &format!("{} dims, usingDefaultDimensions={}", dimensions.len(), usingDefaultDimensions),
    );
    Ok(MonthDimensions { dimensions, usingDefaultDimensions })
}
```

- [ ] **Step 2: Run cargo test**

Run: `cargo test -- get_month_dimensions`
Workdir: `src-tauri`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "refactor: replace _monthly.md read with dimensions.yaml in get_month_dimensions"
```

---

### Task 14: Replace validate_monthly in watcher commitments-changed handler

**Files:**
- Modify: `src-tauri/src/config.rs:359-384`

**Description:** Remove synthetic `MonthlyFile` construction, use `validate_commitments` directly.

- [ ] **Step 1: Replace the validate_monthly call**

In `src-tauri/src/config.rs:356-384`, change the commitments-changed handler:

```rust
match files::read_commitments_file(&watch_root, year, month) {
    Ok(commitments) => {
        let dims = files::read_dimensions_file(&watch_root, year, month)
            .unwrap_or_default();
        let mut errors = validate_dimensions(&dims);
        if let Err(e) = validate_commitments_static(&commitments) {
            errors.push(ConfigErrorDetail {
                kind: "CommitmentValidation".to_string(),
                message: e,
            });
        }
        if let Err(e) = app_handle.emit("commitments-changed", &errors) {
            // ...
```
Note: `validate_commitments` is defined in `commands.rs` and not accessible from `config.rs`. We need to either:
1. Move it to config.rs, or  
2. Call it inline, or
3. Import it from commands (doesn't work easily since it's not pub)

Check the actual function signature. It's `fn validate_commitments(commitments: &[Commitment]) -> Result<(), String>` in commands.rs. We'll need to either make it `pub(crate)` and call it, or duplicate the logic (the watcher just needs basic validation).

The simplest approach: move `validate_commitments` from `commands.rs` to `config.rs` (pure validation, no I/O). Then call it from both places.

In `src-tauri/src/commands.rs:1185-1216`, cut the entire `validate_commitments` function and paste it into `src-tauri/src/config.rs` (after `validate_dimensions`). Make it `pub(crate)`.

In `src-tauri/src/commands.rs`, update the call sites from `validate_commitments(...)` to `crate::config::validate_commitments(...)`.

In `src-tauri/src/config.rs`, update the watcher event loop:
```rust
match files::read_commitments_file(&watch_root, year, month) {
    Ok(commitments) => {
        let dims = files::read_dimensions_file(&watch_root, year, month)
            .unwrap_or_default();
        let mut errors = validate_dimensions(&dims);
        if let Err(e) = validate_commitments(&commitments) {
            errors.push(ConfigErrorDetail {
                kind: "CommitmentValidation".to_string(),
                message: e,
            });
        }
        if let Err(e) = app_handle.emit("commitments-changed", &errors) {
        // ... rest unchanged
```

- [ ] **Step 2: Run cargo test**

Run: `cargo test`
Workdir: `src-tauri`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/src/commands.rs
git commit -m "refactor: replace validate_monthly with validate_commitments in watcher"
```

---

### Task 15: Delete dead _monthly.md functions and structs

**Files:**
- Modify: `src-tauri/src/models.rs:31-37` (delete MonthlyFile)
- Modify: `src-tauri/src/models.rs:298` (delete MonthlyFile reference in test)
- Modify: `src-tauri/src/commands.rs:39-58` (delete read_monthly_file_safe)
- Modify: `src-tauri/src/commands.rs:1702-1713` (delete test_read_monthly_file_safe_corrupt)
- Modify: `src-tauri/src/files.rs:41-45` (delete monthly_path)
- Modify: `src-tauri/src/files.rs:200-232` (delete read_monthly_file, write_monthly_file)
- Modify: `src-tauri/src/files.rs:380-399` (delete ensure_month_instantiated)
- Modify: `src-tauri/src/files.rs:582-585` (delete test_monthly_path)
- Modify: `src-tauri/src/config.rs:137-170` (delete validate_monthly)
- Modify: `src-tauri/src/config.rs:397` (delete MonthlyFile import in test)
- Modify: `src-tauri/src/config.rs:43-52` (rename month_from_monthly_path)
- Modify: `src-tauri/src/config.rs:613-629` (rename test functions)

**Description:** Delete all code exclusively used for _monthly.md I/O.

- [ ] **Step 1: Delete MonthlyFile struct**

In `src-tauri/src/models.rs:29-37`, delete the entire `MonthlyFile` struct and the `// --- Monthly file ---` comment.

In `src-tauri/src/models.rs:298`, delete the `"_monthly.md"` path reference in test `init_result_config_error_with_scan_warnings`.

- [ ] **Step 2: Delete read_monthly_file_safe from commands.rs**

Delete lines ~39-58 (the entire `read_monthly_file_safe` function) and its test at lines ~1702-1713 (`test_read_monthly_file_safe_corrupt`).

Also delete: any remaining `use crate::models::MonthlyFile;` imports in commands.rs that are now unused.

- [ ] **Step 3: Delete _monthly.md functions from files.rs**

Delete:
- `monthly_path` function (lines ~41-45) and its test `test_monthly_path` (lines ~582-585)
- `read_monthly_file` function (lines ~200-212)
- `write_monthly_file` function (lines ~215-232)
- `ensure_month_instantiated` function (lines ~380-399)

- [ ] **Step 4: Delete validate_monthly from config.rs**

Delete the entire `validate_monthly` function (lines ~137-170).

Delete `MonthlyFile` import from test module if still present.

- [ ] **Step 5: Rename month_from_monthly_path → extract_year_month**

In `src-tauri/src/config.rs:43-52`, rename the function:

```rust
fn extract_year_month(path: &std::path::Path) -> Option<(i32, u32)> {
    let mut comps = path.components().rev();
    comps.next()?; // filename
    let month: u32 = comps.next()?.as_os_str().to_str()?.parse().ok()?;
    let year: i32 = comps.next()?.as_os_str().to_str()?.parse().ok()?;
    if (1..=12).contains(&month) {
        Some((year, month))
    } else {
        None
    }
}
```

Update the two call sites (lines ~307, ~346) from `month_from_monthly_path(path)` to `extract_year_month(path)`.

Rename the two test functions (lines ~613, ~619):
- `test_month_from_monthly_path_extracts_changed_month` → `test_extract_year_month`
- `test_month_from_monthly_path_rejects_non_numeric_and_bad_month` → `test_extract_year_month_rejects_invalid`

Update their internal calls from `month_from_monthly_path(...)` to `extract_year_month(...)`.

- [ ] **Step 6: Run cargo test**

Run: `cargo test`
Workdir: `src-tauri`

Expected: All tests pass (minus _monthly.md-related tests we deleted)

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/commands.rs src-tauri/src/files.rs src-tauri/src/config.rs
git commit -m "refactor: delete _monthly.md dead functions and structs"
```

---

### Task 16: Clean up _monthly.md skip checks throughout codebase

**Files:**
- Modify: `src-tauri/src/commands.rs` (7 occurrences)
- Modify: `src-tauri/src/scan.rs:70-72` (skip logic + test at ~245-262)
- Modify: `src-tauri/src/operation_log.rs:441`

**Description:** Remove all `file_name == "_monthly.md" ||` guards in .md file loops. Delete the _monthly.md skip test in scan.rs.

- [ ] **Step 1: Clean up commands.rs loops**

Search for `"_monthly.md"` in `src-tauri/src/commands.rs` and remove the condition from each occurrence. Pattern:
```rust
if file_name == "_monthly.md" || !file_name.ends_with(".md") {
```
→
```rust
if !file_name.ends_with(".md") {
```

There are ~7 occurrences.

- [ ] **Step 2: Clean up scan.rs**

Delete lines ~70-72:
```rust
// _monthly.md is handled by config module
if file_name == "_monthly.md" {
    continue;
}
```

Delete the test `test_monthly_file_skipped` (lines ~248-262):
```rust
#[test]
fn test_monthly_file_skipped() {
    let root = temp_root();
    let monthly = root.join("2026/06/_monthly.md");
    write_file(&monthly, "garbage content that would be corrupt if scanned\n");
    let warnings = scan_data_dir(&root);
    assert!(
        warnings.is_empty(),
        "_monthly.md should be skipped, got {:?}",
        warnings
    );
    fs::remove_dir_all(&root).expect("cleanup");
}
```

- [ ] **Step 3: Clean up operation_log.rs**

In `src-tauri/src/operation_log.rs:441`, remove:
```rust
if file_name == "_monthly.md" {
    continue;
}
```

- [ ] **Step 4: Run cargo test**

Run: `cargo test`
Workdir: `src-tauri`

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/scan.rs src-tauri/src/operation_log.rs
git commit -m "refactor: remove _monthly.md skip checks"
```

---

### Task 17: Delete _monthly.md test fixture

**Files:**
- Delete: `src-tauri/tests/fixtures/2026/06/_monthly.md`

**Description:** Delete the only test fixture _monthly.md file.

- [ ] **Step 1: Delete the file**

```bash
rm src-tauri/tests/fixtures/2026/06/_monthly.md
```

- [ ] **Step 2: Run cargo test to verify nothing breaks**

Run: `cargo test`
Workdir: `src-tauri`

- [ ] **Step 3: Commit**

```bash
git rm src-tauri/tests/fixtures/2026/06/_monthly.md
git commit -m "test: remove _monthly.md test fixture"
```

---

### Task 18: Update frontend from_template → usingDefaultDimensions

**Files:**
- Modify: `src/types.ts:12,101`
- Modify: `src/stores/useStore.ts:14,36`
- Modify: `src/components/MonthView.vue:28,101-108,449`
- Modify: `src/App.vue:82,99`
- Modify: `src/utils/applyInitResult.ts:23`
- Modify: All `src/__tests__/**/*.test.ts` files with `fromTemplate` references

**Description:** Pure rename across the frontend.

- [ ] **Step 1: Update types.ts**

In `src/types.ts`, change:
- Line 12: `from_template: boolean;` → `usingDefaultDimensions: boolean;`
- Line 101: `from_template: boolean;` → `usingDefaultDimensions: boolean;`

- [ ] **Step 2: Update stores/useStore.ts**

In `src/stores/useStore.ts`:
- Line 14: `fromTemplate: boolean;` → `usingDefaultDimensions: boolean;`
- Line 36: `fromTemplate: false,` → `usingDefaultDimensions: false,`

- [ ] **Step 3: Update MonthView.vue**

In `src/components/MonthView.vue`:
- Line 28: `store.fromTemplate = false;` → `store.usingDefaultDimensions = false;`
- Line 101: comment update only
- Line 108: `store.fromTemplate = md.from_template;` → `store.usingDefaultDimensions = md.usingDefaultDimensions;`
- Line 449: `v-if="store.fromTemplate"` → `v-if="store.usingDefaultDimensions"`

- [ ] **Step 4: Update App.vue**

In `src/App.vue`:
- Line 82: `store.fromTemplate = result.from_template;` → `store.usingDefaultDimensions = result.usingDefaultDimensions;`
- Line 99: `store.fromTemplate = dimsResult.from_template;` → `store.usingDefaultDimensions = dimsResult.usingDefaultDimensions;`

- [ ] **Step 5: Update applyInitResult.ts**

In `src/utils/applyInitResult.ts:23`:
```typescript
store.fromTemplate = result.data.from_template;
```
→
```typescript
store.usingDefaultDimensions = result.data.usingDefaultDimensions;
```

- [ ] **Step 6: Update all test files**

In all `src/__tests__/**/*.test.ts` files, replace:
- `fromTemplate: true` → `usingDefaultDimensions: true`
- `fromTemplate: false` → `usingDefaultDimensions: false`
- `from_template: true` → `usingDefaultDimensions: true`
- `from_template: false` → `usingDefaultDimensions: false`
- `store.fromTemplate` → `store.usingDefaultDimensions`
- `expect(store.fromTemplate)` → `expect(store.usingDefaultDimensions)`

- [ ] **Step 7: Run frontend tests**

Run: `pnpm vitest run`

- [ ] **Step 8: Commit**

```bash
git add src/
git commit -m "refactor: rename fromTemplate → usingDefaultDimensions in frontend"
```

---

### Task 19: Full test suite verification

**Description:** Run both Rust and frontend tests to ensure everything passes.

- [ ] **Step 1: Run Rust tests**

Run: `cargo test`
Workdir: `src-tauri`

Expected: All tests pass.

- [ ] **Step 2: Run frontend tests**

Run: `pnpm vitest run`

Expected: All tests pass.

- [ ] **Step 3: Run frontend type check**

Run: `pnpm vue-tsc --noEmit`

Expected: No type errors.

- [ ] **Step 4: Fix any remaining issues**

If any test or type check fails, fix and commit.

- [ ] **Step 5: Final commit if needed**

```bash
git add -A
git commit -m "fix: remaining test/type issues from refactoring"
```

---

## Task Summary

| # | Task | Dependencies |
|---|------|--------------|
| 1 | Rename source in config.rs validate_dimensions | None |
| 2 | Rename monthly_dim_key → goal_dim_key, add role_dim_key | Task 1 |
| 3 | Update test data source strings (Rust) | Task 2 |
| 4 | Update integration test inline YAML | Task 3 |
| 5 | Update frontend source + goalKey | Task 4 |
| 6 | Add roleKey to DimensionPopover | Task 5 |
| 7 | Update compute_attribution/anotate_day_file | Task 2 |
| 8 | Update get_commitment_progress | Task 7 |
| 9 | Update set_commitments role propagation | Task 7 |
| 10 | Fix remaining compilation issues | Task 9 |
| 11 | Rename from_template → usingDefaultDimensions (Rust) | Task 10 |
| 12 | Replace _monthly.md reads in load_root_state | Task 11 |
| 13 | Replace _monthly.md read in get_month_dimensions | Task 12 |
| 14 | Replace validate_monthly in watcher | Task 12 |
| 15 | Delete dead _monthly.md functions and structs | Task 14 |
| 16 | Clean up _monthly.md skip checks | Task 15 |
| 17 | Delete _monthly.md test fixture | Task 16 |
| 18 | Update frontend from_template → usingDefaultDimensions | Task 11 |
| 19 | Full test suite verification | All |
