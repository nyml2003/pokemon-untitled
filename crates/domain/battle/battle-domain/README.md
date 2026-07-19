# battle-domain

## 职责

`battle-domain` 定义第三世代风格的双人对战模型和确定性回合规则。
它校验队伍、宝可梦、招式、能力值和命令输入。
它结算招式、替换、状态、天气和胜负，并返回有序的领域事件。

## 状态契约

`Battle` 是对战状态的聚合根。
调用方先通过 `Battle::legal_actions` 查询一方可以提交的动作，再调用 `Battle::submit` 提交命令。
双方命令齐备时，对战会结算整个回合；倒下的出战成员需要在 `BattlePhase::ForcedReplacement` 中替换。

`Battle::submit` 是原子的。
非法命令返回 `BattleError`，不会改变对战状态或事件历史。
`SubmitOutcome::events` 只包含本次提交新增的事件；`Battle::events` 返回完整历史。

同一队伍输入和种子会产生相同的状态及事件序列。

## 公开 API

- 状态机：`Battle`、`BattleCommand`、`BattlePhase`、`BattleOutcome`、`SubmitOutcome`。
- 队伍与宝可梦：`Team`、`Pokemon`、`Move`、`Ability`、`PokemonType`。
- 数值与规则：`BattleStats`、`TrainingValues`、`calculate_gen3_stats`、`type_effectiveness`。
- 事件与错误：`BattleEvent`、`DamageSource`、`BattleError`、`ValidationError`、`StatProjectionError`。

模块实现保持私有。
调用方只能通过 crate 根导出的类型访问领域 API。

## 设计

[设计说明](docs/design.md) 记录模块职责、状态不变量和事件契约。

## 验证

在 workspace 根目录运行：

```sh
ops format --check
ops test --suite core
```
