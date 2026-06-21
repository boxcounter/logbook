# 月度维度模板（Monthly Dimension Templates）— 设计

> 关联：`SPEC.md`（数据结构、命令清单）、`docs/naming-conventions.md`（§5 落盘契约）、`docs/interaction-principles.md`（不制造意外副作用）

## 1. 问题与目标

**问题**：维度（dimensions）及其取值（values）当前是全局固定的——`config.yaml` 在 `init` 时加载一次，所有月份共用同一份维度定义。用户需要**每个月的维度集合和取值都可能不同**（C 类需求：维度增减 + 取值变化同时发生）。

**现状约束**：
- `config.yaml` 是全局单文件，维度集合跨月恒定。
- 仅 `source: "monthly"` 的维度（当前硬绑定 Goal）取值能按月变，且值来自 `_monthly.md` 的 commitments goals 并集，最多 1 个。
- 任意维度（如 Biz、Client）取值写死在 `config.yaml`，无法按月变；维度集合本身无法按月增减。

**目标**：让每个月拥有独立的维度定义（集合 + 取值），互不影响。

**非目标 / 边界**：
- **不做历史数据迁移**。项目处于 pre-production，仅测试数据，可随时清空。旧 `config.yaml` 不做兼容读取。
- **v1 不做 in-app 维度编辑器**。维度通过直接编辑文件（`template.yaml` / 月度 `_monthly.md`）维护，由文件监听重载——与今天编辑 `config.yaml` 的方式一致。in-app 编辑器是独立的后续 feature。

## 2. 核心模型：全局模板 + 月度快照（语义 B）

采用「全局默认模板 + 月度完整覆盖」，覆盖语义为**建月时复制（template / copy-on-write）**，而非读时回退：

- **`template.yaml`**（由 `config.yaml` 改名而来）：全局默认维度集。schema 不变，仍是 `dimensions: [...]`。它的角色是「新月份的起点模板」。
- **月度快照**：某月**首次被写入**时，把 `template.yaml` 当时的维度**快照**进该月 `_monthly.md` 的新 `dimensions:` 块。此后该月自包含，**改 `template.yaml` 不回溯影响已实例化的月份**。
- **未实例化的月份**：纯浏览时按 `template.yaml` 做**只读预览**，不落盘。

### 2.1 关键不变量

> 某月 `_monthly.md` 存在 `dimensions:` 块 ⟺ 该月已实例化。
> 某月的**生效维度** = 该月 `dimensions:` 块（若存在），否则 = `template.yaml` 的维度。

### 2.2 实例化触发时机（B-2：首次写入）

纯浏览/导航**不**实例化（遵循「翻看不等于创建」，避免在读动作上产生写副作用）。实例化发生在对该月的**首次写入**：`append_entry` / `update_entry` / `delete_entry` / `set_day_note` / `set_commitments`。

触发逻辑收敛到单一函数 `ensure_month_instantiated(root, year, month)`：

1. 读该月 `_monthly.md`（不存在则视为空）。
2. 若已有 `dimensions:` 块 → 直接返回（已实例化，幂等）。
3. 若无 `dimensions:` 块 → 把 `template.yaml` 的维度写入该月 `dimensions:` 块，**保留已有 `commitments:`（合并而非覆盖）**，原子写回 `_monthly.md`。

每个改写当月的命令在执行主体前调用它。`append_entry` 因此会顺带写 `_monthly.md`——这是 B-2 的必然代价（快照需要落点），属预期行为。

> **为何「合并而非覆盖」**：用户可能先设 commitments、后录第一条 entry。实例化若覆盖会冲掉已设的承诺。合并保证两种写入顺序都安全。

## 3. 落盘契约（disk contract）

按 `naming-conventions.md` §5，文件名与字段名是磁盘契约。本次为有意的契约变更，因无历史数据，**不做兼容迁移**（不读旧 `config.yaml`）。

### 3.1 `config.yaml` → `template.yaml`

路径 `{root}/template.yaml`，schema 不变：

```yaml
dimensions:
  - {name: Biz, key: biz, source: static, values: [产品, 市场]}
  - {name: Goal, key: goal, source: monthly}   # 值按月从该月 commitments 解析
```

