# Required Dimensions — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

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

### Task 1: Rust Dimension model — add `required` field

**Files:**
- Modify: `src-tauri/src/models.rs:11-19`
- Modify: `src-tauri/src/config.rs:189-262` (tests only — add `required: false`)

- [ ] **Step 1: Add `required` field to Dimension struct**

In `src-tauri/src/models.rs`, change the Dimension struct:

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

- [ ] **Step 2: Update config.rs unit tests to add `required: false`**

Every `Dimension { ... }` literal in `config.rs` tests needs `required: false` added (since struct has a new field). There are 7 Dimension literals across 7 test functions. Example:

```rust
// Before:
Dimension { name: "Biz".into(), key: "biz".into(), source: "static".into(), values: Some(vec!["X".into()]) },
// After:
Dimension { name: "Biz".into(), key: "biz".into(), source: "static".into(), values: Some(vec!["X".into()]), required: false },
```

- [ ] **Step 3: Run Rust tests to confirm compilation and backward compatibility**

```bash
cd src-tauri && cargo test
```

Expected: All tests PASS. No compilation errors.

- [ ] **Step 4: Write a unit test for serde deserialization of `required`**

Add to `src-tauri/src/config.rs` `#[cfg(test)] mod tests`:

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

- [ ] **Step 5: Run tests to verify**

```bash
cd src-tauri && cargo test config::tests::test_dimension_required
```

Expected: Both new tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/config.rs
git commit -m "feat: add required field to Dimension model"
```

---

### Task 2: Backend — `validate_required_dimensions` function

**Files:**
- Modify: `src-tauri/src/commands.rs` (add function + unit tests)

- [ ] **Step 1: Write failing unit tests for `validate_required_dimensions`**

Add to `src-tauri/src/commands.rs` `#[cfg(test)] mod tests`:

```rust
use crate::models::{Config, Dimension};

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

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd src-tauri && cargo test commands::tests::test_validate_required
```

Expected: Compilation error — `validate_required_dimensions` not found.

- [ ] **Step 3: Implement `validate_required_dimensions`**

Add to `src-tauri/src/commands.rs`, after the `parse_duration` function and before `#[tauri::command] pub fn init`:

```rust
/// Validate that all required dimensions have values in the entry.
/// Returns Ok(()) or Err with a human-readable message listing the first missing required dimension.
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

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd src-tauri && cargo test commands::tests::test_validate_required
```

Expected: All 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: add validate_required_dimensions with unit tests"
```

---

### Task 3: Integrate validation into `append_entry` and `update_entry`

**Files:**
- Modify: `src-tauri/src/commands.rs:225-254` (append_entry, update_entry)
- Modify: `src-tauri/tests/entry_crud_integration.rs` (integration tests)

- [ ] **Step 1: Write failing integration test for `append_new_entry` with missing required dim**

In `src-tauri/tests/entry_crud_integration.rs`, add:

```rust
#[test]
fn test_append_entry_rejects_missing_required_dimension() {
    let suffix = "req_missing";
    setup(suffix);
    let root = test_root(suffix);

    // Write config with a required dimension
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

- [ ] **Step 2: Run integration test to verify it fails**

```bash
cd src-tauri && cargo test test_append_entry_rejects_missing_required_dimension
```

Expected: FAIL — entry is appended without validation.

- [ ] **Step 3: Wire validation into `append_entry` command**

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

Also modify `update_entry` — add validation when dimensions are being changed:

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

- [ ] **Step 4: Run the integration test to verify it passes**

```bash
cd src-tauri && cargo test test_append_entry_rejects_missing_required_dimension
```

Expected: PASS.

- [ ] **Step 5: Write integration test for valid append with required dimensions**

In `src-tauri/tests/entry_crud_integration.rs`:

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

- [ ] **Step 6: Write integration test for update_entry clearing a required dim**

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

- [ ] **Step 7: Run all integration tests**

```bash
cd src-tauri && cargo test --test entry_crud_integration
```

Expected: All integration tests PASS.

- [ ] **Step 8: Run full Rust test suite**

```bash
cd src-tauri && cargo test
```

Expected: ALL tests PASS.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/tests/entry_crud_integration.rs
git commit -m "feat: validate required dimensions in append_entry and update_entry"
```

---

### Task 4: TypeScript Dimension interface

**Files:**
- Modify: `src/types.ts:1-6`

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
cd /Users/boxcounter/Code/Boxcounter/logbook && npx vue-tsc --noEmit 2>&1 | head -30
```

The type system will flag any existing code that constructs Dimension objects without `required`. Fix any compilation errors by adding `required: false` or `required: d.required` as appropriate. (The Dimension objects come from the Rust backend via `get_entries` / `init`, so they're deserialized from JSON — no manual construction in frontend code. This should pass clean.)

Expected: Type check passes.

- [ ] **Step 3: Commit**

```bash
git add src/types.ts
git commit -m "feat: add required field to TypeScript Dimension interface"
```

---

### Task 5: EntryInput.vue — @ menu loop + missing required chips

**Files:**
- Modify: `src/components/EntryInput.vue`

This is the largest change. It covers:
- Computed properties for required dimension state
- Modified `confirmSelection()` for loop behavior
- Modified dim-phase Enter to close when all required filled
- New `openValMenuDirect()` for red chip click
- Red dashed chip display in template
- Menu footer state display

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

- [ ] **Step 2: Add `openValMenuDirect` function for red chip click**

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
}
```

- [ ] **Step 2b: Add `insertAtChar` helper**

After `openDimMenu()`, add a small helper that injects `@` at the current cursor position. Needed so `extractFilterFromInput` can still extract filter text when the menu loops back to dim phase:

```typescript
/// Insert a bare @ at cursor position, so the dim-phase filter can pick up.
function insertAtChar() {
  const cursorPos = inputEl.value?.selectionStart ?? input.value.length;
  input.value = input.value.slice(0, cursorPos) + "@" + input.value.slice(cursorPos);
}
```

- [ ] **Step 3: Modify `confirmSelection()` — loop back to dim list after value selection**

In `confirmSelection()` (lines ~141-153), change the val-phase branch:

```typescript
// REPLACE the val-phase block:
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

- [ ] **Step 4: Modify `confirmSelection()` — dim-phase Enter closes when all required filled**

In `confirmSelection()`, change the dim-phase block:

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

- [ ] **Step 5: Modify `selectByIndex()` — val-phase loop, dim-phase unchanged**

Number keys (1-9) in dim phase always open the value list (unchanged — user may want to select an optional dim or change a filled one). Only Enter is the "close" key. But val phase must ALSO loop (like `confirmSelection`):

In `selectByIndex()` (lines ~155-167), change only the val-phase branch:

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
      insertAtChar();     // re-insert @ so filter works
      openDimMenu();      // loop back to dimension list
    }
  }
