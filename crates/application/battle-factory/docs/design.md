# battle-factory 设计

## 职责划分

会话是唯一聚合，持有租借队伍、对手队伍、战斗会话与连胜状态。渲染与输入不在本 crate；窗口和键盘由 `battle-factory-demo` 承担。

队伍构造复用 `game-session::roster::random_team`，保证相同种子产生相同队伍。每场对手种子由 `run_seed ^ streak * 常量` 派生，因此一轮内的对手序列确定。

## 状态机与不变量

阶段转换由命令驱动，非法命令返回 `FactoryError::WrongPhase` 且不改变状态：

- `StartRun(seed, target_streak)` 只在 `Ready` 或 `Finished` 阶段可用，并要求 `target_streak > 0`；成功后重置连胜与队伍。
- `StartNextBattle` 只在 `Ready` 可用，用当前种子生成对手并构造 `BattleSession`。
- `LeaveFinishedBattle` 只在战斗已结束时可用；胜利则连胜加一，达到目标连胜后进入 `Finished`（清关），否则进入 `SwapOffer`；失败或平局（`Draw`）都按失败处理，直接进入 `Finished`，连胜不增加。
- `ConfirmSwap` 要求 `SwapOffer`，槽位必须在有效范围内；替换后只对替换进来的宝可梦治愈。
- `SkipSwap` 保留队伍回到 `Ready`。

对手种子由 `run_seed ^ (streak + 1) * 常量` 派生，保证首场对手与租借队伍不同，且一轮内的对手序列确定。

保持队伍六人的不变量由 `Team::new` 与交换槽位校验共同保证；`sprite_manifest` 只在对手已生成时返回 `Some`。交换菜单中展示的对手成员来自生成时的满状态队伍，代表可交换的满状态副本，不是战斗中的实时伤势。

## 命令、事件与错误

命令、事件与错误都是类型化值，不携带字符串状态。事件在 `FactoryEvents` 中按发生顺序暴露；展示层据其刷新回放计时与菜单。错误枚举覆盖阶段、战斗与槽位三类普通失败，`Result` 为 `Err` 时返回原会话，不丢失已有战斗。

## 模块可见性

`FactoryOpponentPolicy` 与 `heal_unit` 为私有实现；根只导出会话、命令、事件、快照、成员摘要与错误。
