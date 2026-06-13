# Required Dimensions — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **TDD split:** Rust backend tasks are split into test-writer / implementer pairs — a dedicated agent writes failing tests, then a different agent implements to make them pass. Frontend tasks are single-agent (no component test infra).

**Goal:** Add required/optional flag to Entry dimensions — required dimensions must be filled before submission, enforced by both backend validation and frontend UX.

**Architecture:** Add `required: bool` (default false) to the Dimension model in Rust and TypeScript. Backend validates at `append_entry`/`update_entry` time. Frontend modifies the @mention menu to loop (returning to dimension list after value selection) and shows red dashed chips for missing required dimensions. DimensionPanel labels gain a red `*` indicator.

**Tech Stack:** Rust (Tauri 2.x, yaml_serde), TypeScript (Vue 3 Composition API), Vitest (frontend), cargo test (backend)

---

## File Structure

| File | Role | Change |
|------|------|--------|
| `src-tauri/src/models.rs` | Dimension struct | Add `required` field |
| `src-tauri/src/commands.rs` | Validation + tests | `validate_required_dimensions`, wire into append/update |
| `src-tauri/src/config.rs` | Tests only | Add `required: false` to test fixtures |
| `src-tauri/tests/entry_crud_integration.rs` | Integration tests | Required-dimension CRUD tests |
| `src/types.ts` | Dimension interface | Add `required: boolean` |
| `src/components/EntryInput.vue` | @ menu + chips | Loop menu, red missing chips, footer state |
| `src/components/DimensionPanel.vue` | Labels | Red `*` indicator + legend |

---

### Task 1: Test-Writer — add `required` field to Dimension + write failing tests for `validate_required_dimensions`

**Role:** Test-Writer. Add the `required` field (1 line in models.rs — prerequisite so tests compile), then write tests. Do NOT implement `validate_required_dimensions`. The unit tests MUST fail to compile because the function doesn't exist.

**Files:**
- Modify: `src-tauri/src/models.rs` (add `required` field only — no other changes)
- Modify: `src-tauri/src/config.rs` (add serde deserialization tests)
- Modify: `src-tauri/src/commands.rs` (add unit tests for `validate_required_dimensions`)

**Contract for the implementer (Task 2):**

```rust
// Function to implement in commands.rs:
// pub fn validate_required_dimensions(
//     config: &Config,
//     dimensions: &std::collections::HashMap<String, String>,
// ) -> Result<(), String>
```

- [ ] **Step 1: Add `required` field to Dimension struct (prerequisite)**

In `src-tauri/src/models.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimension {
    pub name: String,
    pub key: String,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    #[serde(default)]  // false when absent
    pub required: bool,
}
```

This 1-line change enables tests to reference `required: true`. Existing tests will break (missing field) — that's the implementer's job in Task 2.

- [ ] **Step 2: Write serde deserialization tests for `required` field**

Add to `src-tauri/src/config.rs`, inside `#[cfg(test)] mod tests`:

```rust
#[test]
fn test_dimension_required_defaults_to_false() {
    let yaml = "name: Test\nkey: test\nsource: static\nvalues: [a]";
    let dim: Dimension = yaml_serde::from_str(yaml).unwrap();
    assert!(!dim.required);
}

#[test]
fn test_dimension_required_true() {
    let yaml = "name: Test\nkey: test\nsource: static\nvalues: [a]\nrequired: true";
    let dim: Dimension = yaml_serde::from_str(yaml).unwrap();
    assert!(dim.required);
}
```

- [ ] **Step 3: Write unit tests for `validate_required_dimensions`**

Add to `src-tauri/src/commands.rs`, inside `#[cfg(test)] mod tests`:

