# 项目架构文档

这组文档同时保存两类材料：

- 当前 Rust workspace 的结构、职责和依赖边界。
- 尚未收口的产品与领域访谈记录。

文档依据 2026-07-15 的源码、`Cargo.toml`、`cargo metadata --no-deps` 和根目录资源树整理。它描述的是当前实现，不是已经批准的重构计划。每一篇都把“现状”和“建议”分开。

## 阅读顺序

1. [世界人物与 NPC 契约方案](006-proposals/002-world-characters-and-npc-contract.md)：主角与 NPC 的统一投影契约、动作能力、地图人物和分阶段迁移方案。
2. [Flex UI 布局与 GPU 渲染改造方案](006-proposals/001-flex-ui-layout-and-rendering.md)：`punctum-ui`、像素 GPU 提交、页面迁移和验收顺序的提案草案。
3. [宏观战略版：设定总纲与核心架构](000-interviews/002-macro-strategy-outline.md)：公平准则、培育、经济、势力、终局和长线战略。它是战略输入，不等于实现决策。
4. [Agent 与活世界：战略访谈记录](000-interviews/001-agent-world-vision-interview.md)：本轮产品、世界和 UI 访谈的原始结论；本轮已结束。
5. [架构大纲](001-overview/001-architecture-outline.md)：文档范围、问题清单和目录。
6. [系统总览](001-overview/002-system-overview.md)：分层、工作区边界和核心术语。
7. [运行时流程](001-overview/003-runtime-flows.md)：游戏、地图编辑器和数据导入的端到端路径。
8. `002-domains/`：战斗、世界地图、数据资产三个业务领域。
9. `003-layers/`：各层 crate 的职责、依赖方向和例外。
10. `004-cross-cutting/`：状态、渲染、输入、资产和质量策略等横切问题。
11. `005-evolution/`：已发现的边界风险、扩展点和待决策问题。
12. `006-proposals/`：基于已确认战略输入编写、但尚未实施的改造方案。

## 文档约定

- `crate` 指 Cargo package。本文不把目录名自动等同于运行时边界。
- “纯”指不直接读写文件、时钟、窗口、GPU 或进程环境；它不等于没有内部状态。
- “所有者”指创建、修改并决定某类状态生命周期的 crate。
- PlantUML 图使用 Markdown 代码块保存，可被支持 PlantUML 的编辑器直接渲染。
- 根 `Cargo.toml` 列出的 33 个 package 是本文统计范围。`crates/foundation/ramus/ramus/` 下的嵌套副本不在根 workspace 成员中，单独在演进文档说明。
- `000-interviews/` 记录探索中的产品语义。除非文中明确标记为已决定，否则不能把它当作实现授权。

## 不在本文范围内

- 不修改当前 crate 边界、依赖或 API。
- 不承诺完整的宝可梦玩法、存档、联网或脚本系统已经存在。
- 不将 `assets/` 的数据文件误称为 Rust crate；它们是 workspace 级项目数据。
