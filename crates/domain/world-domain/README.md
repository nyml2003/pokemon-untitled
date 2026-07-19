# world-domain

`world-domain` 是整数网格世界的纯领域核心。
它定义地图、角色、移动规则和世界事件。

## 职责

本 crate 负责：

- 定义地表、坐标、方向和固定尺寸地图。
- 定义玩家和 NPC 的身份、位置、朝向与阻挡关系。
- 校验世界初始状态，包括地图尺寸、角色身份、位置和占据冲突。
- 执行移动和转向，并返回新的 `World` 与 `WorldOutcome`。
- 将进入草地表示为 `WorldEvent::EncounterTriggered`。

## 状态契约

`World` 是世界状态的聚合根。
`World::transition` 只处理玩家命令。
`World::transition_actor` 只处理非玩家角色命令。
两种转换都会返回新状态，不会修改原 `World`。

玩家从非草地进入草地时触发遭遇事件。
NPC 进入草地不会触发遭遇。
被边界、墙体或阻挡角色挡住的移动会更新朝向，但不会改变位置。

## 公开 API

- 地图模型：`Tile`、`Position`、`Direction`、`TileMap`。
- 角色模型：`WorldActorId`、`WorldActor`。
- 命令和事件：`WorldCommand`、`WorldActorCommand`、`WorldEvent`、`WorldOutcome`。
- 聚合根和错误：`World`、`WorldError`。

模块实现保持私有。
调用方只能通过 crate 根导出的类型访问领域 API。

## 设计

[设计思路](docs/design.md) 说明模型划分、状态转换、不变量和可见性取舍。

## 验证

```sh
cargo test -p world-domain
cargo doc -p world-domain --no-deps
```
