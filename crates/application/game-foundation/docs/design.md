# 设计

`GameState` 是动态状态聚合。它不持有静态 NPC、地图、战斗、商店或物品定义；这些定义由 `ThinSliceContent` 提供。NPC 使用可组合 capability：Gift、Trainer、Merchant 和 Guide 不是互斥的角色类型。

训练师的姓名、队伍和交互脚本属于静态 `TrainerCatalog`。目录校验名称、非空脚本、1 至 6 只宝可梦和 1 至 100 的等级。`TrainerEditCommand` 以克隆候选目录的方式应用编辑，失败时不返回部分修改。Trainer NPC 通过 `TrainerId` 引用目录，`GameState::Interact` 在开始训练师战前确认该定义存在。

地图静态布局使用 `world-domain::TileMap`。动态状态只保存当前地图、位置和朝向；每次 `Move` 由内容布局和当前地图 NPC 重建 `world-domain::World` 后执行。因此碰撞、边界、NPC 阻挡和进入草丛的规则不会在游戏基座中复制。进入草丛只产生待遭遇状态，后续 `Encounter { roll }` 负责携带可复现随机输入并开始战斗。

每条 `GameCommand` 在克隆的候选状态上执行。成功才返回候选状态，因此失败不会部分修改原状态。

`Inventory` 维护格子容量和按物品定义计算的堆叠上限；`Money` 对余额不足和溢出返回显式错误。购买在同一候选状态中先计算价格、扣款和入包，因此外部可见的失败仍保持原状态。

`SaveEnvelope` 包含格式版本、内容版本、状态和基于规范 JSON 的完整性校验。读写两侧均调用 `GameState::validate`，拒绝未知内容 ID、非法 HP/PP、非法背包条目、未知 flag 和不合法训练师完成状态。序列化不访问文件系统，文件读写由 runtime 入口负责。