```rust
use crate::models::{Config, Dimension};
use std::collections::HashMap;

fn make_config(required_keys: &[&str]) -> Config {
    Config {
        dimensions: vec![
            Dimension {
                name: "Biz".into(), key: "biz".into(), source: "static".into(),
                values: Some(vec!["A".into()]), required: required_keys.contains(&"biz"),
            },
            Dimension {
                name: "Cat".into(), key: "cat".into(), source: "static".into(),
                values: Some(vec!["X".into()]), required: required_keys.contains(&"cat"),
            },
            Dimension {
                name: "Goal".into(), key: "goal".into(), source: "monthly".into(),
                values: None, required: required_keys.contains(&"goal"),
            },
        ],
    }
}

#[test]
fn test_validate_required_all_present() {
    let config = make_config(&["biz"]);
    let mut dims = HashMap::new();
    dims.insert("biz".to_string(), "A".to_string());
    assert!(validate_required_dimensions(&config, &dims).is_ok());
}

#[test]
fn test_validate_required_missing_one() {
    let config = make_config(&["biz", "cat"]);
    let mut dims = HashMap::new();
    dims.insert("biz".to_string(), "A".to_string());
    // cat is missing
    let err = validate_required_dimensions(&config, &dims).unwrap_err();
    assert!(err.contains("Cat"), "expected error to mention 'Cat', got: {}", err);
    assert!(err.contains("Missing required dimension"));
}

#[test]
fn test_validate_required_none_required() {
    let config = make_config(&[]);
    let dims = HashMap::new(); // empty is fine — nothing required
    assert!(validate_required_dimensions(&config, &dims).is_ok());
}

#[test]
fn test_validate_required_empty_dimensions() {
    let config = make_config(&["biz"]);
    let dims = HashMap::new();
    let err = validate_required_dimensions(&config, &dims).unwrap_err();
    assert!(err.contains("Biz"));
}
```

- [ ] **Step 4: Verify tests fail**

```bash
cd src-tauri && cargo test 2>&1 | grep -E "error|FAILED|cannot find"
```

Expected: Only `validate_required_dimensions` not found (serde tests should compile and pass since `required` field exists). The existing tests in `config.rs` will have compilation errors from the new `required` field — that's expected and will be fixed by the implementer in Task 2.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/config.rs src-tauri/src/commands.rs
git commit -m "test: add Dimension.required field + failing tests for validate_required_dimensions"
```

---

### Task 2: Implementer — fix existing tests + implement `validate_required_dimensions`

**Role:** Implementer. The `required` field already exists on Dimension (Task 1). Your job: fix existing tests that broke, then implement `validate_required_dimensions` to make the 4 new unit tests pass.

**Files:**
- Modify: `src-tauri/src/config.rs` (fix existing test compilation — add `required: false`)
- Modify: `src-tauri/src/commands.rs` (add `validate_required_dimensions` function)

- [ ] **Step 1: Read the failing test output to understand what needs fixing**

```bash
cd src-tauri && cargo test 2>&1 | head -60
```

You'll see:
- `config.rs` existing tests: `Dimension` struct literals missing field `required`
- `commands.rs` new tests: function `validate_required_dimensions` not found

- [ ] **Step 2: Fix existing Dimension literals in config.rs tests**

Every `Dimension { ... }` literal in `config.rs`'s test module needs `required: false`. There are ~7 literals. Example:

```rust
// Before:
Dimension { name: "Biz".into(), key: "biz".into(), source: "static".into(), values: Some(vec!["X".into()]) },
// After:
Dimension { name: "Biz".into(), key: "biz".into(), source: "static".into(), values: Some(vec!["X".into()]), required: false },
```

- [ ] **Step 3: Implement `validate_required_dimensions`**

Add to `src-tauri/src/commands.rs`, after `parse_duration` and before `#[tauri::command] pub fn init`:

```rust
/// Validate that all required dimensions have values in the entry.
/// Returns Ok(()) or Err with a human-readable message naming the first missing required dimension.
pub fn validate_required_dimensions(
    config: &Config,
    dimensions: &std::collections::HashMap<String, String>,
) -> Result<(), String> {
    for dim in &config.dimensions {
        if dim.required && !dimensions.contains_key(&dim.key) {
            return Err(format!("Missing required dimension: {}", dim.name));
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Run all tests to verify they pass**

```bash
cd src-tauri && cargo test
```

Expected: ALL tests PASS — both existing tests (fixed in Step 2) and the 6 new tests from Task 1.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/config.rs
git commit -m "feat: implement validate_required_dimensions, fix existing tests"
```

---

### Task 3: Test-Writer — integration tests for required dimension validation

**Role:** Test-Writer. Write ONLY integration tests. They MUST fail because `append_entry` and `update_entry` don't validate required dimensions yet.

**Files:**
- Modify: `src-tauri/tests/entry_crud_integration.rs`

**Contract:** The implementer (Task 4) will modify `commands.rs` so that:
- `append_entry` calls `validate_required_dimensions(&config, &entry.dimensions)?` before writing
- `update_entry` calls `validate_required_dimensions(&config, dims)?` only when `update.dimensions` is `Some`

- [ ] **Step 1: Write integration test — append rejects missing required dimension**