```

- [ ] **Step 6: Add missing-required red dashed chips to template**

In the template, find the chips row div (around line ~454: `<div class="flex flex-wrap gap-1.5 mt-2 min-h-[24px] items-center">`). Add the missing-required chips inside this row, after the existing chip loop ends and before the italic placeholder `@ to set dimensions`:

```html
<!-- Missing required chips (red dashed) -->
<span
  v-for="m in missingRequired"
  :key="'missing-' + m.key"
  class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs cursor-pointer border border-dashed"
  :class="{
    'border-red-400 bg-red-50 text-red-700': true,
  }"
  @click="openValMenuDirect(m.key)"
>
  + {{ m.name }}
</span>
```

This goes inside the chips row `<div>`, after the existing `v-for="dim in dimensions"` loop's `</span>` and before the `<span v-if="Object.values(dimValues).every(v => !v)">`.

- [ ] **Step 7: Add menu footer showing required-remaining state**

In the template, inside the menu `<div ref="menuEl">`, after `<div v-if="getMenuItems().length === 0">No matches</div>`, add:

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

- [ ] **Step 8: Update the menu header to show `*` for required dims in dim list**

In the menu header, the subtitle already shows dimension name. No change needed for the header — the dim list items already show properly. But let's add required/optional badges to the dim items themselves.

In the template, find `getMenuItems()` section for `menuPhase === 'dim'`. After `<span class="flex-1">{{ item.label }}</span>`, add a required badge. The dim items in the menu are rendered via `getMenuItems()`, which returns `MenuItem` objects. The `MenuItem` interface needs a `required` field added:

```typescript
// In the MenuItem interface (around line 68):
interface MenuItem {
  label: string;
  sub?: string | null;
  key?: string;
  value?: string;
  required?: boolean;
}

// In getMenuItems(), add required to the dim phase return:
if (menuPhase.value === "dim") {
  return props.dimensions
    .filter(...)
    .map((d) => ({ label: d.name, sub: DIM_ALIASES.value[d.key] || d.key, key: d.key, required: d.required }));
}
```

Then in the template, in the dim-phase items, after `<span class="flex-1">{{ item.label }}</span>`, add:

```html
<span v-if="menuPhase === 'dim' && item.required && !dimValues[item.key || '']" class="text-[10px] text-red-400">required</span>
<span v-else-if="menuPhase === 'dim' && item.required && dimValues[item.key || '']" class="text-[10px] text-green-500">{{ dimValues[item.key || ''] }} ✓</span>
```

- [ ] **Step 9: Run type check**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook && npx vue-tsc --noEmit 2>&1 | head -30
```

Expected: Clean type check.

- [ ] **Step 10: Manual verification checklist**

Since Vue component behavior is hard to unit-test without component testing infrastructure, verify manually:

1. Type `@` → see dimension list with required badges
2. Select a required dim → see value list → pick value → menu returns to dim list (not closed)
3. Dim list now shows `Value ✓` for the filled dim
4. After filling all required dims → footer shows `All required ✓ · Enter to confirm`
5. Press Enter → menu closes, chips show all values
6. Type `@thing 30m` and click Log without setting required dims → red dashed chips appear
7. Click red chip `+ Business line` → value list opens for Business line
8. Pick value → if more required dims remaining, returns to dim list

- [ ] **Step 11: Commit**

```bash
git add src/components/EntryInput.vue
git commit -m "feat: @ menu loop + missing required chips in EntryInput"
```

---

### Task 6: DimensionPanel.vue — required `*` indicator

**Files:**
- Modify: `src/components/DimensionPanel.vue`

- [ ] **Step 1: Add `*` to required dimension labels**

In the template, find the `<label>` for `effectiveDimensions` (the static dimensions loop):

```html
<!-- REPLACE: -->
<label class="text-xs text-gray-500 w-16 shrink-0">{{ dim.name }}</label>

<!-- WITH: -->
<label class="text-xs text-gray-500 w-16 shrink-0">
  {{ dim.name }}<span v-if="dim.required" class="text-red-500"> *</span>
</label>
```

Also update the `monthlyDimension` label if present — though monthly dims are typically not required. Keep consistent:

```html
<!-- REPLACE: -->
<label class="text-xs text-gray-500 w-16 shrink-0">{{ monthlyDimension.name }}</label>

<!-- WITH: -->
<label class="text-xs text-gray-500 w-16 shrink-0">
  {{ monthlyDimension.name }}<span v-if="monthlyDimension.required" class="text-red-500"> *</span>
</label>
```

- [ ] **Step 2: Add `* required` legend at the bottom**

After the last select element (the `</div>` that closes `flex flex-col gap-2`), add:

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

### Task 7: Final verification — full stack

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
