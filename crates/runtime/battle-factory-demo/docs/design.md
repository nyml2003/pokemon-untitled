# battle-factory-demo 设计

## 装配

启动时加载 `CurrentDataSet::embedded`，创建 `FactorySession` 并执行一次 `StartRun`，随后加载战斗精灵资源。`NativeTarget` 在窗口创建后绑定。

## 资源刷新

对战阶段通过 `game-asset-plan::battle_asset_requests` 从当前双方队伍派生资源请求，用 `game-fs-assets` 读取并装配 `NativeAssets`。每次 `StartNextBattle` 或 `ConfirmSwap` 改变队伍后重新装配图集并调用 `NativeTarget::update_assets`；无对手时只加载 UI 蒙版与属性图标。

## 输入与回放

窗口输入先归一化为 `KeyEvent`，再映射为 `GameControl`。战斗阶段调用 `BattleUiState::handle_key`，返回 `BattleUiOutcome::Submit` 时提交对应动作；战斗结束按 A 提交 `LeaveFinishedBattle`。回放与精灵帧由真实时间计时推进，`AdvanceBattlePlayback` 只在有待播步骤时提交。

工厂菜单阶段直接由本二进制映射方向键与 A/B：

- `Ready`：A 开始下一场。
- `SwapOffer`：↑↓/←→ 选择槽位，A 交换，B 跳过。
- `Finished`：A 用新随机种子开始新一轮。

## 边界

窗口、计时、资源 I/O 与输入设备都在本壳内；业务状态与规则仍由 `battle-factory` 拥有，渲染只消费 `FactorySnapshot` 与战斗会话快照。