```rust
#[test]
fn test_append_entry_rejects_missing_required_dimension() {
    let suffix = "req_missing";
    setup(suffix);
    let root = test_root(suffix);

    fs::write(
        root.join("config.yaml"),
        "dimensions:\n  - name: Biz\n    key: biz\n    source: static\n    values: [A, B]\n    required: true\n  - name: Goal\n    key: goal\n    source: monthly\n",
    )
    .unwrap();

    let date = "2026-06-12";
    let new_entry = NewEntry {
        item: "Missing required dim".to_string(),
        duration: "30".to_string(),
        dimensions: HashMap::new(), // biz is required but missing
    };

    let result = tauri_app_lib::files::append_new_entry(&root, date, &new_entry);
    assert!(result.is_err(), "should reject entry with missing required dimension");
    let err = result.unwrap_err();
    assert!(
        err.contains("Missing required dimension"),
        "error should mention missing required dimension, got: {}",
        err
    );

    teardown(suffix);
}
```

- [ ] **Step 2: Write integration test — append accepts when required dimensions present**

```rust
#[test]
fn test_append_entry_accepts_when_required_dimensions_present() {
    let suffix = "req_ok";
    setup(suffix);
    let root = test_root(suffix);

    fs::write(
        root.join("config.yaml"),
        "dimensions:\n  - name: Biz\n    key: biz\n    source: static\n    values: [A, B]\n    required: true\n  - name: Goal\n    key: goal\n    source: monthly\n",
    )
    .unwrap();

    let date = "2026-06-12";
    let mut dims = HashMap::new();
    dims.insert("biz".to_string(), "A".to_string());

    let new_entry = NewEntry {
        item: "Has required dim".to_string(),
        duration: "30".to_string(),
        dimensions: dims,
    };

    let result = tauri_app_lib::files::append_new_entry(&root, date, &new_entry);
    assert!(result.is_ok(), "should accept entry with required dimensions present");

    // Verify it was written
    let day_file = tauri_app_lib::files::read_day_file(&root, date).unwrap();
    assert_eq!(day_file.entries.len(), 1);
    assert_eq!(day_file.entries[0].dimensions.get("biz").unwrap(), "A");

    teardown(suffix);
}
```

- [ ] **Step 3: Write integration test — update rejects clearing required dimension**

```rust
#[test]
fn test_update_entry_rejects_clearing_required_dimension() {
    let suffix = "req_update";
    setup(suffix);
    let root = test_root(suffix);

    fs::write(
        root.join("config.yaml"),
        "dimensions:\n  - name: Biz\n    key: biz\n    source: static\n    values: [A, B]\n    required: true\n  - name: Goal\n    key: goal\n    source: monthly\n",
    )
    .unwrap();

    let date = "2026-06-12";
    let mut dims = HashMap::new();
    dims.insert("biz".to_string(), "A".to_string());

    let entry = tauri_app_lib::files::append_new_entry(
        &root, date,
        &NewEntry { item: "Original".into(), duration: "30".into(), dimensions: dims },
    ).unwrap();

    // Try to update with empty dimensions (clearing required dim)
    let update = UpdateEntry {
        item: None,
        duration: None,
        dimensions: Some(HashMap::new()), // clears biz
    };

    let result = tauri_app_lib::files::update_entry_in_file(&root, date, &entry.id, &update);
    assert!(result.is_err(), "should reject update that clears required dimension");

    // Verify original entry unchanged
    let day_file = tauri_app_lib::files::read_day_file(&root, date).unwrap();
    assert_eq!(day_file.entries[0].dimensions.get("biz").unwrap(), "A");

    teardown(suffix);
}
```

- [ ] **Step 4: Verify tests fail**

```bash
cd src-tauri && cargo test --test entry_crud_integration test_append_entry_rejects
```

Expected: FAIL — `append_new_entry` succeeds, test expects error.

```bash
cd src-tauri && cargo test --test entry_crud_integration test_append_entry_accepts
```

Expected: PASS — this one should pass even without validation (required dim is present, written to file).

```bash
cd src-tauri && cargo test --test entry_crud_integration test_update_entry_rejects
```