### 3.2 `_monthly.md` frontmatter 新增 `dimensions:` 块

与现有 `commitments:` 并列：

```yaml
# 2026/07/_monthly.md
---
dimensions:
  - {name: Biz, key: biz, source: static, values: [产品, 市场]}
  - {name: Client, key: client, source: static, values: [甲, 乙]}
  - {name: Goal, key: goal, source: monthly}
commitments:
  - {role: Dev, allocation: 40, goals: [Ship it]}
---
```

`template.yaml` 的 `dimensions:` 与 `_monthly.md` 的 `dimensions:` 是**同一套 schema**（同样的 `Dimension` 结构），以保证「快照」是干净的整块复制，而非格式转换。

## 4. Rust 后端

### 4.1 `models.rs`
- `MonthlyFile` 增字段 `dimensions: Vec<Dimension>`（`#[serde(default)]`，缺省空 vec）。
- `Config` 结构语义变为「模板」。类型标识符可改名 `Template`（§4：类型名自由，不影响 payload）。本设计采用改名为 `Template`，但其序列化字段仍是 `dimensions`。

### 4.2 `files.rs`
- `config_path` → `template_path`，返回 `{root}/template.yaml`。
- 读模板的函数（原读 `config.yaml`）随之更名，错误信息 `"config.yaml not found"` → `"template.yaml not found"`（`models.rs:247` 附近）。

### 4.3 `config.rs`
- `validate_config` 抽为 `validate_dimensions(dims: &[Dimension]) -> Vec<ConfigErrorDetail>`，供 `template.yaml` 与月度块**复用**。原有规则不变（name/key 非空、key 合法字符、static 必有非空 values、最多 1 个 `source: monthly`、source 合法）。
- `validate_monthly` 在校验 commitments 之外，对该月 `dimensions:` 块（若存在）调用 `validate_dimensions`。
- `watch_files`：监听对象由 `config.yaml` 改为 `template.yaml`（含 `config.rs:170` 的 `file_name == "config.yaml"` 判断）+ 当月 `_monthly.md`。`template.yaml` 变更 → emit 事件，使**未实例化的当前月预览**刷新（已实例化的月不受影响）。

### 4.4 `commands.rs`
- 新增 `ensure_month_instantiated(root, year, month)`（见 §2.2）。在 `append_entry` / `update_entry` / `delete_entry` / `set_day_note` / `set_commitments` 主体前调用。
- 新增命令 `get_month_dimensions(root_path, year, month) -> Result<MonthDimensions, String>`，**纯读、不实例化**：
  ```rust
  struct MonthDimensions {
      dimensions: Vec<Dimension>,  // 月度块若存在，否则 template
      from_template: bool,         // true = 该月尚未实例化，当前展示的是模板预览
  }
  ```
  前端换月时调用，`from_template` 驱动「本月仍用默认模板」的预览标识。
- `create_starter_files`：写 `template.yaml`（替换 `config.yaml`，`commands.rs:938` 附近）。
- `get_stats`：维度取自该月 `dimensions:` 块（已实例化）；按 key 聚合逻辑不变，只是维度键集合现在按月而定。

### 4.5 `operation_log.rs`
- replay 拷贝逻辑（`config_src = root.join("config.yaml")`，`operation_log.rs:176/184` 附近）改为拷贝 `template.yaml`。测试 fixture 字面量同步更新。

### 4.6 IPC 契约变更（前后端同步，按破坏性变更对待）
维度从「全局、init 注入一次」变为「按所视月份动态」：

- `InitResult::Ready { config: Config, ... }` → `Ready { dimensions: Vec<Dimension>, from_template: bool, ... }`，返回**当前月的生效维度**（已实例化则为月度块，否则 template）及其 `from_template` 标志，供首屏预览标识使用。`init` 是纯读，**不实例化当前月**（遵循 B-2）。`today` / `commitments` 不变。
- `ConfigError` 变体保留（错误现可能来自 `template.yaml` 或当月 `_monthly.md`，message 已带上下文区分）。

## 5. 前端

