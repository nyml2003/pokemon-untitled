# Grid 路径

> 分类：现状；最后核对：2026-07-20。
> 依据：`punctum-grid`、`map-render`、`game-view`、`map-editor-view` 与 `game-native-plan`。

## Grid 表达地图与编辑器

Grid 路径用于世界地图和地图编辑器。`punctum-grid` 提供离散位置、矩形、尺寸和稠密 `Surface`；`map-render` 用地图项目、原子图块目录、相机、像素偏移和 `MapGridLayout` 生成 `MapScenePlan`。

`MapScenePlan` 产出地图图层和图块图像。`game-view` 再将地图层、角色、HUD 和文本组织成 `GameView`/`ViewLayer`。世界场景由 `game-scene-view` 调用这条链路；`map-editor-view` 同样使用 `map-render`，但追加编辑器专用的选择、工具和语义可视化。

## 视口与图层

Grid 的坐标是离散单元。`Viewport` 把目标像素尺寸、原点和单元像素尺寸关联起来，`MapGridLayout` 决定 map surface 与一块图块占用的 Grid span。世界投影根据玩家位置构造相机；编辑器根据编辑器视图范围构造相机和 tile span。

`GameView` 可以包含多个 `ViewLayer`、surface、图像和文本标签。它仍是纯表示，不拥有窗口、GPU device 或图像文件。地图和编辑器因此共享同一个图块投影模型，同时保留不同的上层图层。

## 到帧计划的边界

`FramePlan::from_game_view` 将 `GameView`、`NativeAssets`、viewport 和文本缩放转换为 GPU submission pass。它验证 surface 的存在与尺寸、引用资产和图层一致性，然后生成 GPU 数据和文本标签。WGPU 提交位于下一层的 `NativeTarget`。
