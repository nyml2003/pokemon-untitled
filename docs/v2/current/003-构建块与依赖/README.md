# 003 构建块与依赖

> 分类：现状；最后核对：2026-07-20

本模块说明 45 个 package 的层级归属、职责边界和允许的依赖方向。它不替代单个 crate 的 API 文档。

| 顺序 | 页面 | 内容边界 |
| ---: | --- | --- |
| 001 | [分层职责](001-分层职责.md) | 七个层的总职责、数量和禁止项。 |
| 002 | [Foundation 与 Domain](002-Foundation与Domain.md) | 纯通用模型、领域规则、数据与叙事模型。 |
| 003 | [Application 与 Presentation](003-Application与Presentation.md) | 会话、编辑器状态、视图、UI 与帧计划。 |
| 004 | [Adapter、Runtime 与 Quality](004-AdapterRuntime与Quality.md) | 外部能力、可执行程序和质量工具。 |
| 005 | [依赖规则与数据合同](005-依赖规则与数据合同.md) | 允许的依赖、命令、事件、快照与帧计划。 |

阅读本模块后，按任务进入 [004 游戏运行时](../004-游戏运行时/README.md) 或 [005 地图创作](../005-地图创作/README.md)。
