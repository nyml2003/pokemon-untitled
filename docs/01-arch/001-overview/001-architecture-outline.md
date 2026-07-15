# 架构梳理大纲

## 结论

当前项目已经具备清晰的“领域 -> 用例 -> 表现 -> 适配 -> 运行时”主线，也有独立的基础图形栈和地图编辑器路径。它仍处在快速演进阶段：部分表现层持有交互状态，运行时 crate 仍承担较多组装工作，数据和资产边界尚未统一成稳定的端口接口。

本文档集的目标是先把这些事实写清，再给出可选的收敛方向。它不把目录分层当作强制架构规则。

## 要回答的问题

| 问题 | 文档位置 | 完成标准 |
| --- | --- | --- |
| 项目有哪些可执行程序和主要数据流？ | `001-overview/` | 能从输入追到状态、视图和平台输出。 |
| 每个 crate 做什么，不该做什么？ | `003-layers/` | 34 个 workspace package 均有职责说明。 |
| 业务领域如何划分？ | `002-domains/` | 战斗、世界地图、数据资产的模型与边界明确。 |
| 哪些层级关系已被依赖图打破？ | `003-layers/002-dependency-rules.md` | 标出具体依赖和风险，不用抽象判断代替证据。 |
| 状态、时间、输入、渲染、文件 I/O 分别由谁负责？ | `004-cross-cutting/` | 每类横切问题有所有者和数据流图。 |
| 下一轮功能如何落位？ | `005-evolution/` | 为存档、事件、NPC、脚本、编辑器、平台扩展给出落点。 |

## 文档目录

```text
docs/01-arch/
├── README.md
├── 001-overview/
│   ├── 001-architecture-outline.md
│   ├── 002-system-overview.md
│   └── 003-runtime-flows.md
├── 002-domains/
│   ├── 001-battle.md
│   ├── 002-world-and-map.md
│   └── 003-game-data-and-assets.md
├── 003-layers/
│   ├── 001-foundation.md
│   ├── 002-domain-and-application.md
│   ├── 003-presentation.md
│   ├── 004-adapters-and-runtime.md
│   └── 005-dependency-rules.md
├── 004-cross-cutting/
│   ├── 001-state-and-time.md
│   ├── 002-rendering-and-input.md
│   └── 003-data-assets-and-quality.md
└── 005-evolution/
    ├── 001-risks-and-debt.md
    ├── 002-extension-points.md
    └── 003-decisions-needed.md
```

## 事实采集方法

1. 以根 `Cargo.toml` 的 `[workspace].members` 为 crate 清单。
2. 以 `cargo metadata --no-deps` 的 package 依赖为实际依赖图。
3. 以 `src/` 的公开类型、入口函数和测试为职责证据。
4. 以运行时 crate 的 `main`、文件系统调用和窗口/GPU 调用定位副作用。
5. 以 `assets/`、`maps/`、`fixtures/`、`scripts/` 定义 workspace 级数据与工具边界。

## 当前已知结构

| 区域 | 数量 | 当前含义 |
| --- | ---: | --- |
| `domain` | 4 | 战斗规则、游戏数据、地图项目格式、世界坐标与遭遇规则。 |
| `application` | 5 | 战斗编排、战斗回放、整局游戏、地图编辑和世界用例。 |
| `presentation` | 9 | 交互状态、语义视图、渲染计划和程序化资产。 |
| `adapter` | 7 | 文件、数据导入、WGPU、Crossterm、Ramus 等外部技术边界。 |
| `runtime` | 2 | Winit 应用生命周期、窗口事件、持久化调用和依赖组装。 |
| `foundation` | 6 个 package | 网格、输入、GPU、终端、UI 与 Ramus 核心能力。 |
| `quality` | 1 | 端到端游戏场景验证。 |

上表的 foundation 数量包括 5 个 `punctum-*` crate 和 `ramus-core`。虽然根 `Cargo.toml` 没有显式列出 `ramus-core` 的路径，`cargo metadata` 将它识别为第 34 个 workspace member；其物理目录位于 `foundation/ramus` 下。

## 需要确认的产品方向

下列问题会改变长期边界。本文先按保守假设记录扩展点，不直接选边。

1. 游戏目标是单机 Gen3 风格试玩、可完成 RPG，还是可编辑的通用怪物 RPG 引擎？
2. `map-editor` 是团队内部制作工具，还是最终需要对外发布的独立产品？
3. Ramus 是仅用于战斗控制台命令，还是未来游戏事件、NPC 和任务的通用规则/脚本语言？
4. 地图项目、玩家进度和世界状态是否必须支持稳定存档与版本迁移？
5. 终端后端是否仍是需要支持的发布目标，还是仅保留作调试工具？

## 完成定义

- 每个 workspace package 至少在一篇 layer 文档中出现。
- 所有主程序都有时序或组件图。
- 每个领域都有“已有能力、缺失能力、扩展落点”。
- 每条架构建议明确触发条件、收益和代价。
- 文档不要求代码改动；源代码仍是行为的唯一事实来源。
