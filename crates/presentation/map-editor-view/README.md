# map-editor-view

## 职责

`map-editor-view` 将 `EditorModel`、原子图块目录和 viewport 投影为 `EditorFrame`。
它生成地图图层、工作台 UI 树和 UI 命中到 `EditorIntent` 的映射。

## 状态契约

`project` 不修改 `EditorModel` 或资源目录。
编辑操作不会在视图层执行；`intent_for_ui_hit` 仅返回调用方可交给 reducer 的意图。

## 公开 API

使用 `project` 构建完整工作台。
使用 `centered_map_viewport` 和 `editor_viewport` 计算显示区域。
使用 `EditorViewError` 区分地图、视图表面和 UI 构建失败。

## 设计

详见[设计说明](docs/design.md)。

## 验证

在 workspace 根目录运行 `ops test --suite all` 和 `ops format --check`。
