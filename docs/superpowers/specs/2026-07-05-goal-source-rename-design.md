# Spec: commitment:goals → commitments:role:goals 重命名

日期: 2026-07-05

---

## 1. 背景

`source: "commitments:goals"` 未能反映数据模型中 goals 嵌套于 role 下的层级关系。`commitments.yaml` 的结构为：

```yaml
- role: Developer
  allocation: 40
  goals:
    - Feature A
    - Code review
```

goals 并非 commitments 的顶层字段，而是每个 role 的子字段。`"commitments:role:goals"` 精确镜像此路径，降低配置阅读者的认知负担。

---

## 2. 设计目标

`"commitments:goals"` → `"commitments:role:goals"` 纯字符串重命名，无行为变更。

- `"commitments:role"` 保持不动（roles 本身是 commitments 的顶层字段，无需下钻）
- 不引入向后兼容（与上次 `monthly` → `commitments:goals` 迁移同策略）

---

## 3. 变更范围

### 3.1 Rust 后端

| 文件 | 变更 |
|------|------|
| `models.rs` | `Dimension.source` 注释中合法值列表更新 |
| `config.rs` | `validate_dimensions` 的 `"commitments:goals"` match arm → `"commitments:role:goals"`；错误消息中合法值枚举更新 |
| `commands.rs` | `goal_dim_key()` 内部 `source == "commitments:goals"` → `"commitments:role:goals"`；内联 YAML 种子数据 (~8 处) |
| 集成测试 (~10 文件) | inline YAML `source: commitments:goals` → `commitments:role:goals` |
| `tests/fixtures/template.yaml` | 模板 fixture source 字段更新 |

### 3.2 前端

| 文件 | 变更 |
|------|------|
| `types.ts:7` | 联合类型：`"commitments:goals"` → `"commitments:role:goals"` |
| `DimensionEditorModal.vue` | 下拉 `<option>` value、去重校验字面量、信息卡文案 |
| `DimensionPopover.vue` | `source === "commitments:goals"` → `"commitments:role:goals"` |

### 3.3 前端测试

~10 文件，fixture 中 `source: "commitments:goals"` → `"commitments:role:goals"`。

### 3.4 不动

- `"commitments:role"` — 不受影响
- `"MultipleGoalSource"` error kind — 描述的是 "goal source 维度重复"，与 source 字符串值无关
- `commitments.yaml` — 数据文件，不含 source 标识
- goal→role cross-filter 逻辑
- `goal_dim_key()` / `role_dim_key()` 的行为语义

---

## 4. 迁移

存量 `dimensions.yaml` / `dimensions.template.yaml` 中 `source: commitments:goals` 需手动改为 `source: commitments:role:goals`。`validate_dimensions` 报 `InvalidSource` 时列出合法值，用户按提示修改。
