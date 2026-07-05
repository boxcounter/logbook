# Data Integrity Guard — Design

日期：2026-07-05
状态：Approved

## 目标

保护用户数据的完整、不被破坏。核心威胁模型：

1. **内部操作异常** — App 自身 bug 或 crash 导致的写中断、半写、不一致
2. **外部修改** — 外部编辑器（含 AI Agent）修改文件导致格式/语义错误，App 继续读写造成数据进一步损毁、扩散为不可逆破坏

检测到任何不一致后，整个 App 切换为只读模式，禁用所有写入操作，用人类易读的方式向用户清晰告知。

## 架构

新增 `IntegrityGuard` 模块（Rust 侧），核心是一个全局 `AtomicBool` 控制读写状态。

```
init() → integrity_scan_recent() → 全量扫描近 3 月
   ↓ 任一文件 format/semantic 错误
integrity_ok ← false → App 进入只读模式
   ↓ 全部通过
integrity_ok ← true → App 正常

append/update/delete/set_day_note/set_commitments
   ↓ 每次调用入口
guard.check()? → true → 执行写入
   ↓ = false
返回 Err → 前端不显示编辑/录入界面

file watcher 检测到 config 文件变更
   → 重新校验受影响月份 → 通过 → guard.reset() → 恢复正常
```

### 核心接口

```rust
impl IntegrityGuard {
    fn check() -> Result<()>;           // 所有写操作入口调用
    fn set_compromised(reason: IntegrityIssue);  // 标记只读
    fn reset();                          // 恢复正常
    fn status() -> IntegrityStatus;      // 前端查询状态和原因
}
```

### 状态模型

```
          启动全量扫描通过
  [Normal] ──────────────────────────────────────────► [Normal]
     ▲                                                    
     │ reset() (config 修复后 watcher 触发重扫通过)         
     │                                                     
     └── [ReadOnly] ◄── set_compromised()                  
             启动扫描失败 / 写入前校验失败 / watcher 重扫仍失败
```

`ReadOnly` 状态下所有写操作（append/update/delete/set_day_note/set_commitments）在 `guard.check()` 处被拦截，返回 `Err`。

## 校验范围

当前月 + 过去 3 个月（含跨年边界）。扫描该范围内所有 `{YYYY}-{MM}-{DD}.md` 文件。不追踪"上次扫过"，每次启动全扫这 4 个月。

### 第一层：格式完整性

| 检查项 | 触发条件 | 校验内容 |
|--------|---------|---------|
| YAML parse | 读 day file | frontmatter 可成功解析为 `DayFile` struct |
| JSONL parse | 读 op log | 每行可成功解析为 `OpLogEntry` struct |
| UTF-8 有效 | 读任意文件 | 文件内容为有效 UTF-8 |

### 第二层：语义完整性

| 检查项 | 触发条件 | 校验内容 |
|--------|---------|---------|
| duration 合法 | 解析 entry 后 | `duration: u32 > 0` |
| UUID v4 格式 | 解析 entry 后 | `id` 匹配 `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx` |
| dimension key 有效 | 解析 entry 后 | entry 的 dimension key 存在于当月 `dimensions.yaml`（不 fallback 到 template） |
| dimension value 非空 | 解析 entry 后 | `required` 维度的值不为空字符串 |

不做跨文件一致性（op log vs day file、goal/role 引用完整性）——那属于业务逻辑校验，不在本次范围。

### 启动全量扫描

- 范围：当前月 + 过去 3 个月
- 每找到一个错误 → 立即设置只读，停止扫描（fail-fast）
- 全通过 → 正常

### 写入前校验（pre-write gate）

每次写操作（append/update/delete/set_day_note/set_commitments）：

1. `guard.check()` — 全局只读 → 直接返回 `Err`
2. 对**目标文件**做轻量 format/semantic 校验（仅该文件）
3. 通过 → 正常写入；不通过 → `guard.set_compromised(issue)`，返回 `Err`

只读状态的错误信息不需要重复告知用户（启动横幅已在），直接返回错误即可。

## 用户告知

当只读模式激活时，前端在 MonthView 顶部展示一条持久化错误横幅：

**概要**：「数据保护模式已激活 — 文件异常，录入与编辑已暂停」

**展开明细**：受损文件路径 + 具体错误。例如：
- `2026/07/05.md：YAML 解析失败，第 12 行`
- `2026/06/30.md：条目 a3f2... 的 duration 为 0`

**修复指引**：「请用文本编辑器修复以上文件后，按 ⌘R 重新加载；或从备份恢复文件后重启 App」

横幅不阻塞浏览历史数据，但覆盖录入框区域（EntryComposer）使其不可交互。

## 文件监听与恢复

当 file watcher 检测到监听的 config 文件（`dimensions.template.yaml`、当月 `dimensions.yaml`、`commitments.yaml`）发生变更：

1. 重新执行**受影响月份**的完整性校验（仅该文件所在月，不全量扫 4 个月）
2. 通过 → `guard.reset()`，移除横幅，恢复写入
3. 仍不通过 → 保持只读，更新横幅中的错误详情

**day file 不在 watcher 监听范围内**（大量文件不宜全量 watch）。修复 day file 后用户需手动 `⌘R` 触发重扫。

## CLI 行为

只读状态下所有写命令返回错误：

```
Write denied: data integrity compromised
```

不执行任何写入。读命令（list、progress）不受影响。

## 实现范围

- **Rust 侧**：`IntegrityGuard` 模块、启动扫描、写入前校验、watcher 重扫逻辑
- **前端侧**：横幅组件、录入框禁用、错误详情展示、⌘R 重扫支持
- **CLI 侧**：只读拦截
- **测试**：集成测试覆盖启动扫描（正常/异常）、写入前校验（正常/异常）、恢复流程
