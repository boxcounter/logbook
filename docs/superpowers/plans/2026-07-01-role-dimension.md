# Role 维度 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 entry 可以直接归属到 role（不经 goal），使 role spent 反映真实耗时。

**Architecture:** 后端新增 `Attribution` enum 和 `CommitmentProgressResult` 包装结构；`get_commitment_progress` 返回分段 goal/general spent + 未归属/mismatch 计数。前端 DimensionPopover 支持 role 维度 + 交叉过滤；CommitmentsPanel 分段进度条 + warning bar；EntryRow amber 标记未归属/mismatch entry。

**Tech Stack:** Rust (Tauri 2.x, yaml_serde) + Vue 3 + TypeScript + vitest

**Spec:** `docs/superpowers/specs/2026-07-01-role-dimension-design.md`

---

## File Structure

| 操作 | 文件 | 职责 |
|------|------|------|
| 修改 | `src-tauri/src/models.rs` | 新增 `Attribution` enum、`CommitmentProgressResult`；改 `Entry`（+attribution）、`CommitmentProgress`（spent_minutes → goal/general） |
| 修改 | `src-tauri/src/commands.rs` | 新增 `compute_attribution` helper；改 `get_commitment_progress`（新返回值 + 分段 spent）；改 `get_entries`/`append_entry`/`update_entry`（注入 attribution）；改 `set_commitments`（role 改名/删除同步 entry.role 维度） |
| 修改 | `src/types.ts` | 新增 `Attribution`、`CommitmentProgressResult`；改 `Entry`、`CommitmentProgress` |
| 修改 | `src/stores/useStore.ts` | 新增 `commitmentProgressResult` 字段 |
| 修改 | `src/components/CommitmentsPanel.vue` | 分段 bar、图例、warning bar |
| 修改 | `src/components/composite/EntryRow.vue` | amber 标记（基于 `entry.attribution`） |
| 修改 | `src/components/DimensionPopover.vue` | role 维度支持 + 交叉过滤 |
| 修改 | `src/__tests__/mocks/fixtures.ts` | 更新 mock 函数 |
| 修改 | `src-tauri/tests/` (集成测试) | 新测试覆盖 role 归属、分段 spent、mismatch 检测 |

---

### Task 1: Rust 数据模型 — Attribution + CommitmentProgressResult + Entry/CommitmentProgress 变更

**Files:**
- Modify: `src-tauri/src/models.rs`

- [ ] **Step 1: 在 models.rs 中添加 Attribution enum 和 CommitmentProgressResult struct**

```rust
// 在 CommitmentProgress 定义之后（约 line 60）添加：

/// Entry 归属状态，后端读 entry 时判定。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Attribution {
    /// 正常归属（有 role 维度，或通过 goal → role 映射）
    Ok,
    /// 无 role 且无 goal（或 goal 未声明）
    Unattributed,
    /// 有 role 有 goal，但 goal 不在该 role 下声明
    Mismatch,
}

/// get_commitment_progress 返回值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentProgressResult {
    pub roles: Vec<CommitmentProgress>,
    pub unattributed_count: u32,
    pub unattributed_total_minutes: u32,
    pub mismatch_count: u32,
}
```

- [ ] **Step 2: 修改 Entry struct — 添加 attribution 字段**

```rust
// models.rs，Entry struct（约 line 71-78），改为：
pub struct Entry {
    pub id: String,
    pub item: String,
    pub duration: u32,
    pub dimensions: BTreeMap<String, String>,
    #[serde(default)]
    pub attribution: Attribution,
}
```

`#[serde(default)]` 确保旧 YAML（无 attribution 字段）反序列化时不报错——默认值由 `Attribution::default()`（见 Step 3）提供，但旧数据读入后会在 commands 层被重新计算覆盖，因此这个 default 仅作为反序列化安全网。

- [ ] **Step 3: 为 Attribution 实现 Default trait**

```rust
// 在 Attribution 定义之后：
impl Default for Attribution {
    fn default() -> Self {
        Attribution::Ok
    }
}
```

注意：旧 YAML 文件中的 entry 没有 `attribution` 字段，反序列化时 serde 会用 `Default`。但每次读 entry 时 commands 层会调用 `compute_attribution` 重新计算并覆盖，所以这个 default 只在反序列化失败时起作用。

- [ ] **Step 4: 修改 CommitmentProgress struct**

```rust
// 替换现有的 CommitmentProgress（约 line 47-53）：
pub struct CommitmentProgress {
    pub role: String,
    pub allocation_minutes: u32,
    pub goal_spent_minutes: u32,
    pub general_spent_minutes: u32,
    pub goals: Vec<GoalProgress>,
}
```

移除 `spent_minutes`，新增 `goal_spent_minutes` 和 `general_spent_minutes`。

- [ ] **Step 5: 检查编译**

```bash
cd src-tauri && cargo check
```

Expected: 编译错误（commands.rs 和 tests 还在引用旧的 `spent_minutes`），这些在后续 task 中修正。

---

### Task 2: 前端 TypeScript 类型 — 同步 models 变更

**Files:**
- Modify: `src/types.ts`

- [ ] **Step 1: 添加 Attribution 类型和 CommitmentProgressResult**

```typescript
// src/types.ts，在 GoalProgress 之后（约 line 31）添加：

export type Attribution = "ok" | "unattributed" | "mismatch";

export interface CommitmentProgressResult {
  roles: CommitmentProgress[];
  unattributed_count: number;
  unattributed_total_minutes: number;
  mismatch_count: number;
}
```

- [ ] **Step 2: 修改 Entry interface**

```typescript
// 替换现有 Entry（约 line 52-57）：
export interface Entry {
  id: string;
  item: string;
  duration: number;
  dimensions: Record<string, string>;
  attribution: Attribution;
}
```

- [ ] **Step 3: 修改 CommitmentProgress interface**

```typescript
// 替换现有 CommitmentProgress（约 line 21-26）：
export interface CommitmentProgress {
  role: string;
  allocation_minutes: number;
  goal_spent_minutes: number;
  general_spent_minutes: number;
  goals: GoalProgress[];
}
```

移除 `spent_minutes`，替换为 `goal_spent_minutes` + `general_spent_minutes`。

- [ ] **Step 4: 类型检查**

```bash
npx vue-tsc --noEmit 2>&1 | head -30
```

Expected: 大量 type errors（组件还在引用旧的 `spent_minutes`），在后续 task 中修正。

---

