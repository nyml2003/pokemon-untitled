# 叙事 DSL 详细设计

本目录细化[开放世界叙事脚本 DSL 与 Ramus 能力层提案](../01-arch/006-proposals/004-narrative-dsl-and-ramus.md)。除下列编译器基线外，其余内容仍是未实施设计。

当前已实现的基线位于 `narrative-token`、`narrative-cps`、`narrative-compiler` 和 `world-application`：它支持分块内存字节流、ASCII token、actor 绑定、`move`、`face`、`say`、`wait`、`end` 和 `ScriptProgram` continuation 图。`world-application` 每个逻辑 tick 为每个 actor-bound 脚本执行一个 CPS 节点，并通过既有世界移动规则提交结果。`end()` 会在该 NPC 的下一 tick 重启脚本。

当前运行时只支持确定性的 NPC 基础行为。`say` 保存 `text:` 资源键；游戏视图目前把键名作为临时气泡文本。资源 catalog、正式 i18n、Ramus、存档、拦截器、世界账本和文件缓存仍未实现。

## 阅读顺序

1. [语言、词法与文法](001-language-and-grammar.md)：ASCII 规则、资源 ID、LL(1) 产生式、流式 parser 事件和编译诊断。
2. [编译产物与任务运行时](002-compiler-and-task-runtime.md)：纯流式编译、CPS continuation 图、拦截器、结算和存档恢复。
3. [Ramus、世界提交与审计](003-ramus-world-commit-and-audit.md)：capability、provider、`WorldPatch`、账本、AI 提案和 CLI/GUI。
4. [编译制品、观测与边界治理](004-build-observability-and-boundaries.md)：并行预编译、缓存、构建事件和依赖方向。

## 共同约束

- 脚本只表达受限流程，不直接修改 `WorldState`。
- Ramus 只授权和调度，不拥有游戏世界。
- 世界应用层是唯一的状态提交出口。
- 模型输出不可信，必须经过解析、编译、校验和世界规则检查。
- 打开 UI、等待 GPU 或等待模型不推进世界；结果只在离散结算点应用。
