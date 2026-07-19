# Pixel UI 路径

> 分类：现状；最后核对：2026-07-20。
> 依据：`punctum-ui`、`game-ui-kit`、`game-ui`、`game-view`、`game-scene-view` 与 `game-native-plan`。

## 树先解析为帧

Pixel UI 以 `UiTree<Action>` 表示页面结构。节点可包含样式、文本、图片、精灵、动作和子节点；`UiTree::resolve(UiSize)` 在纯 Rust 中计算布局、裁剪、绘制命令和命中区域，返回 `UiFrame<Action>`。

`game-ui-kit` 提供主题、面板、文字、图片、精灵、列表项、按钮、弹窗和标签栏等构建件。它们只构造 tree，不加载纹理或调用 renderer。`game-ui` 持有表现状态并投影战斗、控制台和图鉴相关页面；`game-scene-view` 将解析后的页面包装为 `SceneFrame`。

## 当前页面路径

| 页面或覆盖层 | 投影结果 | 动作类型 |
| --- | --- | --- |
| 战斗界面 | `UiFrame` | 战斗/表现动作。 |
| 图鉴 | `UiFrame<PokedexAction>` | 图鉴选择与交互。 |
| 命令控制台 | `UiFrame` | 控制台操作。 |
| 世界场景的控制台 | Grid 基础场景加 `UiFrame` 覆盖层。 | 控制台操作。 |

`game-scene-view` 在图鉴、战斗或世界路径中选择 UI frame，并可把控制台作为 overlay。UI 的开关、选择和动画状态属于 presentation；游戏事实仍来自 `GameSnapshot`。

## 从 UI frame 到 native 计划

`FramePlan::from_ui_frame` 遍历 `UiFrame` 的绘制命令，将填充、边框、图片和精灵转换为像素/GPU 计划，将文字转换为 `NativeTextLabel`。资源内容 ID 再通过 `NativeAssets` 查找 atlas 资源。缺失白色资源、未知资产、无效 UI 内容或 GPU 计划失败返回 `FramePlanError`。

这条路径在 `FramePlan` 汇合到 native 提交；`punctum-ui` 不依赖 `AssetKey`、atlas ID、WGPU 或 glyphon。