### Task 3: Rust — compute_attribution helper + 单元测试

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: 编写 compute_attribution 函数的单元测试**

```rust
// 在 commands.rs 的 #[cfg(test)] mod tests 中添加（约 line 1330 附近）：

#[test]
fn test_compute_attribution_unattributed_no_dimensions() {
    use std::collections::HashMap;
    let dims = BTreeMap::new();
    let goal_to_role: HashMap<String, String> = HashMap::new();
    let role_to_goals: HashMap<String, Vec<String>> = HashMap::new();
    let result = compute_attribution(&dims, "goal", &goal_to_role, &role_to_goals);
    assert_eq!(result, Attribution::Unattributed);
}

#[test]
fn test_compute_attribution_ok_via_goal_fallback() {
    use std::collections::HashMap;
    let mut dims = BTreeMap::new();
    dims.insert("goal".to_string(), "Ship X".to_string());
    let mut goal_to_role: HashMap<String, String> = HashMap::new();
    goal_to_role.insert("Ship X".to_string(), "Dev".to_string());
    let role_to_goals: HashMap<String, Vec<String>> = HashMap::new();
    let result = compute_attribution(&dims, "goal", &goal_to_role, &role_to_goals);
    assert_eq!(result, Attribution::Ok);
}

#[test]
fn test_compute_attribution_unattributed_unknown_goal() {
    use std::collections::HashMap;
    let mut dims = BTreeMap::new();
    dims.insert("goal".to_string(), "Unknown".to_string());
    let goal_to_role: HashMap<String, String> = HashMap::new();
    let role_to_goals: HashMap<String, Vec<String>> = HashMap::new();
    let result = compute_attribution(&dims, "goal", &goal_to_role, &role_to_goals);
    assert_eq!(result, Attribution::Unattributed);
}

#[test]
fn test_compute_attribution_ok_role_only() {
    use std::collections::HashMap;
    let mut dims = BTreeMap::new();
    dims.insert("role".to_string(), "Dev".to_string());
    let goal_to_role: HashMap<String, String> = HashMap::new();
    let mut role_to_goals: HashMap<String, Vec<String>> = HashMap::new();
    role_to_goals.insert("Dev".to_string(), vec!["Ship X".to_string()]);
    let result = compute_attribution(&dims, "goal", &goal_to_role, &role_to_goals);
    assert_eq!(result, Attribution::Ok);
}

#[test]
fn test_compute_attribution_ok_role_and_matching_goal() {
    use std::collections::HashMap;
    let mut dims = BTreeMap::new();
    dims.insert("role".to_string(), "Dev".to_string());
    dims.insert("goal".to_string(), "Ship X".to_string());
    let goal_to_role: HashMap<String, String> = HashMap::new();
    let mut role_to_goals: HashMap<String, Vec<String>> = HashMap::new();
    role_to_goals.insert("Dev".to_string(), vec!["Ship X".to_string()]);
    let result = compute_attribution(&dims, "goal", &goal_to_role, &role_to_goals);
    assert_eq!(result, Attribution::Ok);
}

#[test]
fn test_compute_attribution_mismatch() {
    use std::collections::HashMap;
    let mut dims = BTreeMap::new();
    dims.insert("role".to_string(), "Dev".to_string());
    dims.insert("goal".to_string(), "Design review".to_string());
    let goal_to_role: HashMap<String, String> = HashMap::new();
    let mut role_to_goals: HashMap<String, Vec<String>> = HashMap::new();
    role_to_goals.insert("Dev".to_string(), vec!["Ship X".to_string()]);
    // Design review is NOT in Dev's goals → mismatch
    let result = compute_attribution(&dims, "goal", &goal_to_role, &role_to_goals);
    assert_eq!(result, Attribution::Mismatch);
}

#[test]
fn test_compute_attribution_unattributed_unknown_role() {
    use std::collections::HashMap;
    let mut dims = BTreeMap::new();
    dims.insert("role".to_string(), "Ghost".to_string());
    dims.insert("goal".to_string(), "Ship X".to_string());
    let goal_to_role: HashMap<String, String> = HashMap::new();
    let role_to_goals: HashMap<String, Vec<String>> = HashMap::new();
    // Ghost role not in commitments → unattributed
    let result = compute_attribution(&dims, "goal", &goal_to_role, &role_to_goals);
    assert_eq!(result, Attribution::Unattributed);
}

#[test]
fn test_compute_attribution_dynamic_goal_key() {
    use std::collections::HashMap;
    let mut dims = BTreeMap::new();
    dims.insert("objective".to_string(), "Launch".to_string());
    let mut goal_to_role: HashMap<String, String> = HashMap::new();
    goal_to_role.insert("Launch".to_string(), "PM".to_string());
    let role_to_goals: HashMap<String, Vec<String>> = HashMap::new();
    // goal_key is "objective" (non-default)
    let result = compute_attribution(&dims, "objective", &goal_to_role, &role_to_goals);
    assert_eq!(result, Attribution::Ok);
}
```

- [ ] **Step 2: 运行测试，确认失败**

```bash
cd src-tauri && cargo test test_compute_attribution 2>&1
```

Expected: FAIL — `compute_attribution` 尚未定义。

- [ ] **Step 3: 实现 compute_attribution 函数**

```rust
// 在 commands.rs 中，monthly_dim_key 函数之后、get_commitment_progress 之前（约 line 631）添加：

/// 根据 entry 的 dimensions 和当月 commitments 判定归属状态。
///
/// `goal_key` 是月度 goal 维度的 key（通过 monthly_dim_key 解析）。
/// `goal_to_role` 是 goal 名 → role 名的映射。
/// `role_to_goals` 是 role 名 → goal 名列表的映射（用于 mismatch 检测）。
fn compute_attribution(
    dimensions: &BTreeMap<String, String>,
    goal_key: &str,
    goal_to_role: &std::collections::HashMap<String, String>,
    role_to_goals: &std::collections::HashMap<String, Vec<String>>,
) -> Attribution {
    let role = dimensions.get("role");
    let goal = dimensions.get(goal_key);

    match (role, goal) {
        (None, None) => Attribution::Unattributed,
        (None, Some(g)) => {
            if goal_to_role.contains_key(g.as_str()) {
                Attribution::Ok
            } else {
                Attribution::Unattributed
            }
        }
        (Some(_), None) => Attribution::Ok,
        (Some(r), Some(g)) => {
            if let Some(goals) = role_to_goals.get(r.as_str()) {
                if goals.contains(g) {
                    Attribution::Ok
                } else {
                    Attribution::Mismatch
                }
            } else {
                // role 不在 commitments 中
                Attribution::Unattributed
            }
        }
    }
}
```

