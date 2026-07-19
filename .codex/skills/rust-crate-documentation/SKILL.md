---
name: rust-crate-documentation
description: 为本项目的 Rust crate 新建或更新 README 与 `docs/` 设计文档。需要说明 crate 职责、状态契约、公开 API、设计取舍或人工 review 入口时使用。
---

# Rust crate 文档

按 [crate-documents.md](references/crate-documents.md) 编写 crate 文档。

## 默认流程

1. 阅读目标 crate 的 `Cargo.toml`、根导出、实现和测试。
2. 在 README 只写当前 crate 负责的模型、规则、状态契约和公开 API。
3. 在 `docs/` 记录代码已体现的设计取舍、模块划分、不变量和演进约束。
4. 从 README 链接设计文档，并使用相对路径。
5. 检查每个陈述都能由当前 crate 的代码或测试证明。

## 硬规则

- 不在当前 crate 文档中分配或推测其他 crate 的职责。
- 不把外部实现细节、历史过程或未来设想写成当前契约。
- README 说明使用者需要知道的边界；设计文档说明维护者需要知道的取舍。
- 修改 Rust API 注释时，另行使用 `rust-commenting`。