Expected: FAIL — `update_entry_in_file` succeeds, test expects error.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/tests/entry_crud_integration.rs
git commit -m "test: failing integration tests for required dimension validation in commands"
```

---

### Task 4: Implementer — wire validation into `append_entry` and `update_entry`

**Role:** Implementer. Read the integration tests from Task 3. Modify `commands.rs` to make the 2 failing tests pass. The "accepts" test (Step 2) should already pass.

**Files:**
- Modify: `src-tauri/src/commands.rs` (append_entry, update_entry)

- [ ] **Step 1: Verify which tests fail and which pass**

```bash
cd src-tauri && cargo test --test entry_crud_integration 2>&1 | grep -E "test |FAILED|ok"
```

Expected: `test_append_entry_rejects...` FAILED, `test_update_entry_rejects...` FAILED, `test_append_entry_accepts...` ok.

- [ ] **Step 2: Wire validation into `append_entry`**

In `src-tauri/src/commands.rs`, modify `append_entry`:

```rust
#[tauri::command]
pub fn append_entry(root_path: String, date: String, entry: NewEntry) -> Result<Entry, String> {
    error_log::log_command_enter("append_entry", &format!("date={} item={} dur={}", date, entry.item, entry.duration));
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    let duration = parse_duration(&entry.duration)?;

    // --- NEW: validate required dimensions ---
    let config = files::read_config(root)?;
    validate_required_dimensions(&config, &entry.dimensions)?;
    // --- END NEW ---

    let entry = Entry {
        id: uuid::Uuid::new_v4().to_string(),
        item: entry.item,
        duration,
        dimensions: entry.dimensions,
    };
    let result = files::append_to_day_file(root, &date, &entry);
    let ok = result.is_ok();
    error_log::log_command_exit("append_entry", ok, &format!("id={}", entry.id));
    result
}
```

- [ ] **Step 3: Wire validation into `update_entry`**

Same file, modify `update_entry`:

```rust
#[tauri::command]
pub fn update_entry(root_path: String, date: String, entry_id: String, update: UpdateEntry) -> Result<DayFile, String> {
    error_log::log_command_enter("update_entry", &format!("date={} id={}", date, entry_id));
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    if let Some(ref dur_str) = update.duration {
        parse_duration(dur_str)?;
    }
    // --- NEW: validate required dimensions when dimensions are being updated ---
    if let Some(ref dims) = update.dimensions {
        let config = files::read_config(root)?;
        validate_required_dimensions(&config, dims)?;
    }
    // --- END NEW ---
    let result = files::update_entry_in_file(root, &date, &entry_id, &update);
    let ok = result.is_ok();
    error_log::log_command_exit("update_entry", ok, &format!("{} entries", result.as_ref().map(|d| d.entries.len()).unwrap_or(0)));
    result
}
```

- [ ] **Step 4: Run integration tests to verify all pass**

```bash
cd src-tauri && cargo test --test entry_crud_integration
```

Expected: ALL integration tests PASS.

- [ ] **Step 5: Run full Rust test suite**

```bash
cd src-tauri && cargo test
```

Expected: ALL tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: validate required dimensions in append_entry and update_entry"
```

---

### Task 5: TypeScript Dimension interface

**Role:** Single agent. Mechanical type change — add `required: boolean`.

**Files:**
- Modify: `src/types.ts`

- [ ] **Step 1: Add `required` field**

```typescript
export interface Dimension {
  name: string;
  key: string;
  source: "static" | "monthly";
  values?: string[];
  required: boolean;
}
```

- [ ] **Step 2: Verify type check passes**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook && npx vue-tsc --noEmit
```

Expected: Clean type check. (Dimension objects come from Rust backend via JSON deserialization — no manual construction in frontend code.)

- [ ] **Step 3: Commit**

```bash
git add src/types.ts
git commit -m "feat: add required field to TypeScript Dimension interface"
```

---

### Task 6: EntryInput.vue — @ menu loop + missing required chips

**Role:** Single agent. Frontend component changes — no component test infra, uses manual verification.

**Files:**
- Modify: `src/components/EntryInput.vue`

This task covers:
- Computed properties for required dimension state
- `insertAtChar()` helper for filter continuity when looping
- `openValMenuDirect()` for red chip click
- Modified `confirmSelection()` — val-phase loop back to dim list; dim-phase Enter closes when all required filled
- Modified `selectByIndex()` — val-phase loop (dim-phase unchanged — number keys always go to value list)
- Red dashed chip display in template
- Menu footer + dim list item badges

- [ ] **Step 1: Add computed properties for required dimension tracking**

After the existing `goalOptions` computed (line ~55), add:

```typescript
const allRequiredFilled = computed(() => {
  return props.dimensions
    .filter(d => d.required)
    .every(d => dimValues.value[d.key]);
});

