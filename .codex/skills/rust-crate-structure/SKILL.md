---
name: rust-crate-structure
description: 拆分或重组本项目的 Rust crate 源文件、模块和公开 API。大 `lib.rs`、模块职责混杂、需要收紧 `pub` 可见性，或需要在不破坏调用方的前提下建立根导出面时使用。
---

# Rust crate 结构

按 [module-structure.md](references/module-structure.md) 拆分 crate。

## 默认流程

1. 阅读目标 crate 的 `Cargo.toml`、`src/` 和测试。
2. 搜索 workspace 对该 crate 的使用，列出已有公开路径。
3. 按领域职责划分模块，而不是按函数数量机械切文件。
4. 让 `lib.rs` 只保留 crate 文档、私有 `mod` 声明和逐项 `pub use`。
5. 将跨模块实现细节收紧为 `pub(crate)`，其余成员保持私有。
6. 运行格式检查、crate 测试和依赖方检查。

## 硬规则

- 模块默认私有；不要为了方便使用 `pub mod`。
- 不使用通配符 re-export；根导出面必须逐项列出。
- 结构重组不改变已有业务行为和公开根路径，除非任务明确要求。
- 不为模块拆分新增只供内部使用的公开 setter 或 helper。
