# 项目代理指引

这是一个 Rust workspace。修改前先阅读目标 crate 的 `Cargo.toml` 和相关源码，再决定边界归属；不要根据目录名猜测职责。

除代码、命令、路径和必要技术术语外，文档、注释、提交说明和协作沟通尽可能使用中文。

## 核心原则

- 分层披露：此文件只保留跨任务的原则和规则入口；实现细节、流程和例外写在对应的 skill 或主题文档中，按任务读取。
- 证据优先：当前源码、manifest 和测试优先于文档。变更使现状文档失真时，在同一变更中更新或删除无法证明的陈述。
- 边界优先：按职责与可替换边界归属代码，保持依赖由外向内；不要为便利让内层依赖外部能力。
- 纯核心：规则与状态转换保持确定、可测试，并通过类型化输入、结果和错误跨越边界；窗口、文件、网络、真实时间和平台 API 留在外壳。
- 单向状态流：外部输入先转换为核心命令，状态变化只由状态所有者执行；视图和渲染只消费快照与投影，不能直接改写业务状态。
- 验收匹配风险：无窗口验证不能证明原生平台行为；按改动影响范围选择验证，并如实报告没有执行的验证。

架构事实、分层和数据合同见 [当前架构](docs/v2/current/001-架构总览/README.md)、[构建块与依赖](docs/v2/current/003-构建块与依赖/README.md) 与 [开发与验收](docs/v2/current/008-开发与验收/README.md)。

## 按需读取

- 新增或修改 Rust 的 `//`、`//!`、`///` 注释时，读取 [Rust 注释规范](.codex/skills/rust-commenting/SKILL.md)。
- 新增或修改 crate 的 README 或 `docs/` 设计文档时，读取 [Rust crate 文档规范](.codex/skills/rust-crate-documentation/SKILL.md)。
- 拆分或重组 crate、收紧模块可见性、调整根导出面时，读取 [Rust crate 结构规范](.codex/skills/rust-crate-structure/SKILL.md)。
- 新增、修改或审查 Rust 代码时，读取 [Rust 安全开发规范](.codex/skills/rust-safety-standards/SKILL.md)。
- 执行或建议配置、格式化、检查、测试、同步、构建、CI 或原生运行验证时，读取 [Ops First Workflow](.codex/skills/ops-first-workflow/SKILL.md)。