注意：需要 `use crate::models::Attribution;` 或在函数中使用完整路径。

- [ ] **Step 4: 运行测试，确认通过**

```bash
cd src-tauri && cargo test test_compute_attribution 2>&1
```

Expected: 8 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/commands.rs
git commit -m "feat: add Attribution enum, CommitmentProgressResult, and compute_attribution helper"
```

- [ ] **Step 6: 等待核对检查点**

---

### Task 4: Rust — 修改 get_entries / append_entry / update_entry，注入 attribution

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: 添加辅助函数：构建 goal→role 和 role→goals 映射 + 读 commitments 并计算 attribution**

```rust
// 在 compute_attribution 之后添加：

/// 从 commitments 构建 goal→role 和 role→goals 映射
fn build_commitment_maps(
    commitments: &[Commitment],
) -> (
    std::collections::HashMap<String, String>,
    std::collections::HashMap<String, Vec<String>>,
) {
    let mut goal_to_role: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut role_to_goals: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for c in commitments {
        let goals = c.goals.clone();
        for g in &goals {
            goal_to_role.insert(g.clone(), c.role.clone());
        }
        role_to_goals.insert(c.role.clone(), goals);
    }
    (goal_to_role, role_to_goals)
}

/// 为 DayFile 中所有 entry 计算 attribution
fn annotate_day_file(
    day_file: &mut DayFile,
    goal_key: &str,
    goal_to_role: &std::collections::HashMap<String, String>,
    role_to_goals: &std::collections::HashMap<String, Vec<String>>,
) {
    for entry in &mut day_file.entries {
        entry.attribution = compute_attribution(&entry.dimensions, goal_key, goal_to_role, role_to_goals);
    }
}
```

- [ ] **Step 2: 修改 get_entries — 注入 attribution**

```rust
// 修改 get_entries（约 line 406-415）：
#[tauri::command]
pub fn get_entries(root_path: String, date: String) -> Result<DayFile, String> {
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    let mut day_file = crate::files::read_day_file(root, &date)?;

    // 注入 attribution
    let year = date[..4].parse::<i32>().unwrap_or(0);
    let month = date[5..7].parse::<u32>().unwrap_or(0);
    let commitments = crate::files::read_commitments_file(root, year, month).unwrap_or_default();
    let goal_key = monthly_dim_key(root, year, month);
    let (goal_to_role, role_to_goals) = build_commitment_maps(&commitments);
    annotate_day_file(&mut day_file, &goal_key, &goal_to_role, &role_to_goals);

    Ok(day_file)
}
```

- [ ] **Step 3: 修改 append_entry — 返回的 Entry 带 attribution**

```rust
// 修改 append_entry（约 line 418-458），在 entry 构造后（约 line 440，Entry { ... } 构造之后）、日志记录之前添加：

    let mut entry = Entry {
        id: uuid::Uuid::new_v4().to_string(),
        item,
        duration: minutes,
        dimensions: dims.clone(),
        attribution: Attribution::default(),
    };

    // 注入 attribution
    {
        let commitments = crate::files::read_commitments_file(root, year, month).unwrap_or_default();
        let goal_key = monthly_dim_key(root, year, month);
        let (goal_to_role, role_to_goals) = build_commitment_maps(&commitments);
        entry.attribution = compute_attribution(&entry.dimensions, &goal_key, &goal_to_role, &role_to_goals);
    }
```

注意：`append_entry` 目前返回 `Entry`（不是 `DayFile`）。不需要改为 `DayFile`——这是前端 undo toast 使用的对象，attribution 自带即可。

- [ ] **Step 4: 修改 update_entry — 返回的 DayFile 带 attribution**

```rust
// 修改 update_entry（约 line 461-516），在 day_file 构造完成后（约 line 515，map_err 之前）添加：

    let mut day_file = /* 现有的 day_file 构造 */;

    // 注入 attribution
    {
        let commitments = crate::files::read_commitments_file(root, year, month).unwrap_or_default();
        let goal_key = monthly_dim_key(root, year, month);
        let (goal_to_role, role_to_goals) = build_commitment_maps(&commitments);
        annotate_day_file(&mut day_file, &goal_key, &goal_to_role, &role_to_goals);
    }

    Ok(day_file)
```

- [ ] **Step 5: 同样修改 delete_entry — 返回的 DayFile 带 attribution**

```rust
// 修改 delete_entry（约 line 519-549），同样在 day_file 读取后注入 attribution：
```

- [ ] **Step 6: 编译检查**

```bash
cd src-tauri && cargo check 2>&1
```

Expected: 仍有错误（`get_commitment_progress` / `set_commitments` / tests 引用旧的 `spent_minutes`）。

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: inject attribution into entries returned by get_entries/append_entry/update_entry/delete_entry"
```

- [ ] **Step 8: 修改 load_root_state（init 路径）— 注入 today DayFile 的 attribution**

在 `load_root_state` 函数中，`read_day_file_safe` 读取 today DayFile 之后（约 line 282），返回 `InitResult::Ready` 之前（约 line 300），添加：

```rust
    // Inject attribution into today's entries
    {
        let commitments = crate::files::read_commitments_file(root, now.year(), now.month()).unwrap_or_default();
        let goal_key = monthly_dim_key(root, now.year(), now.month());
        let (goal_to_role, role_to_goals) = build_commitment_maps(&commitments);
        annotate_day_file(&mut today_file, &goal_key, &goal_to_role, &role_to_goals);
    }
```

注意：`build_commitment_maps` 和 `annotate_day_file` 已在 Step 1 中定义。

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "fix: inject attribution into today DayFile in init path"
```

- [ ] **Step 10: 等待核对检查点**

---

### Task 5: Rust — 重写 get_commitment_progress（新返回值 + 分段 spent）

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: 编写 get_commitment_progress 新行为测试**

```rust
// 在 commands.rs 的 #[cfg(test)] mod tests 中添加：

