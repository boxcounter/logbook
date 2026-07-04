# 命名约定

治理本项目（Tauri + Vue + TS）的命名。新增/修改代码、code review 时按此核对。

## 1. 组件按职责命名，不按外观/结构

名字要回答「它负责什么」，不是「它长什么样」。

- ✗ `TwoLineInput`（描述"两行"布局）
- ✓ `EntryComposer`（描述"组合一条 Entry"）

UI 后缀（`Popover` / `Modal` / `Panel` / `Card` / `Row`）允许且鼓励——它表达组件在界面里的角色，不算"按外观命名"。如 `DimensionPopover`、`CommitmentsModal`、`RoleCard` 均合规。

## 2. 目录分层：base / composite

- `src/components/base/`：无领域逻辑的 UI 原语（`AppButton`、`ProgressBar`、`Toast`）。
- `src/components/composite/`：含领域逻辑的组合组件。
- 根目录 `src/components/`：当前仍有历史遗留的领域组件（`MonthView`、`EntryList`、`DimensionPopover` 等）。新增组件按上述二分归位；不为单个组件做孤立迁移（会制造新的不一致）。

## 3. 输入 DTO 用 `*Input` 后缀

跨前后端传递的输入参数类型，用 `*Input` 后缀与持久化实体区分：

- `CreateEntryInput` / `UpdateEntryInput`（前端→后端的命令参数）vs `Entry`（持久化实体）。
- 这同时点明跨层差异，例如 `CreateEntryInput.duration` 是前端预解析的字符串，而 `Entry.duration` 是分钟整数。

## 4. 前后端同一概念用同一词

同一领域概念在 Rust 与 TS 两侧用同一个词。

**serde 字段名即 IPC 契约**：改 `models.rs` 里 struct 的字段名，等于改前后端通信的 JSON 键，必须两端同步，按破坏性变更对待。只改 struct/interface 的**标识符**（类型名）则不影响 payload。

## 5. 落盘格式与代码标识符解耦

磁盘契约独立于代码标识符，互不绑定：

- `operation_log.rs` 落盘的 op 字符串（`"append"`/`"update"`/`"delete"`/`"set_day_note"`）是 `match` 里的硬编码字面量，与 `Operation` 枚举的 Rust 名无关——可自由重命名枚举，但**勿动字面量**。
- frontmatter / `_monthly.md` / `config.yaml` 的字段名是磁盘契约，同理。
- 要改落盘字面量/字段名，必须走显式迁移（读旧、写新），不能裸改。

## 常见误判

- `AvailableMonth`（`{ year, month }`）命名正确：每个元素代表**一个**月份，`availableMonths: AvailableMonth[]` 是其数组。单数类型名 + 复数数组变量名是正确组合，不要"理顺"成复数类型名。

## 6. CLI 动词约定

### 动词矩阵

| 动词 | 操作 | 适用 |
|------|------|------|
| `list` | 取集合 | `entries list`, `commitments list`, `dimensions list` |
| `get` | 取单体 | 暂无，未来 `entries get --id <uuid>` |
| `add` | 集合内新增 | `entries add` |
| `update` | 修改单体 | 未来 `entries update --id <uuid>` |
| `delete` | 删除单体 | 未来 `entries delete --id <uuid>` |
| `set` | 整体替换 | `commitments set`, `dimensions set` |
| `progress` | 衍生计算视图 | `commitments progress`（领域专用名，非 CRUD） |

### 区分规则

- **`list` vs `get`**：资源是集合用 `list`，是单件用 `get`。不互为别名。
- **`set` vs `update`**：`set` 操作资源本身（整体替换），`update` 操作集合中单个成员（需要 `--id` 或 `--key` 定位）。
- **stdin 约定**：所有写入命令（`add`、`set`）统一从 stdin 读取 JSON 或 YAML，不用 CLI flags 分散传参。
