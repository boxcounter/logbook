# Spec: 数据版本管理

日期: 2026-07-04

---

## 1. 背景

- 当前所有数据文件（`dimensions.template.yaml`、`{month}/dimensions.yaml`、`{month}/commitments.yaml`、`{date}.md`、`.logbook/operations/*.jsonl`）均无版本标识
- 向后兼容性仅靠 serde `#[serde(default)]`，新增字段靠默认值兜底，删除/重命名字段无法处理
- 历史格式变更（`template.yaml` 重命名、`_monthly.md` 拆分与删除）依赖一次性 init 时迁移代码，迁移逻辑侵蚀主应用
- 明确设计意图：版本检查在主应用，迁移在独立工具

---

## 2. 设计目标

1. 数据目录有版本号，主应用根据版本号判断数据格式是否兼容
2. 版本不兼容时，主应用不尝试修复——提示用户使用迁移工具
3. 现有存量用户（无 version.txt）需先运行迁移工具初始化版本号
4. 新用户 setup 时自动写入当前版本号

---

## 3. 数据模型

### 3.1 version.txt

- 路径：`{root}/version.txt`
- 格式：纯文本，内容为单个无符号整数（如 `1`），不含引号、换行或前后空白
- 写入者：
  - `set_root_path`（仅新用户 setup 时，写入 `CURRENT_DATA_VERSION`）
  - 迁移工具（bump 版本号）
- 读取者：`commands::init` 启动时检查
- 主应用不 bump 版本号，仅 setup 时写一次初始值

### 3.2 CURRENT_DATA_VERSION

- Rust 常量：`const CURRENT_DATA_VERSION: u32 = 1`
- 代表当前代码期望的数据格式版本
- 新增版本时 bump（仅由格式变更的 PR 修改）

### 3.3 InitResult 新增变体

```rust
// 无 version.txt
InitResult::DataVersionNotFound { root_path: String }

// version.txt 存在但版本号 != CURRENT_DATA_VERSION
InitResult::DataVersionMismatch { root_path: String, expected: u32, found: u32 }
```

### 3.4 前端类型

`types.ts` 中 `InitResult` 镜像需同步新增两个变体对应的类型和字段。

---

## 4. 命令流程变更

### 4.1 init

```text
init()
  → read_root_path()
  → 若无 root_path → NeedsSetup
  → check_data_version(root)      // 新增
    → 无 version.txt               → return DataVersionNotFound
    → 版本号 != CURRENT            → return DataVersionMismatch
    → 版本号 == CURRENT            → 继续
  → ensure_watcher()
  → load_root_state(root)
```

`check_data_version` 为纯函数，不修改文件。出错路径短路返回，不启动文件监听，不进入 `load_root_state`。

### 4.2 set_root_path

```text
set_root_path(root)
  → save_root_path(root)
  → write_version_file(root, CURRENT_DATA_VERSION)   // 新增
  → load_root_state(root)
  → ensure_watcher(root)
```

### 4.3 新增函数

| 函数 | 位置 | 职责 |
|------|------|------|
| `check_data_version(root) → Result<(), InitResult>` | commands.rs | 读 version.txt，比对版本号 |
| `write_version_file(root, version: u32) → Result<()>` | files.rs | 原子写入 version.txt |

---

## 5. 前端处理

- `DataVersionNotFound` 和 `DataVersionMismatch` 均渲染**数据迁移提示界面**
- 与现有 RecoveryScreen 同级（新建组件，不复用 RecoveryScreen）
- 界面内容：错误说明（附录 `expected` 和 `found` 版本号）、操作指引（使用迁移工具更新数据目录）
- 不提供换目录、跳过等绕过路径——格式不兼容时唯一出路是迁移
- `App.vue` 或 store 中新增对应的路由判断逻辑

---

## 6. 错误处理与边界情况

### 6.1 version.txt 存在但内容无效

- 文件为空、非整数、包含多余空白 → 视为 `DataVersionNotFound`
- 版本号为 0 → 视为无效，同 `DataVersionNotFound`

### 6.2 存量用户

- 已有 root_path 设置但无 version.txt 的存量用户，init 时触发 `DataVersionNotFound`
- 需先运行迁移工具写入 `version.txt`

### 6.3 文件监听

- `version.txt` 不被文件监听器监视——版本号在进程生命周期内不变
- 如果在应用运行期间外部工具 bump 版本号并重新加载页面，下次 init 会触发 `DataVersionMismatch`

### 6.4 迁移工具不在本 spec 范围

- 迁移工具是独立工具，不属于 Logbook 主应用
- 本 spec 仅定义主应用侧的版本检查与错误提示
- 迁移工具自身的设计留待后续

---

## 7. 测试

### 7.1 单元测试

| 场景 | 文件 |
|------|------|
| `check_data_version` 版本匹配 | commands.rs tests |
| `check_data_version` 版本不匹配 | commands.rs tests |
| `check_data_version` 文件不存在 | commands.rs tests |
| `check_data_version` 文件内容无效 | commands.rs tests |
| `write_version_file` 写入与读取一致性 | files.rs tests |

### 7.2 集成测试

| 场景 |
|------|
| 全新 setup → 写入 version.txt → init 成功 |
| 存量数据无 version.txt → init 返回 DataVersionNotFound |
| 手动修改 version.txt 为错误版本 → init 返回 DataVersionMismatch |
