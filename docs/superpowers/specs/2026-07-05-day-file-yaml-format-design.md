# Day File YAML Format

Date: 2026-07-05
Status: approved

## Motivation

日文件当前是 `.md` 扩展名，内容实质是 YAML frontmatter（`---\n{yaml}\n---`，无正文）。`.md` 扩展名具误导性——文件里没有 Markdown。切换为 `.yaml` 消除语义不一致。

## Design

### 核心变更

**`src-tauri/src/files.rs`**

- `day_path()`：`.md` → `.yaml`（一行）
- `read_day_file()`：`parse_frontmatter::<DayFile>(&content)` → `yaml_serde::from_str::<DayFile>(&content)`。保留 BOM strip（`content.trim_start_matches('\u{feff}')`），与 `read_dimensions_file()` 一致
- `write_day_file()`：去掉 `format!("---\n{}---\n", yaml_body)` 包装，直接写 `yaml_body`
- 删除 `parse_frontmatter()` 函数及其单元测试（无其他调用方）

**`src-tauri/src/scan.rs`**

- 注释 `.md` → `.yaml`
- `file_name.ends_with(".md")` → `"yaml"`
- `_monthly.md` 跳过守卫删除（文件已不存在，逻辑冗余）
- `trim_end_matches(".md")` → `trim_end_matches(".yaml")`，同时修正 strip 长度（3 → 5）
- 测试 fixture 文件名更新

**`src-tauri/src/commands.rs`**

- 所有 `file_name.ends_with(".md")` → `"yaml"`（约 10 处）
- 所有 `file_name.trim_end_matches(".md")` → `"yaml"`（约 10 处）
- 所有 `_monthly.md` 守卫删除（清理已死逻辑）
- `resolve_reveal_target` 注释更新
- 所有测试 fixture 更新

**`src-tauri/src/integrity.rs`**

- `file_name.ends_with(".md")` → `"yaml"`，去掉 `_monthly.md` 守卫
- `trim_end_matches(".md")` → `"yaml"`

**`src-tauri/src/operation_log.rs`**

- `collect_md_files` → `collect_yaml_files`（函数重命名）
- `collect_md_files_recursive` → `collect_yaml_files_recursive`
- `ext == "md"` → `"yaml"`
- 相关注释更新

**`src-tauri/src/models.rs`**

- `CURRENT_DATA_VERSION`: `1` → `2`

**`src-tauri/tests/`（集成测试）**

- 所有 `.md` 文件名/路径更新为 `.yaml`
- 测试数据内容去掉 `---` 包装

### 不变的部分

- `DayFile` / `Entry` / `CreateEntryInput` / `UpdateEntryInput` 结构体及字段不变
- 序列化用 `yaml_serde` 不变
- 原子写入（`.tmp` + rename）不变
- 文件路径结构 `{root}/{YYYY}/{MM:02}/{YYYY}-{MM:02}.{ext}` 不变（仅扩展名改）
- 月目录下 `dimensions.yaml`、`commitments.yaml` 不变
- `operation_log` JSONL 的 op 字面量（`"append"`/`"update"`/`"delete"`/`"set_day_note"`）不变
- 前端除 `reveal_day_file` mock 外无代码变动（日文件内容不流向前端结构体——走 `DayFile` serde，扩展名对前端透明）

### 迁移

独立迁移工具，逻辑：

1. 读取 `root_path.txt` 定位数据目录
2. 遍历所有 `YYYY-MM-DD.md` 文件
3. 每个文件：读入 → 去掉 `---\n` 头和尾部 `\n---` → 写入 `YYYY-MM-DD.yaml` → 删除 `YYYY-MM-DD.md`
4. `version.txt` 写入 `2`
5. 幂等：已有 `.yaml` 的同名文件跳过

不支持双格式兼容期。pre-PMF 阶段，迁移是跑一次的事。

### 测试策略

- 单元测试：`files.rs` 内更新 `day_path` / `read_day_file` / `write_day_file` 的测试
- 删除 `parse_frontmatter` 相关测试
- 集成测试：`tests/` 下所有 fixture 文件名和内容更新
- `scan.rs` 测试：fixture 文件名从 `.md` 改 `.yaml`，删除 `_monthly.md` 相关 case
- 无新增测试（纯格式等价变换）
