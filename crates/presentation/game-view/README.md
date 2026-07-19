# game-view

## 职责

`game-view` 将世界、战斗、图鉴、命令控制台和 Game Foundation 状态投影为 `GameView` 或 `UiTree`。
它同时选择稳定的资源键和图层顺序。

## 状态契约

所有投影函数只读取传入快照。
相同输入会产生相同的视图、标签、图像和 UI 树。
`GameView::new` 要求图层按 `LayerKind` 升序排列。

## 公开 API

使用 `project_world`、`project_battle` 和 `project_console` 生成网格视图。
使用 `project_pokedex`、`project_battle_ui`、`project_console_ui` 和 `project_foundation` 生成 `punctum-ui` 树。
`project_foundation` 提供旅程、背包和训练家卡片三页，产出的 `FoundationPageAction` 只描述用户意图；宿主负责将其交给 Ramus 路由。
资源辅助函数返回 adapter 可解析的 `AssetKey`。

## 设计

详见[设计说明](docs/design.md)。

## 验证

在 workspace 根目录运行 `ops test --suite all` 和 `ops format --check`。