#[test]
fn test_get_commitment_progress_with_role_dimension() {
    use crate::models::{Commitment, CommitmentProgress, CommitmentProgressResult, GoalProgress};
    use std::collections::HashMap;

    let tmp = std::env::temp_dir().join("logbook-test-role-progress");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    // Setup: template.yaml
    let template = r#"dimensions:
  - name: Goal
    key: goal
    source: monthly
"#;
    std::fs::write(tmp.join("dimensions.template.yaml"), template).unwrap();

    // Setup: commitments.yaml
    let commitments_yaml = r#"- role: Dev
  allocation: 20
  goals:
    - Ship X
- role: PM
  allocation: 10
  goals:
    - Roadmap
"#;
    let month_dir = tmp.join("2026").join("07");
    std::fs::create_dir_all(&month_dir).unwrap();
    std::fs::write(month_dir.join("commitments.yaml"), commitments_yaml).unwrap();
    std::fs::write(month_dir.join("dimensions.yaml"), "dimensions: []\n").unwrap();

    // Day 1: entry with role=Dev, goal=Ship X → Ok, goal segment
    let day1 = r#"---
entries:
  - id: e1
    item: Code feature
    duration: 120
    dimensions:
      role: Dev
      goal: Ship X
  - id: e2
    item: Standup
    duration: 30
    dimensions:
      role: Dev
  - id: e3
    item: Email
    duration: 15
    dimensions: {}
---"#;
    std::fs::write(month_dir.join("2026-07-01.md"), day1).unwrap();

    // Day 2: entry via goal fallback (no role dim)
    let day2 = r#"---
entries:
  - id: e4
    item: Roadmap planning
    duration: 60
    dimensions:
      goal: Roadmap
  - id: e5
    item: Mismatch case
    duration: 45
    dimensions:
      role: Dev
      goal: Roadmap
---"#;
    std::fs::write(month_dir.join("2026-07-02.md"), day2).unwrap();

    let result = get_commitment_progress(
        tmp.to_string_lossy().to_string(),
        2026,
        7,
    ).unwrap();

    // Dev role
    let dev = result.roles.iter().find(|r| r.role == "Dev").unwrap();
    // e1 (120m goal=Ship X) + e2 (30m general) + e5 (45m goal=Roadmap but Dev role → mismatch → general)
    assert_eq!(dev.goal_spent_minutes, 120);  // only e1
    assert_eq!(dev.general_spent_minutes, 75);  // e2 + e5
    assert_eq!(dev.allocation_minutes, 1200);

    // PM role
    let pm = result.roles.iter().find(|r| r.role == "PM").unwrap();
    // e4 (60m goal=Roadmap, fallback to PM)
    assert_eq!(pm.goal_spent_minutes, 60);
    assert_eq!(pm.general_spent_minutes, 0);

    // Unattributed
    assert_eq!(result.unattributed_count, 1);  // e3
    assert_eq!(result.unattributed_total_minutes, 15);

    // Mismatch
    assert_eq!(result.mismatch_count, 1);  // e5

    let _ = std::fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 2: 运行测试，确认失败**

```bash
cd src-tauri && cargo test test_get_commitment_progress_with_role_dimension 2>&1
```

Expected: FAIL（`get_commitment_progress` 尚未重写，返回类型不匹配）。

- [ ] **Step 3: 重写 get_commitment_progress**

核心逻辑变更（替换现有 line 633-746）：

```rust
#[tauri::command]
pub fn get_commitment_progress(
    root_path: String,
    year: i32,
    month: u32,
) -> Result<CommitmentProgressResult, String> {
    use crate::models::{CommitmentProgress, GoalProgress};
    use std::collections::HashMap;

    let root = std::path::Path::new(&root_path);

    // 1. Read commitments.yaml
    let commitments = crate::files::read_commitments_file(root, year, month).unwrap_or_else(|e| {
        error_log::log_error(
            "get_commitment_progress",
            &format!("Failed to read commitments.yaml for {}-{:02}: {:?}", year, month, e),
        );
        vec![]
    });

    // 2. Build maps
    let (goal_to_role, role_to_goals) = build_commitment_maps(&commitments);

    // 3. Initialize result structures
    let mut role_goal_spent: HashMap<String, u32> = HashMap::new();   // role → goal segment
    let mut role_general_spent: HashMap<String, u32> = HashMap::new(); // role → general segment
    let mut goal_spent: HashMap<String, u32> = HashMap::new();
    let mut unattributed_count: u32 = 0;
    let mut unattributed_total: u32 = 0;
    let mut mismatch_count: u32 = 0;

    for c in &commitments {
        role_goal_spent.entry(c.role.clone()).or_insert(0);
        role_general_spent.entry(c.role.clone()).or_insert(0);
        for g in &c.goals {
            goal_spent.entry(g.clone()).or_insert(0);
        }
    }

    // 4. Scan day files
    let goal_key = monthly_dim_key(root, year, month);
    let month_dir = root.join(year.to_string()).join(format!("{:02}", month));

    if month_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&month_dir) {
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        error_log::log_error("get_commitment_progress", &format!("read_dir error: {:?}", e));
                        continue;
                    }
                };
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if file_name == "_monthly.md" || !file_name.ends_with(".md") {
                    continue;
                }
                match crate::files::read_day_file(root, file_name.trim_end_matches(".md")) {
                    Ok(day_file) => {
                        for e in &day_file.entries {
                            let attr = compute_attribution(&e.dimensions, &goal_key, &goal_to_role, &role_to_goals);

                            match attr {
                                Attribution::Ok => {
                                    // Determine which role and whether it's goal or general
                                    if let Some(role) = e.dimensions.get("role") {
                                        // Role dimension → direct attribution
                                        if let Some(goal_val) = e.dimensions.get(&goal_key) {
                                            // Has goal — check if it's a declared goal for this role
                                            if let Some(goals) = role_to_goals.get(role) {
                                                if goals.contains(goal_val) {
                                                    // Matching goal → goal segment
                                                    *role_goal_spent.entry(role.clone()).or_insert(0) += e.duration;
                                                    *goal_spent.entry(goal_val.clone()).or_insert(0) += e.duration;
                                                } else {
                                                    // Goal not in this role → general segment
                                                    *role_general_spent.entry(role.clone()).or_insert(0) += e.duration;
                                                }
                                            } else {
                                                *role_general_spent.entry(role.clone()).or_insert(0) += e.duration;
                                            }
                                        } else {
                                            // No goal → general segment
                                            *role_general_spent.entry(role.clone()).or_insert(0) += e.duration;
                                        }
                                    } else if let Some(goal_val) = e.dimensions.get(&goal_key) {
                                        // No role, but has goal → fallback to goal's role
                                        if let Some(role) = goal_to_role.get(goal_val) {
                                            *role_goal_spent.entry(role.clone()).or_insert(0) += e.duration;
                                            *goal_spent.entry(goal_val.clone()).or_insert(0) += e.duration;
                                        }
                                    }
                                }
                                Attribution::Unattributed => {
                                    unattributed_count += 1;
                                    unattributed_total += e.duration;
                                }
                                Attribution::Mismatch => {
                                    mismatch_count += 1;
                                    // Still count toward the role's general segment
                                    if let Some(role) = e.dimensions.get("role") {
                                        *role_general_spent.entry(role.clone()).or_insert(0) += e.duration;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error_log::log_error("get_commitment_progress", &format!("read_day_file error: {}", e));
                    }
                }
            }
        }
    }

    // 5. Build result
    let mut roles: Vec<CommitmentProgress> = Vec::new();
    for c in &commitments {
        let goals: Vec<GoalProgress> = c
            .goals
            .iter()
            .map(|g| GoalProgress {
                name: g.clone(),
                spent_minutes: *goal_spent.get(g).unwrap_or(&0),
            })
            .collect();
        roles.push(CommitmentProgress {
            role: c.role.clone(),
            allocation_minutes: c.allocation * 60,
            goal_spent_minutes: *role_goal_spent.get(&c.role).unwrap_or(&0),
            general_spent_minutes: *role_general_spent.get(&c.role).unwrap_or(&0),
            goals,
        });
    }

    Ok(CommitmentProgressResult {
        roles,
        unattributed_count,
        unattributed_total_minutes: unattributed_total,
        mismatch_count,
    })
}
```

- [ ] **Step 4: 运行新测试，确认通过**

```bash
cd src-tauri && cargo test test_get_commitment_progress_with_role_dimension 2>&1
```

Expected: PASS。

- [ ] **Step 5: 更新旧的 get_commitment_progress 测试**

```bash
cd src-tauri && cargo test test_get_commitment_progress 2>&1
```

Expected: 部分测试 FAIL（返回类型变了，引用 `spent_minutes` 的 assert 需要更新）。逐个修正旧测试：
- `spent_minutes` → `goal_spent_minutes`（旧测试没有 role 维度，所有 time 都走 goal fallback）
- 结果从 `Vec<CommitmentProgress>` 变为 `result.roles`

- [ ] **Step 6: 修正所有旧测试中的断言，确认全部通过**

```bash
cd src-tauri && cargo test 2>&1
```

Expected: 所有 Rust 测试 PASS。

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/models.rs
git commit -m "feat: rewrite get_commitment_progress with role attribution and segmented spent"
```

- [ ] **Step 8: 等待核对检查点**

---

### Task 6: Rust — 修改 set_commitments（role 改名/删除时同步 entry.role 维度）

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: 在 set_commitments 中添加 role 改名逻辑**

现有 `set_commitments`（约 line 749-801）已经有 goal 改名逻辑（`detect_goal_changes` + 扫描 day files 替换 goal 维度）。需要添加对 role 改名的支持。

新增辅助函数：

```rust
/// 检测 role 改名：新旧 commitments 之间，role 名变了但 goals 相同。
/// 返回 (old_name, new_name) 列表。
fn detect_role_changes(old: &[Commitment], new: &[Commitment]) -> Vec<(String, String)> {
    let mut changes = Vec::new();
    // 按 goals 集合匹配：如果 old 中某个 role 的 goals 集合和 new 中某个 role 的 goals 集合完全一致但 role 名不同，则为改名
    for o in old {
        let old_goals: std::collections::BTreeSet<&String> = o.goals.iter().collect();
        if let Some(n) = new.iter().find(|n| {
            let new_goals: std::collections::BTreeSet<&String> = n.goals.iter().collect();
            old_goals == new_goals && o.role != n.role
        }) {
            changes.push((o.role.clone(), n.role.clone()));
        }
    }
    changes
}
```

在 `set_commitments` 中，`detect_goal_changes` 之后（约 line 777）、应用 goal 改名之前（约 line 790），添加：

```rust
    // Detect role renames
    let role_changes = detect_role_changes(&old_commitments, &new_commitments);

    // Apply role renames to all day file entries' dimensions.role
    for (old_name, new_name) in &role_changes {
        if let Ok(entries) = std::fs::read_dir(&month_dir) {
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if file_name == "_monthly.md" || !file_name.ends_with(".md") {
                    continue;
                }
                if let Ok(mut day_file) = crate::files::read_day_file(root, file_name.trim_end_matches(".md")) {
                    let mut changed = false;
                    for e in &mut day_file.entries {
                        if e.dimensions.get("role").map(|r| r == old_name).unwrap_or(false) {
                            e.dimensions.insert("role".to_string(), new_name.to_string());
                            changed = true;
                        }
                    }
                    if changed {
                        // Write back via atomic write (use files helper or inline)
                        let _ = crate::files::write_day_file(root, file_name.trim_end_matches(".md"), &day_file);
                    }
                }
            }
        }
    }