### 5.1 状态：从全局单例到按月刷新
- `store.config`（`Config`，全局，`App.vue:95` / `SetupScreen.vue:27` 注入）→ 改为 `store.dimensions`（`Dimension[]`，当前所视月份的生效维度）。去掉前端的 `Config` 包装类型。
- **换月时**调用 `get_month_dimensions` 刷新 `store.dimensions`（与现有换月加载 commitments 的流程并行）。
- 消费点改写：
  - `EntryRow.vue:20` `store.config?.dimensions` → `store.dimensions`
  - `MonthView.vue:112`（`validKeys`）、`MonthView.vue:371`（传入 `:dimensions`）同上
  - `DimensionPopover` / `EntryComposer` / `EntryRowEdit` 仍通过 props 接收 `dimensions`，**无需改动**（数据源在上层已切换）。

### 5.2 未实例化月份的预览标识
当 `from_template === true`，在维度相关 UI 处给一个轻量提示（如 CommitmentsPanel 或录入区附近一行次要文字「本月沿用默认模板」）。首次写入后该标识消失。具体样式遵循设计 token 与交互原则，留待实现阶段定。

## 6. 错误处理

- `template.yaml` 非法 → `init` 返回 `ConfigError`（无法实例化任何月份）。
- 手改的某月 `dimensions:` 块非法 → 读该月（`get_month_dimensions` / 换月 / watcher 重载）时报错，经现有 `ConfigError` / 事件通道呈现。
- `source: monthly`（Goal）+「最多 1 个 monthly 维度」约束原样保留。`template.yaml` 中声明 Goal 但无 commitments 合法——`source: monthly` 跳过 values 校验，值在实例化后按该月 commitments 解析。
- `template.yaml` 缺失 → 沿用现有「缺配置」路径（`NeedsSetup` / 首启 setup 流程）。

## 7. 测试

### 单元（`src/` 内 `#[cfg(test)]`，纯函数）
- `validate_dimensions` 对 template 与月度块两种输入复用，覆盖原有全部规则。
- 快照合并：给定 template 维度 + 含 commitments、无 dims 块的 `MonthlyFile`，产出的块含 template 维度且保留 commitments。

### 集成（`tests/`，碰文件系统）
1. **空月纯读**：`get_month_dimensions` 返回 `template` 维度、`from_template = true`，且**未写任何文件**。
2. **首次写入实例化**：空月 `append_entry` 后，`_monthly.md` 出现 `dimensions:` 块且等于当时 `template`；随后修改 `template.yaml`，该月 `get_month_dimensions` 仍返回旧快照（`from_template = false`）。
3. **月度覆盖**：手写某月 `dimensions:` 块（与 template 不同）→ `get_month_dimensions` 返回月度块。
4. **commitments 触发**：空月 `set_commitments` 同样实例化 dims 块，且不冲掉刚设的 commitments（合并）。
5. **改名路径**：`create_starter_files` 写 `template.yaml`；`init` 读 `template.yaml`；watcher 识别 `template.yaml` 变更；operation_log replay 拷贝 `template.yaml`。

## 8. 实现影响面小结

| 层 | 改动 |
|---|---|
| 落盘 | `config.yaml`→`template.yaml`；`_monthly.md` 加 `dimensions:` 块 |
| `models.rs` | `MonthlyFile.dimensions`；`Config`→`Template` |
| `files.rs` | `config_path`→`template_path` + 读模板函数/错误信息 |
| `config.rs` | `validate_config`→`validate_dimensions`（复用）；`validate_monthly` 增 dims 校验；watcher 改名 |
| `commands.rs` | `ensure_month_instantiated`；`get_month_dimensions`；5 个写命令接入触发；`create_starter_files` 改名；`init` 返回 dims |
| `operation_log.rs` | replay 拷贝 `template.yaml` |
| 前端 store | `store.config`→`store.dimensions`，换月刷新 |
| 前端消费点 | `EntryRow.vue`、`MonthView.vue`（props 下游组件无需改） |
| 前端 UI | 未实例化月份的预览标识 |

**最大认知转变在前端**：`dimensions` 从全局单例变成跟随所视月份的状态——改动量主要来自此处；后端的快照逻辑是局部的。
