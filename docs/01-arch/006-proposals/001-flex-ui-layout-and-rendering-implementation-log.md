# Flex UI 实施记录

> 更新：2026-07-16
> 状态：第二阶段进行中

## 当前结论

图鉴已经迁移到 `punctum-ui`。它不再生成固定 32×24 的 `GameView` 或 `Surface`。

战斗和命令控制台的迁移尚未完成。它们的最终视觉必须与当前运行版本一致。不能借 Flex 重构改变战场构图、状态卡位置、精灵层级、菜单布局、颜色或信息密度。

大地图和地图编辑器继续使用 Grid。它们不属于本次迁移范围。

## 已完成

| 位置 | 已完成内容 |
| --- | --- |
| `crates/foundation/punctum-ui` | 纯 Rust UI 树、行列 Flex、叠放、命中测试、矩形裁剪、margin、border、padding 和 `border_radius` 数据模型。布局 crate 不依赖 `punctum-grid` 或 GPU。 |
| `crates/foundation/punctum-gpu` | `GpuPixelImage` 和 `plan_pixels`。像素页面无需分配 Grid `Surface`。 |
| `crates/presentation/game-native-plan` | `FramePlan::from_ui_frame`。它将 UI 绘制命令转换为图集资源、像素实例和 native 文本标签。圆角填充使用 `ui/rounded-rect` 遮罩资源。 |
| 图鉴 | `project_pokedex` 返回 `UiTree`。`game-scene-view` 解析为 `SceneFrame::Ui`，`game-host` 走 `FramePlan::from_ui_frame`。 |
| 测试 | 此前已运行 `cargo test --workspace` 并通过；GPU headless smoke 和完整游戏图集检查仍是已有的 ignored 测试。 |

## 当前工作区状态

下面的改动正在进行，尚未验收：

1. `docs/01-arch/006-proposals/001-flex-ui-layout-and-rendering.md` 已开始补充盒子模型、圆角和非地图页面迁移范围。
2. `UiStyle` 已增加 `margin`、`border`、`padding`、`border_radius`。
3. `UiDrawCommand::Fill` 和 `UiDrawCommand::Image` 已携带圆角信息；native plan 已将圆角填充映射到圆角遮罩。
4. 已新增 `project_battle_ui` 与 `project_console_ui` 的第一版，但它们改变了既有视觉，不能作为最终实现。
5. `game-scene-view` 当前临时把战斗和控制台路由到这两个第一版 UI。继续工作前必须替换为视觉等价方案。

## 不可违反的约束

- `punctum-ui` 继续是纯布局和绘制命令 crate。不能引入 `GridRect`、`Surface`、`wgpu`、窗口或资产加载。
- `game-native-plan` 是 UI 内容 ID 到 GPU 资源 ID 的边界。资源名称和图集细节不能漏进 `punctum-ui`。
- `GameSession` 继续拥有业务事实。`PresentationState` 只拥有菜单、焦点、动画和输入瞬态状态。
- 大地图和地图编辑器继续走原来的 Grid 提交路径。
- 图鉴、战斗、控制台最终走 Pixel UI 路径。
- 页面迁移必须保留现有视觉。布局更灵活不等于可以重新设计页面。

## 正确的后续方案

需要补“逻辑画布坐标”能力，而不是重画战斗页面。

1. 在 `punctum-ui` 增加与 Grid 无关的逻辑画布定位：按父 viewport 的比例将逻辑 `x`、`y`、`width`、`height` 映射为像素 `UiRect`。
2. 逻辑画布只使用 `UiSize`、整数比例和 `UiRect`。它不引用格子、`GridRect` 或 `Surface`。
3. `game-view` 用现有战斗和控制台的视觉规格生成 `UiNode`。背景、精灵、状态卡、文字和菜单按原有 32×24 画面比例放置。
4. `project_battle_ui` 和 `project_console_ui` 必须以现有 `project_battle` / `project_console` 的输出作为视觉 oracle。迁移测试应比较元素数量、资源键、相对矩形、绘制顺序和文字内容。
5. 地图上的控制台需要 Grid/Pixel 同帧合成。不能为了绕开合成而把地图背景替换成新的纯色页面。
6. 完成视觉等价的 Pixel UI 后，才切换 `game-scene-view` 运行时路由，并删除非迁移路径。

## 需要补的能力

| 能力 | 目的 | 所属边界 |
| --- | --- | --- |
| 逻辑画布定位 | 保留旧页面的构图，同时输出响应式像素矩形 | `punctum-ui` |
| 圆角图片裁剪 | 让 `border_radius` 同时作用于图片，而非只作用于填充 | `punctum-gpu` / `punctum-wgpu` |
| 像素绘制批次与 scissor | 正确处理每个 UI 命令的裁剪 | `punctum-gpu` / `punctum-wgpu` |
| Grid + Pixel 合成帧 | 地图上叠加控制台、HUD 和后续轻量面板 | `game-native-plan` / native adapter |
| 视觉等价测试 | 防止迁移改变既有页面设计 | `game-view` / `game-scene-view` 测试 |

## 验收标准

战斗和控制台迁移完成前，至少满足：

- 同一业务快照下，旧投影和新 UI 投影具有相同的文字、资源键、相对位置和稳定绘制顺序。
- 战斗精灵、状态卡、HP 条、类型图标、行动菜单和消息区保持原有层级。
- 控制台保持地图覆盖层，而不是替换地图场景。
- `border_radius` 在填充、边框和图片上都可用，半径被限制在外框短边的一半。
- `cargo test --workspace` 通过。

