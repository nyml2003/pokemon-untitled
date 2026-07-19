# map-tile-semantics

## 职责

`map-tile-semantics` 定义分层地图图块的序列化语义目录。
它校验目录覆盖率和规则，再将 `map_project::MapProject` 检查为结构化诊断。

## 状态契约

`TileSemanticsCatalog::from_json` 和 `validate` 会拒绝未知、重复、不完整或格式错误的定义。
`TileSemanticsCatalog::lint` 不修改目录或地图。
它返回检测到的所有层叠、邻接、图案、禁用图块和缺失定义违规。

## 公开 API

调用方使用 `TileSemanticsCatalog`、`TileDefinition`、`TileStatus`、matcher 和规则表达图块约束。
使用 `MapSemanticDiagnostic` 和 `SemanticRuleLocation` 呈现 lint 失败。
`TileSemanticsError` 描述无效目录输入。

## 设计

详见[设计说明](docs/design.md)。

## 验证

在 workspace 根目录运行 `ops test --suite all` 和 `ops format --check`。