const requiredRemaining = computed(() => {
  return props.dimensions
    .filter(d => d.required && !dimValues.value[d.key])
    .length;
});

const missingRequired = computed(() => {
  return props.dimensions
    .filter(d => d.required && !dimValues.value[d.key])
    .map(d => ({ key: d.key, name: d.name }));
});
```

- [ ] **Step 2: Add helper functions — `openValMenuDirect` and `insertAtChar`**

After `openDimMenu()` (line ~109), add:

```typescript
/// Open the @ menu directly at value selection for a specific dimension.
/// Used when clicking a missing-required red chip — no @mention to replace.
function openValMenuDirect(dimKey: string) {
  menuPhase.value = "val";
  activeDimKey.value = dimKey;
  selectedIndex.value = 0;
  filterText.value = "";
  menuVisible.value = true;
  // Focus the input so keyboard navigation works
  inputEl.value?.focus();
}

/// Insert a bare @ at cursor position, so `extractFilterFromInput`
/// can still extract filter text when the menu loops back to dim phase.
function insertAtChar() {
  const cursorPos = inputEl.value?.selectionStart ?? input.value.length;
  input.value = input.value.slice(0, cursorPos) + "@" + input.value.slice(cursorPos);
}
```

- [ ] **Step 3: Modify `confirmSelection()` — val-phase loop, dim-phase close**

In `confirmSelection()`, change the val-phase block:

```typescript
// REPLACE:
  } else if (menuPhase.value === "val" && activeDimKey.value && item.value) {
    dimValues.value = { ...dimValues.value, [activeDimKey.value]: item.value };
    removeMentionFromInput();
    closeMenu();
    inputEl.value?.focus();
  }

// WITH:
  } else if (menuPhase.value === "val" && activeDimKey.value && item.value) {
    dimValues.value = { ...dimValues.value, [activeDimKey.value]: item.value };
    removeMentionFromInput();
    if (allRequiredFilled.value) {
      closeMenu();
      inputEl.value?.focus();
    } else {
      insertAtChar();     // re-insert @ so filter works in dim phase
      openDimMenu();      // loop back to dimension list
    }
  }
```

Change the dim-phase block:

```typescript
// REPLACE:
  if (menuPhase.value === "dim" && item.key) {
    openValMenu(item.key);
  }

// WITH:
  if (menuPhase.value === "dim" && item.key) {
    if (allRequiredFilled.value) {
      removeMentionFromInput();
      closeMenu();
      inputEl.value?.focus();
    } else {
      openValMenu(item.key);
    }
  }
```

- [ ] **Step 4: Modify `selectByIndex()` — val-phase loop, dim-phase unchanged**

Number keys (1-9) in dim phase **always** open the value list (unchanged). Only val phase needs the loop:

```typescript
// REPLACE val-phase block:
  } else if (menuPhase.value === "val" && activeDimKey.value) {
    dimValues.value = { ...dimValues.value, [activeDimKey.value]: items[idx].value || items[idx].label };
    removeMentionFromInput();
    closeMenu();
    inputEl.value?.focus();
  }

// WITH:
  } else if (menuPhase.value === "val" && activeDimKey.value) {
    dimValues.value = { ...dimValues.value, [activeDimKey.value]: items[idx].value || items[idx].label };
    removeMentionFromInput();
    if (allRequiredFilled.value) {
      closeMenu();
      inputEl.value?.focus();
    } else {
      insertAtChar();
      openDimMenu();
    }
  }
```

Do NOT change the dim-phase branch of `selectByIndex` — number keys always navigate to value list.

- [ ] **Step 5: Add missing-required red dashed chips to template**

In the chips row `<div>` (around line ~454), after the `v-for="dim in dimensions"` loop's closing `</span>` and before the `@ to set dimensions` placeholder, add:

```html
<!-- Missing required chips (red dashed) -->
<span
  v-for="m in missingRequired"
  :key="'missing-' + m.key"
  class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs cursor-pointer border border-dashed border-red-400 bg-red-50 text-red-700"
  @click="openValMenuDirect(m.key)"
>
  + {{ m.name }}
