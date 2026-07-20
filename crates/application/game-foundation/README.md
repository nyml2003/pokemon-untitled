# game-foundation

## 职责

定义薄切片的静态内容、训练师目录、动态游戏状态、确定性命令转换、背包/经济规则和版本化存档 envelope。

## 状态契约

`GameState::transition` 在命令失败时返回原状态。随机遭遇由 `Encounter { roll }` 的显式输入决定。HP、PP、经验、金钱、背包、事件 flag、训练师完成状态和进行中战斗都属于动态状态。

## 公开 API

`TrainerCatalog` 定义训练师姓名、最多六只宝可梦和交互脚本；`TrainerEditCommand` 以纯状态转换编辑这些字段。
`ThinSliceContent` 以稳定 ID 查询地图、warp、NPC capability、物品、商店、战斗、训练师和精灵模板。训练师 NPC 同时引用战斗奖励和训练师目录中的队伍。静态地图布局复用 `world-domain`；`GameState` 只保存地图 ID、位置和朝向，并在移动时重建世界 reducer。`SaveEnvelope` 只转换字节，不读写文件，并在写入和重载时验证所有动态内容引用。

旧的薄切片专用内容 getter 不属于此 crate 的稳定 API；调用方应持有稳定 ID 并查询 `ThinSliceContent`。

## 设计

状态、命令和保存格式的边界见 [docs/design.md](docs/design.md)。

## 验证

`ops format --check`、`ops lint`、`ops test --suite all`、`ops docs check`。
