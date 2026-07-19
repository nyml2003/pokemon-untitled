---
name: rust-safety-standards
description: 在 Pokemon Untitled 中新增、修改或审查 Rust 代码时，落实无 panic 的显式失败处理、零 Clippy 压制和未使用变量治理。用于实现功能、修复缺陷、重构、代码审查或编写测试，尤其是涉及 Result、Option、索引、外部输入和 lint 警告时。
---

# Rust 安全开发规范

用显式分支和结构化错误表达普通失败。不要把错误、警告或未使用值藏起来。

## 引用路由

- 处理 `Result`、`Option`、索引或普通失败：读 [failure-handling.md](references/failure-handling.md)。
- 处理 Clippy、编译器未使用警告或测试设置：读 [lint-and-binding-rules.md](references/lint-and-binding-rules.md)。

## 默认流程

1. 先列出函数可能失败、缺失或越界的输入和状态。
2. 把调用链改成返回 `Result`、`Option` 或明确的业务分支。
3. 用 `?`、`match`、`let ... else`、`if let` 或安全访问 API 消费结果。
4. 删除未使用的值和参数；接口必须保留时使用精确的 `_`。
5. 修正 Clippy 报告的设计问题，不新增或扩大 lint 豁免。
6. 通过 `ops format --check`、`ops lint` 和与改动范围匹配的 `ops test` 验证。

## 硬规则

- 不在新增或修改的 Rust 代码中使用 `unwrap`、`expect`、`panic!`、`todo!`、`unimplemented!` 或 `unreachable!`。
- 不先判断再对同一个 `Result` 或 `Option` 调用 `unwrap`；在一次分支中绑定成功值或返回错误。
- 不用数组下标、整数转换或其他隐式 panic 路径处理外部、可变或未证明安全的输入。
- 不新增 `#[allow(clippy::...)]`、`#[expect(clippy::...)]`、`#[allow(unused...)]` 或等价的命令行 lint 压制。
- 未使用的绑定优先删除；确实需要忽略时，名称只能是 `_`，不能用 `_value`、`_unused` 等名称隐藏问题。
- 修改已有违规代码时，触及的函数或测试应一并移除该违规；不要求无关历史代码随功能改动整体清理。