```

- [ ] **Step 2: 添加 role 删除逻辑**

在 role 改名之后、写入 commitments.yaml 之前：

```rust
    // Detect deleted roles: roles in old but not in new
    let old_role_names: std::collections::BTreeSet<&String> = old_commitments.iter().map(|c| &c.role).collect();
    let new_role_names: std::collections::BTreeSet<&String> = new_commitments.iter().map(|c| &c.role).collect();
    let deleted_roles: Vec<&String> = old_role_names.difference(&new_role_names).cloned().collect();

    // Clear role dimension on entries for deleted roles
    for role_name in &deleted_roles {
        if let Ok(entries) = std::fs::read_dir(&month_dir) {
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if file_name == "_monthly.md" || !file_name.ends_with(".md") {
                    continue;
                }
                if let Ok(mut day_file) = crate::files::read_day_file(root, file_name.trim_end_matches(".md")) {
                    let mut changed = false;
                    for e in &mut day_file.entries {
                        if e.dimensions.get("role").map(|r| r == *role_name).unwrap_or(false) {
                            e.dimensions.remove("role");
                            changed = true;
                        }
                    }
                    if changed {
                        let _ = crate::files::write_day_file(root, file_name.trim_end_matches(".md"), &day_file);
                    }
                }
            }
        }
    }
