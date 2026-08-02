# battle-factory

## 职责

定义并执行对战工厂模式的确定性状态机：租借固定等级队伍、连胜计数、战后交换与败局结束。它只拥有工厂规则；真实战斗委托给 `battle-session`，随机队伍生成委托给 `game-session::roster`。

## 状态契约

外部只能经 `FactorySession::transition` 改变会话。会话持有租借队伍、对手队伍、可选战斗会话、连胜数、目标连胜数与阶段。

阶段：

| 阶段 | 含义 | 可接受命令 |
| --- | --- | --- |
| `Ready` | 未开始或两次对战之间 | `StartRun`、`StartNextBattle` |
| `Battle` | 对战中 | `SubmitBattleAction`、`AdvanceBattlePlayback`、`LeaveFinishedBattle` |
| `SwapOffer` | 胜利后的交换选择 | `ConfirmSwap`、`SkipSwap` |
| `Finished` | 失败或清关 | `StartRun` |

战后租借队伍按原始形态和招式治愈；换宠只替换对应槽位，并保证队伍始终为六人。`ConfirmSwap` 与 `SkipSwap` 只能在 `SwapOffer` 阶段使用，`SubmitBattleAction` 在回放或已结束时返回 `PlayerActionUnavailable`。

## 公开 API

- `FactorySession::new`、`transition`、`snapshot`、`sprite_manifest`、`legal_player_actions`。
- `FactoryCommand`：开始一轮、开始下一场、战斗动作、回放推进、结算、交换、跳过。
- `FactoryEvent`：`RunStarted`、`BattleStarted`、`BattleActionSubmitted`、`BattlePlaybackAdvanced`、`BattleResolved`、`SwapApplied`、`RunEnded`。
- `FactoryError`：阶段错误、战斗缺失、非法槽位、目标连胜为 0 等类型化错误。

## 设计

见 [设计](docs/design.md)。

## 验证

```text
ops test --suite core
```
