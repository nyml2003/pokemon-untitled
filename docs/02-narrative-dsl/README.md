# 叙事 DSL 详细设计

本目录细化[开放世界叙事脚本 DSL 与 Ramus 能力层提案](../01-arch/006-proposals/004-narrative-dsl-and-ramus.md)。所有内容都是未实施的设计，不代表当前 Rust workspace 已具备对应能力。

## 阅读顺序

1. [语言、词法与文法](001-language-and-grammar.md)：ASCII 规则、资源 ID、LL(1) 产生式、流式 parser 事件和编译诊断。
2. [编译产物与任务运行时](002-compiler-and-task-runtime.md)：纯流式编译、CPS continuation 图、拦截器、结算和存档恢复。
3. [Ramus、世界提交与审计](003-ramus-world-commit-and-audit.md)：capability、provider、`WorldPatch`、账本、AI 提案和 CLI/GUI。

## 共同约束

- 脚本只表达受限流程，不直接修改 `WorldState`。
- Ramus 只授权和调度，不拥有游戏世界。
- 世界应用层是唯一的状态提交出口。
- 模型输出不可信，必须经过解析、编译、校验和世界规则检查。
- 打开 UI、等待 GPU 或等待模型不推进世界；结果只在离散结算点应用。