```

- [ ] **Step 3: 编写 role 改名集成测试**

在 `src-tauri/tests/commitment_editor_integration.rs` 中添加测试，验证 role 改名后 entry 的 `dimensions.role` 被更新。

- [ ] **Step 4: 运行全部测试**

```bash
cd src-tauri && cargo test 2>&1
```

Expected: 全部 PASS。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/tests/
git commit -m "feat: sync entry dimensions.role on role rename/delete in set_commitments"
```

- [ ] **Step 6: 等待核对检查点**

---

### Task 7: 前端 — 更新 store 和 applyInitResult + 修复引用 spent_minutes 的代码

**Files:**
- Modify: `src/stores/useStore.ts`
- Modify: `src/utils/applyInitResult.ts`
- Modify: `src/components/CommitmentsPanel.vue`
- Modify: `src/components/composite/CommitmentsModal.vue`
- Modify: `src/components/composite/RoleCard.vue`
- Modify: `src/utils/commitments.ts`

- [ ] **Step 1: 更新 useStore.ts — 新增 commitmentProgressResult 字段**

```typescript
// src/stores/useStore.ts, AppStore interface（约 line 10-23）：
export interface AppStore {
  status: AppStatus;
  rootPath: string;
  dimensions: Dimension[];
  fromTemplate: boolean;
  configErrors: ConfigErrorDetail[];
  configCategory: RecoveryCategory | null;
  today: DayFile | null;
  commitments: Commitment[];
  commitmentProgress: CommitmentProgress[];           // ← 保留兼容（从 result.roles 提取）
  commitmentProgressResult: CommitmentProgressResult | null; // ← 新增
  currentDate: string;
  monthEntries: Record<string, Entry[]>;
  availableMonths: AvailableMonth[] | null;
}
```

```typescript
// createStore（约 line 27-45），初始值新增：
commitmentProgressResult: null,
```

- [ ] **Step 2: 更新所有引用 CommitmentProgress.spent_minutes 的前端代码**

**CommitmentsPanel.vue** — `pct` 函数和显示：

```typescript
// line 22-25，改为：
function pct(spent: number, alloc: number): string {
  if (alloc === 0) return "0%";
  return Math.min(100, Math.round((spent / alloc) * 100)) + "%";
}
// spent 传入 goal + general
```

```html
<!-- line 57，显示改为 goal + general 总和 -->
<span class="mono">{{ formatDurationCompact(s.goal_spent_minutes + s.general_spent_minutes) }}</span>...
```

```html
<!-- line 64，进度条总宽度用 goal + general -->
:style="{ width: pct(s.goal_spent_minutes + s.general_spent_minutes, s.allocation_minutes), ...
```

**CommitmentsModal.vue** — `loggedTotal` computed（line 72）：

```typescript
const loggedTotal = computed(() =>
  props.progress.reduce((s, p) => s + p.goal_spent_minutes + p.general_spent_minutes, 0)
);
```

**RoleCard.vue** — `roleSpent` computed（line 66-68）：

```typescript
const roleSpent = computed(() => {
  const p = props.progress.find(p => p.role === props.role.origRole);
  return (p?.goal_spent_minutes ?? 0) + (p?.general_spent_minutes ?? 0);
});
```

**`src/utils/commitments.ts`** — `goalLoggedMinutes`（line 1-11）：
```typescript
// spent_minutes → goal_spent_minutes（此处只引用 goal level，不引用 role level）
export function goalLoggedMinutes(progress: CommitmentProgress[], origName: string | null): number {
  if (!origName) return 0;
  for (const p of progress) {
    const g = p.goals.find(x => x.name === origName);
    if (g) return g.spent_minutes;  // unchanged — GoalProgress.spent_minutes 不变
  }
  return 0;
}
```

`goal_spent_minutes` 只在 `CommitmentProgress` (role level) 上改名，`GoalProgress.spent_minutes` 保持不变。

- [ ] **Step 3: 更新 applyInitResult.ts — 处理新的数据形状**

```typescript
// src/utils/applyInitResult.ts，Ready 分支（约 line 17-24）：
case "Ready": {
  store.rootPath = data.root_path;
  store.dimensions = data.dimensions;
  store.fromTemplate = data.from_template;
  store.today = data.today;
  store.configCategory = null;
  store.status = "ready";
  break;
}
```

`Ready` 不直接包含 `commitmentProgressResult`——它由 `get_commitment_progress` 异步获取。此文件暂不需要修改。

- [ ] **Step 4: 确认类型检查通过（仅针对当前修改的文件）**

```bash
npx vue-tsc --noEmit 2>&1 | grep -E "^src/(stores|utils|components/(CommitmentsPanel|composite/(CommitmentsModal|RoleCard)))" | head -20
```

Expected: 无错误。

- [ ] **Step 5: Commit**

```bash
git add src/stores/ src/utils/ src/components/
git commit -m "refactor: update spent_minutes references to goal_spent_minutes + general_spent_minutes"
```

- [ ] **Step 6: 等待核对检查点**

---

### Task 8: 前端 — CommitmentsPanel 分段进度条 + 图例 + warning bar

**Files:**
- Modify: `src/components/CommitmentsPanel.vue`

- [ ] **Step 1: 在 CommitmentsPanel 中实现分段 bar**

替换现有单条进度条（line 60-66）为分段进度条：

```html
<!-- 在 bar-track 内放两个 div：goal 段 + general 段 -->
<div class="h-[4px] bg-[var(--color-divider)] rounded-[var(--radius-sm)] overflow-hidden flex mt-xs">
  <div
    v-if="s.goal_spent_minutes > 0"
    data-test="progress-goal"
    class="h-full transition-all"
    :style="{
      width: pct(s.goal_spent_minutes, s.allocation_minutes),
      background: 'linear-gradient(90deg, var(--color-brand-gradient-from), var(--color-brand-gradient-to))'
    }"
  />
  <div
    v-if="s.general_spent_minutes > 0"
    data-test="progress-general"
    class="h-full transition-all"
    :style="{
      width: pct(s.general_spent_minutes, s.allocation_minutes),
      background: 'linear-gradient(90deg, #c4b5fd, #ddd6fe)'
    }"
  />
</div>
```

- [ ] **Step 2: 添加图例**

在 role 列表之后、warning bar 之前：