</span>
```

- [ ] **Step 6: Add menu footer for required-remaining state**

Inside the menu `<div ref="menuEl">`, after the "No matches" empty state `<div>`, add:

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

- [ ] **Step 7: Add required/✓ badges to dim list items in menu**

Extend `MenuItem` interface (around line 68):

```typescript
interface MenuItem {
  label: string;
  sub?: string | null;
  key?: string;
  value?: string;
  required?: boolean;
}
```

In `getMenuItems()`, add `required` to the dim phase return:

```typescript
// In the dim-phase branch of getMenuItems():
return props.dimensions
  .filter(...)
  .map((d) => ({
    label: d.name,
    sub: DIM_ALIASES.value[d.key] || d.key,
    key: d.key,
    required: d.required,
  }));
```

In the template, inside the dim-phase menu item, after `<span class="flex-1">{{ item.label }}</span>`, add:

```html
<span v-if="menuPhase === 'dim' && item.required && !dimValues[item.key || '']" class="text-[10px] text-red-400">required</span>
<span v-else-if="menuPhase === 'dim' && item.required && dimValues[item.key || '']" class="text-[10px] text-green-500">{{ dimValues[item.key || ''] }} ✓</span>
```

- [ ] **Step 8: Run type check**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook && npx vue-tsc --noEmit
```

Expected: Clean.

- [ ] **Step 9: Manual verification**

Launch the app and verify:

1. Type `@` → dim list shows required badges, filled dims show `Value ✓`
2. Pick a required dim → value list → pick value → menu returns to dim list (not closed)
3. After filling all required dims → footer: `All required ✓ · Enter to confirm`
4. Press Enter → menu closes; chips show all values
5. Type an entry without setting required dims, click Log → red dashed chips appear below input
6. Click red chip `+ Business line` → value list opens for Business line
7. Pick value → if more required dims remaining, returns to dim list; otherwise closes
8. Number keys (1-9) in dim phase always navigate to value list (even when all required are filled)
9. Press Esc anytime → menu closes (even with missing required dims — red chips handle that flow)

- [ ] **Step 10: Commit**

```bash
git add src/components/EntryInput.vue
git commit -m "feat: @ menu loop + missing required chips in EntryInput"
```

---

### Task 7: DimensionPanel.vue — required `*` indicator

**Role:** Single agent. Small template change.

**Files:**
- Modify: `src/components/DimensionPanel.vue`

- [ ] **Step 1: Add `*` to required dimension labels**

For `effectiveDimensions` loop:

```html
<!-- REPLACE: -->
<label class="text-xs text-gray-500 w-16 shrink-0">{{ dim.name }}</label>

<!-- WITH: -->
<label class="text-xs text-gray-500 w-16 shrink-0">
  {{ dim.name }}<span v-if="dim.required" class="text-red-500"> *</span>
</label>
```

For `monthlyDimension`:

```html
<!-- REPLACE: -->
<label class="text-xs text-gray-500 w-16 shrink-0">{{ monthlyDimension.name }}</label>

<!-- WITH: -->
<label class="text-xs text-gray-500 w-16 shrink-0">
  {{ monthlyDimension.name }}<span v-if="monthlyDimension.required" class="text-red-500"> *</span>
</label>
```

- [ ] **Step 2: Add `* required` legend**

After the closing `</div>` of `flex flex-col gap-2`:

```html
<div class="text-[10px] text-gray-400 mt-1">
  <span class="text-red-500">*</span> required
</div>
```

- [ ] **Step 3: Run type check**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook && npx vue-tsc --noEmit
```

Expected: Clean.

- [ ] **Step 4: Commit**

```bash
git add src/components/DimensionPanel.vue
git commit -m "feat: add required * indicator to DimensionPanel labels"
```

---

### Task 8: Final verification — full stack

**Role:** Any agent. Integration check across all changes.

- [ ] **Step 1: Run full Rust test suite**

```bash
cd src-tauri && cargo test
```

Expected: ALL tests PASS.

- [ ] **Step 2: Run full frontend type check + tests**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook && npx vue-tsc --noEmit && npx vitest run
```

Expected: Type check PASS, existing tests PASS.

- [ ] **Step 3: Launch app and smoke test**

```bash
pnpm tauri dev
```

Verify:
1. Existing config without `required` still loads (backward compatibility)
2. Add `required: true` to a dimension in config.yaml → see `*` in DimensionPanel
3. Create entry without required dim → red chips appear
4. Click red chip → menu opens at value selection → select → submit works
5. Type @ → select values → menu loops until all required filled → Enter confirms
6. Edit an existing entry and clear a required dim → backend error returned

- [ ] **Step 4: Commit any final fixes**

```bash
git add -A && git commit -m "chore: final verification fixes for required dimensions"
```
