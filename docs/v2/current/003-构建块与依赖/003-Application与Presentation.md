# Application 与 Presentation

> 分类：现状；最后核对：2026-07-20。
> 依据：application/presentation crate 的根导出、`game-session`、`world-application`、`game-scene-view` 与 `game-native-plan`。

## Application 持有用例状态

application 的六个 crate 将领域模型编排为可调用用例。它们接受类型化命令，执行状态转换，返回事件、观察或错误；不接受 Winit 事件、不读取目录，也不提交渲染命令。

| crate | 状态或用例边界 |
| --- | --- |
| `game-session` | 一局产品游戏的权威会话，协调世界与战斗场景。 |
| `world-application` | 世界用例、角色外观、NPC 脚本进度和世界观察。 |
| `battle-application`、`battle-session` | 对战动作、对战交互与播放协调。 |
| `map-editor-core` | 地图编辑模型、意图、效果和控制器。 |
| `tile-editor-core` | 瓦片语义编辑状态与操作。 |

`GameSession` 以拥有权转移的形式执行 `transition`：输入 `GameCommand`，返回更新后的 session 与 `Result<GameEvents, GameError>`。它通过 `snapshot` 生成只读 `GameSnapshot`。`WorldApplication` 则在 `World` 之外持有角色外观、NPC 脚本延续和对白，使领域世界不依赖创作项目的表现细节。

## Presentation 只投影

presentation 的十个 crate 将状态和资产描述转换为可渲染的数据，不拥有 `GameSession`、`WorldApplication` 或编辑器的权威状态。

| 组 | crate | 输出 |
| --- | --- | --- |
| 资产 | `game-assets`、`map-assets`、`game-asset-plan` | 资产键、解码图像、地图资源和请求计划。 |
| 视图与 UI | `game-view`、`game-ui`、`game-ui-kit` | 游戏视图、表现状态、UI tree 与已布局的 `UiFrame`。 |
| 场景与地图 | `map-render`、`map-editor-view`、`game-scene-view` | 地图层、编辑器视图和 `SceneFrame`。 |
| native 计划 | `game-native-plan` | `NativeAssets`、帧 pass、文本标签与 `FramePlan`。 |

`game-scene-view::project_scene` 接收游戏快照、表现快照、地图、资产目录和 viewport，返回 `ProjectedScene`。它根据世界、战斗、图鉴和控制台状态选择 `SceneFrame`，但不创建窗口或调用 GPU。`game-native-plan` 将已经解析的 `GameView` 或 `UiFrame` 转为 `FramePlan`；atlas resource ID 和文本边界也在这一步确定。

## 两类状态不得混淆

游戏状态决定玩家在世界或战斗中的事实，应保留在 application/domain。表现状态决定控制台是否打开、动画偏移、精灵帧或图鉴选择，应留在 `game-ui` 等 presentation crate。前者通过命令、事件和快照变化；后者从快照派生或响应已归一化输入。

presentation 可以显示操作失败，但不能用 UI 组件直接改写领域对象。runtime 负责把表现动作转换为 `GameCommand`、编辑意图或其他 application 输入。