```html
<div
  v-if="hasCommitments"
  class="flex gap-lg text-micro text-[var(--color-text-secondary)] pt-md mt-sm border-t border-[var(--color-divider)]"
>
  <span class="flex items-center gap-xs">
    <span class="w-[10px] h-[10px] rounded-[2px] flex-shrink-0" style="background: linear-gradient(90deg, var(--color-brand-gradient-from), var(--color-brand-gradient-to))"></span>
    Goal
  </span>
  <span class="flex items-center gap-xs">
    <span class="w-[10px] h-[10px] rounded-[2px] flex-shrink-0" style="background: linear-gradient(90deg, #c4b5fd, #ddd6fe)"></span>
    General
  </span>
</div>
```

- [ ] **Step 3: 添加 warning bar**

在图例之后（或在 commitments 区域底部）：

```html
<div
  v-if="warningVisible"
  data-test="warning-bar"
  class="mt-md p-sm rounded-[var(--radius-form)] bg-[#fffbeb] border border-[#fde68a] text-secondary flex items-center justify-between text-[#92400e]"
>
  <span>
    ⚠ 未归属耗时：<strong>{{ formatDurationCompact(warningUnattributedMinutes) }}</strong>
    <template v-if="warningMismatchCount > 0">
      / role/goal 不匹配：{{ warningMismatchCount }} 条
    </template>
  </span>
  <span class="text-micro" style="color: #b45309">entry 缺少 role 或 goal 维度</span>
</div>
```

对应的 computed：

```typescript
const progressResult = computed(() => store.commitmentProgressResult);

const warningVisible = computed(() => {
  if (!progressResult.value) return false;
  return progressResult.value.unattributed_count > 0 || progressResult.value.mismatch_count > 0;
});

const warningUnattributedMinutes = computed(() =>
  progressResult.value?.unattributed_total_minutes ?? 0
);

const warningMismatchCount = computed(() =>
  progressResult.value?.mismatch_count ?? 0
);
```

注意：需要在 `useStore` 中引入 store（如果尚未引入）。

- [ ] **Step 4: 编写 CommitmentsPanel 测试**

在 `src/__tests__/` 中创建或更新测试，验证：
- 进度条有两段（data-test="progress-goal" 和 data-test="progress-general"）
- warning bar 在有未归属时显示（data-test="warning-bar"）
- warning bar 在无问题时隐藏

- [ ] **Step 5: 运行前端测试**

```bash
npx vitest run src/__tests__/ 2>&1 | tail -20
```

Expected: 测试通过（新测试）或有可接受的 failure（旧测试需要更新 mock 数据）。

- [ ] **Step 6: Commit**

```bash
git add src/components/CommitmentsPanel.vue
git commit -m "feat: add segmented progress bar, legend, and warning bar to CommitmentsPanel"
```

- [ ] **Step 7: 等待核对检查点**

---

### Task 9: 前端 — EntryRow amber 标记

**Files:**
- Modify: `src/components/composite/EntryRow.vue`

- [ ] **Step 1: 添加 amber 标记逻辑**

```typescript
// 在 EntryRow.vue script setup 中添加计算属性：
const isProblemEntry = computed(() =>
  props.entry.attribution === "unattributed" || props.entry.attribution === "mismatch"
);
```

- [ ] **Step 2: 添加行首 amber 圆点和样式**

在 template 中，duration 之前添加 amber 圆点（可见条件：`isProblemEntry`）：

```html
<span
  v-if="isProblemEntry"
  class="flex-shrink-0 text-[#d97706] mr-xs"
  style="font-size: 14px; width: 16px; text-align: center;"
  title="entry 未归属任何 role"
>●</span>
```

duration 数字着色：

```html
<span
  data-test="duration-display"
  data-edit-target="duration"
  class="mono text-secondary flex-shrink-0 ml-lg pt-2xs"
  :class="isProblemEntry ? 'text-[#d97706] font-medium' : 'text-[var(--color-text-primary)]'"
>
```

行背景微黄（修改顶层 div 的 class）：

```html
<div
  v-else
  data-test="entry-row"
  class="group flex justify-between items-start gap-sm px-md py-sm transition-colors"
  :class="[
    { 'just-added': justAdded },
    isProblemEntry
      ? 'bg-[#fffbeb] hover:bg-[#fef3c7] border-amber-200'
      : 'hover:bg-[var(--color-surface-muted)]',
    index > 0 ? 'border-t border-[var(--color-divider)]' : '',
    isProblemEntry && index > 0 ? '!border-[#fde68a]' : '',
  ]"
  @dblclick="onDblClick"
>
```

- [ ] **Step 3: 更新 EntryRow 测试 mock**

在 `src/__tests__/mocks/fixtures.ts` 中，`makeEntry` 函数添加 `attribution` 默认值：

```typescript
export function makeEntry(overrides: Partial<Entry> = {}): Entry {
  return {
    id: "e1",
    item: "Test item",
    duration: 30,
    dimensions: {},
    attribution: "ok",
    ...overrides,
  };
}
```

- [ ] **Step 4: 编写 EntryRow amber 标记测试**

测试 entry 的 `attribution` 为 `"unattributed"` / `"mismatch"` 时 amber 圆点和样式生效；为 `"ok"` 时不显示。

- [ ] **Step 5: 运行测试，确认通过**

```bash
npx vitest run src/__tests__/ 2>&1 | tail -20
```

- [ ] **Step 6: Commit**

```bash
git add src/components/composite/EntryRow.vue src/__tests__/mocks/fixtures.ts
git commit -m "feat: add amber indicator for unattributed and mismatch entries in EntryRow"
```

- [ ] **Step 7: 等待核对检查点**

---

### Task 10: 前端 — DimensionPopover 支持 role + 交叉过滤

**Files:**
- Modify: `src/components/DimensionPopover.vue`

- [ ] **Step 1: 在 dim 阶段添加 Role 选项**

在 dim 列表中添加 Role 维度项（仅当有 commitments 声明时）。`activeDimensions` computed（它过滤了 template 中的维度列表）需要额外处理：role 不在 template 的 dimensions 里，但需要出现在 popover 的 dim 列表中。

```typescript
// 新增 computed：
const hasCommitments = computed(() => props.commitments.length > 0);
```

```html
<!-- 在 dim 列表底部（现有非 deleted 维度之后）添加： -->
<button
  v-if="hasCommitments"
  data-test="dim-role"
  class="..."
  @click="selectDim('role')"
>
  <span>Role</span>
  <span v-if="dimValues.role" class="...">{{ dimValues.role }}</span>
  <span v-else class="...">optional</span>
</button>
```

- [ ] **Step 2: val 阶段展示 role 列表**

