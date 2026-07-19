# Foundation 与 Domain

> 分类：现状；最后核对：2026-07-20。
> 依据：foundation/domain crate 的 `Cargo.toml`、根导出与单元测试。

## Foundation 是无业务前提的模型

foundation 的六个 crate 不依赖游戏 crate、文件系统、窗口或平台 API。它们提供可由多个上层复用的纯数据模型和转换。

| crate | 关键输出 |
| --- | --- |
| `punctum-grid` | `GridPos`、`GridRect`、`GridSize`、`Surface` 与有界 `Patch`。 |
| `punctum-input` | `KeyEvent`、`TextEvent`、物理键、逻辑键与修饰键合同。 |
| `punctum-gpu` | atlas、viewport、`SubmissionPlan` 与提交字节编码。 |
| `punctum-terminal` | 终端 cell、尺寸和 patch 计划。 |
| `punctum-ui` | `UiTree`、布局后的 `UiFrame`、绘制命令和命中区域。 |
| `ramus-core` | capability tree、授权、解析、编译、计划和执行前检查。 |

`punctum-input` 将“键的含义”和“已提交文字”分开：字符键不等于文字提交，文本只能经非空 `TextEvent` 进入。`punctum-ui` 只构建和解析 UI 树；它不持有应用状态、加载资源或调用渲染器。`punctum-gpu` 只计划和编码 GPU 数据，WGPU 提交留给 adapter。

## Domain 定义游戏语言

domain 的九个 crate 定义各业务区域的合法数据、ID、规则与错误，不决定用户界面、路径或执行节奏。

| 领域 | crate | 当前边界 |
| --- | --- | --- |
| 对战 | `battle-domain` | 宝可梦、招式、回合与对战规则。 |
| 游戏数据 | `game-data` | 已校验的只读数据集和图鉴数据。 |
| 地图 | `map-project`、`map-tile-semantics` | 地图项目、瓦片、碰撞、事件和语义规则。 |
| 世界 | `world-domain`、`world-project` | 世界网格、角色、命令、事件与项目模型。 |
| 叙事 | `narrative-token`、`narrative-cps`、`narrative-compiler` | token、延续表示和脚本编译。 |

ID、位置、方向、命令和错误使用专门类型，而不是在上层散落字符串和整数约定。`map-project` 的校验结果可被 storage 和编辑器共同使用；`world-domain` 的世界规则不需要知道地图来自 JSON、二进制容器还是内存构造。

## 纯度边界

domain 可以内嵌只读且版本受控的数据，例如 `game-data` 用 `include_bytes!` 加载生成的数据集。它不读取运行时路径或目录。需要读取、写入、网络、窗口、GPU 或真实时钟时，调用者必须在 adapter/runtime 提供数据，或由 application 以命令和输入值接收。

规则失败通过领域错误类型返回。上层可以显示错误、映射退出码或记录日志，但不应把错误文本解析回规则分支。
