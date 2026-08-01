# battle-domain

## 职责

`battle-domain` 承载第三世代风格双人对战的**实体数据模型**：战斗单位、物种数据、战斗状态、招式、能力值与值对象。

本层是一层数据模型，不承载战斗计算逻辑。属性克制、伤害公式、状态结算与回合推进由上层规则层（`battle-application` 引擎）基于本层数据实现。

## 数据契约

`Battle` 是对战数据的聚合容器：双方队伍、出战槽位、阶段、待处理命令、确定性随机源、回合、事件历史与天气。

`BattleUnit` 是一只进入战斗的宝可梦，由注入的 `Species`（不可变）+ `BattleUnitId` + 可变的 `BattleState` 组成。

推进对战状态（`submit`/`legal_actions`）由上层引擎提供：`battle_application::submit_battle` / `legal_actions_battle`。`BattleEvent` 是有序事实记录，调用方应将其作为展示或持久化的数据。

## 公开 API

- 对战单位：`BattleUnit`、`Species`、`BattleState`、`BattleUnitId`、`VolatileStatus`、`VolatileStatuses`、`NationalDexId`、`FormId`
- 对战数据：`Battle`、`Team`、`BattleCommand`、`BattlePhase`、`BattleOutcome`、`BattleEvent`、`SubmitOutcome`
- 值对象与枚举：`Move`、`Ability`、`PokemonType`、`MajorStatus`、`Weather`、`BattleStats`、`TypeEffectiveness`
- 能力值：`StatBlock`、`IndividualValues`、`EffortValues`、`Nature`、`TrainingValues`、`CalculatedStats`、`calculate_gen3_stats`
- 错误：`BattleError`、`ValidationError`、`StatProjectionError`

模块实现保持私有。调用方只能通过 crate 根导出的类型访问。

## 设计

[设计说明](docs/design.md) 记录模块职责、状态不变量和数据契约。
[实体模型](docs/entities.md) 定义对战单位、物种数据、状态与值对象的实体边界。

## 验证

在 workspace 根目录运行：

```sh
ops format --check
ops test --suite core
```