在 `selectDim` 选择 "role" 时，val 阶段展示 `props.commitments` 中声明的 role 名：

```typescript
// 修改 activeValues computed（约 line 41-53），添加 role 分支：
const activeValues = computed(() => {
  if (stage.value !== "val") return [];
  if (selectedDim.value === "role") {
    return props.commitments.map(c => c.role);
  }
  const dim = props.dimensions.find(d => d.key === selectedDim.value);
  if (!dim) return [];
  if (dim.source === "monthly") return goalOptions.value;
  return dim.values ?? [];
});
```

- [ ] **Step 3: 实现交叉过滤**

当 entry 已有 `role` 维度时，选 goal 的 val 列表只显示该 role 下的 goals。当 entry 已有 `goal` 维度时，选 role 的 val 列表只显示包含该 goal 的 role。

```typescript
// 修改 activeValues，加入交叉过滤逻辑：
const activeValues = computed(() => {
  if (stage.value !== "val") return [];
  if (selectedDim.value === "role") {
    let roles = props.commitments.map(c => c.role);
    // 交叉过滤：如果已选了 goal，只显示包含该 goal 的 role
    const goalKey = /* resolve monthly goal key — 从 props.dimensions 或默认 "goal" */;
    const existingGoal = props.dimValues[goalKey];
    if (existingGoal) {
      roles = roles.filter(r =>
        props.commitments.find(c => c.role === r)?.goals.includes(existingGoal)
      );
    }
    return roles;
  }
  if (selectedDim.value === goalKey.value) {
    let goals = goalOptions.value;
    // 交叉过滤：如果已选了 role，只显示该 role 下的 goals
    const existingRole = props.dimValues["role"];
    if (existingRole) {
      const roleCommitment = props.commitments.find(c => c.role === existingRole);
      if (roleCommitment) goals = roleCommitment.goals;
    }
    return goals;
  }
  // ... 其余 dim 的处理不变
});
```

其中 `goalKey` 需要解析当前月 goal 维度的 key（前端已有的逻辑是：从 `props.dimensions` 中找 `source === "monthly"` 的维度 key，默认 `"goal"`）。

```typescript
const goalKey = computed(() => {
  const monthly = props.dimensions.find(d => d.source === "monthly");
  return monthly?.key ?? "goal";
});
```

- [ ] **Step 4: 更新 DimensionPopover 测试**

更新 `src/__tests__/DimensionPopover.test.ts`，添加测试：
- dim 列表中出现 Role 选项（当有 commitments 时）
- 无 commitments 时 Role 不出现
- 选 Role 后 val 阶段显示 role 名列表
- 交叉过滤：已选 role=Dev 时 goal 列表只显示 Dev 的 goals
- 交叉过滤：已选 goal=Ship X 时 role 列表只显示包含该 goal 的 role

- [ ] **Step 5: 运行测试**

```bash
npx vitest run src/__tests__/DimensionPopover.test.ts 2>&1
```

Expected: 通过。

- [ ] **Step 6: Commit**

```bash
git add src/components/DimensionPopover.vue src/__tests__/DimensionPopover.test.ts
git commit -m "feat: add role dimension support and cross-filtering to DimensionPopover"
```

- [ ] **Step 7: 等待核对检查点**

---

### Task 11: 前端 — 连接数据流（App.vue / MonthView 调用链）

**Files:**
- Modify: `src/App.vue`（或调用 `get_commitment_progress` 的文件）
- Modify: `src/components/MonthView.vue`（如果直接调用）

- [ ] **Step 1: 找到调用 get_commitment_progress 的位置并更新**

```bash
grep -rn "get_commitment_progress" src/ --include="*.vue" --include="*.ts"
```

找到调用点后，更新为接收 `CommitmentProgressResult` 类型：

```typescript
// 调用方式改为：
const result = await invoke<CommitmentProgressResult>("get_commitment_progress", {
  rootPath: store.rootPath,
  year: store.selectedYear,
  month: store.selectedMonth,
});
store.commitmentProgress = result.roles;
store.commitmentProgressResult = result;
```

- [ ] **Step 2: 更新 MonthView 中传给 CommitmentsPanel 的 props**

CommitmentsPanel 的 props 目前接收 `progress: CommitmentProgress[]`。新增 `progressResult: CommitmentProgressResult` 或在组件内部直接从 store 读取 `commitmentProgressResult`。

建议：CommitmentsPanel 直接从 `useStore()` 读取 `commitmentProgressResult`，避免 props drilling。

- [ ] **Step 3: 确认前端编译和测试通过**

```bash
npx vue-tsc --noEmit 2>&1 | tail -10
npx vitest run 2>&1 | tail -20
```

Expected: 无类型错误，所有测试通过。

- [ ] **Step 4: Commit**

```bash
git add src/
git commit -m "feat: wire CommitmentProgressResult into data flow"
```

- [ ] **Step 5: 等待核对检查点**

---

### Task 12: 端到端验证 — 构建 & 类型检查 & 完整测试

**Files:**
- 所有文件

- [ ] **Step 1: 完整 Rust 编译检查**

```bash
cd src-tauri && cargo check 2>&1
```

Expected: 无编译错误或 warning。

- [ ] **Step 2: 完整 Rust 测试**

```bash
cd src-tauri && cargo test 2>&1
```

Expected: 所有测试 PASS。

- [ ] **Step 3: 完整前端类型检查**

```bash
npx vue-tsc --noEmit 2>&1
```

Expected: 无类型错误。

- [ ] **Step 4: 完整前端测试**

```bash
npx vitest run 2>&1
```

Expected: 所有测试 PASS。

- [ ] **Step 5: 构建**

```bash
pnpm run build 2>&1
```

Expected: 构建成功。

- [ ] **Step 6: 手动启动验证**

```bash
pnpm tauri dev
```

验证项：
- 打开 app，CommitmentsPanel 显示分段进度条（如有 role-tagged entry）
- DimensionPopover 中出现 Role 选项
- 录入时选 role=Developer 但无 goal，entry 显示在对应 role 的 light 段
- 录入时无 role 无 goal，entry 行显示 amber ● 标记
- CommitmentsPanel 底部显示 warning bar（有未归属 entry 时）
- CommitmentsModal 中修改 role 名，entry 的 role 维度同步更新

- [ ] **Step 7: 如有问题，修复并重新验证**

- [ ] **Step 8: Final commit**

```bash
git add -A
git commit -m "chore: final verification and cleanup for role dimension feature"
```
