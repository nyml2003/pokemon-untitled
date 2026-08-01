# battle-domain 实体模型

## 定位与边界

`battle-domain` 承载对战领域的**实体数据模型**：对战单位、物种数据、状态、对战聚合与值对象。本层是一层数据模型，不承载战斗计算逻辑。

战斗计算逻辑——特性对属性的修正、属性克制倍率、伤害公式、异常状态结算与回合推进——不属于本层，由上层规则层基于本层数据计算。本层只保证数据完整与类型安全。

## 核心实体：BattleUnit

```rust
/// 战斗单位的唯一标识，区分同种族的个体。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleUnitId(String);

/// 注入的物种数据，不可变，由装配层从图鉴/种族数据转换后拷贝注入。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Species {
    /// 物种显示名。
    pub name: String,
    /// 六维种族值：HP、攻击、防御、特攻、特防、速度。
    pub base_stats: StatBlock<u16>,
    /// 全国图鉴编号。
    pub national_dex_id: NationalDexId,
    /// 变体编号。
    pub form_id: FormId,
    /// 属性列表。
    pub types: Vec<PokemonType>,
    /// 默认特性列表。
    pub default_abilities: Vec<Ability>,
}

/// 一只进入战斗的宝可梦：注入的物种数据 + 个体标识 + 可变状态。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleUnit {
    /// 注入的物种数据，只读。
    species: Species,
    /// 个体标识。
    id: BattleUnitId,
    /// 可变战斗状态。
    state: BattleState,
}

/// 战斗中一切可能变化的数据。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleState {
    /// 当前等级。
    level: u8,
    /// 最终战斗能力值。
    stats: BattleStats,
    /// 最大 HP。
    max_hp: u32,
    /// 当前 HP。
    current_hp: u32,
    /// 已携带招式（含剩余 PP）。
    moves: Vec<Move>,
    /// 当前生效的特性。
    ability: Vec<Ability>,
    /// 主要异常状态（含剩余回合）。
    major_status: Option<MajorStatus>,
    /// 能力阶级。
    stages: StatStages,
    /// 临时状态容器（替身、连续守住等）。
    volatile_statuses: VolatileStatuses,
}

/// 一种临时状态，只在生效时存在。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VolatileStatus {
    /// 替身及其剩余 HP。
    Substitute { remaining_hp: u32 },
    /// 连续守住计数。
    ProtectStreak { count: u8 },
    // 束缚、混乱、蓄力等在此扩展
}

/// 通用临时状态容器。新增临时状态只加枚举变体，不改变 `BattleState` 的结构。
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct VolatileStatuses(Vec<VolatileStatus>);
```

标识类型：

```rust
/// 全国图鉴编号。
pub struct NationalDexId(u16);
/// 形态/变体编号。
pub struct FormId(u32);
/// 宝可梦标识。
pub struct PokemonId(String);
/// 招式标识。
pub struct MoveId(String);
/// 招式槽位（0 至 3）。
pub struct MoveSlot(u8);
/// 队伍槽位（0 至 5）。
pub struct TeamSlot(u8);
```

## 聚合

```rust
/// 一场对局。
pub struct Battle {
    /// 双方队伍。
    teams: [Team; 2],
    /// 双方当前出战槽位。
    active: [TeamSlot; 2],
    /// 当前命令阶段。
    phase: BattlePhase,
    /// 双方待处理命令。
    pending: [Option<PendingCommand>; 2],
    /// 确定性伪随机源。
    rng: DeterministicRng,
    /// 从一递增的回合编号。
    turn: u32,
    /// 从开始累积的有序事件历史。
    events: Vec<BattleEvent>,
    /// 各阵营本回合畏缩标记。
    flinched: [bool; 2],
    /// 各阵营本回合守住标记。
    protected: [bool; 2],
    /// 各阵营引火已触发标记。
    flash_fire: [bool; 2],
    /// 当前天气及其剩余回合。
    weather: Option<WeatherState>,
}

/// 恰好六只 BattleUnit 的对战队伍，个体标识互不重复。
pub struct Team {
    /// 六名成员。
    members: [BattleUnit; TEAM_SIZE],
}
```

对战状态由行为方法推进，事件是不可变的有序事实记录。

## 命令与事件

```rust
/// 一方在当前阶段可提交的动作。
pub enum Action {
    /// 使用指定槽位的招式。
    UseMove(MoveSlot),
    /// 换入后备成员。
    Switch(TeamSlot),
    /// 逃跑。
    Run,
    /// 使用挣扎。
    Struggle,
}

/// 一方提交给对战状态机的命令。
pub struct BattleCommand {
    /// 提交命令的阵营。
    side: Side,
    /// 要执行的动作。
    action: Action,
}

/// 当前接受的命令类型。
pub enum BattlePhase {
    /// 正常回合。
    Turn,
    /// 需强制替换倒下的出战成员。
    ForcedReplacement(ReplacementSides),
    /// 对战已结束。
    Finished(BattleOutcome),
}

/// 需强制替换的阵营集合。
pub enum ReplacementSides {
    /// 仅一方。
    One,
    /// 仅另一方。
    Two,
    /// 双方。
    Both,
}

/// 对战结果。
pub enum BattleOutcome {
    /// 一方获胜。
    Winner(Side),
    /// 一方逃走。
    Escaped(Side),
    /// 平局。
    Draw,
}

/// 单次提交的增量结果。
pub struct SubmitOutcome {
    /// 本次提交新增的事件。
    events: Vec<BattleEvent>,
    /// 处理后的对战阶段。
    phase: BattlePhase,
    /// 是否仍在等待对方提交命令。
    waiting_for_opponent: bool,
}

/// 待结算的命令与替换候选（内部）。
struct PendingCommand {
    /// 待结算命令。
    command: BattleCommand,
    /// 可选的替换候选。
    replacement: Option<TeamSlot>,
}
```

事件记录包括：`UsedMove`（实际出招）、`DamageSource`（伤害来源：招式/反伤/特性/异常/天气）、`BattleEvent`（有序事实记录：命令接受、替换、伤害、状态施加、天气变化等）。

## 值对象与枚举

```rust
/// 招式。
pub struct Move {
    /// 招式标识。
    id: MoveId,
    /// 招式名。
    name: String,
    /// 招式属性列表。
    move_types: Vec<PokemonType>,
    /// 物理 / 特殊 / 变化。
    category: MoveCategory,
    /// 威力。
    power: u16,
    /// 命中率。
    accuracy: Accuracy,
    /// 最大 PP。
    max_pp: u8,
    /// 当前 PP。
    current_pp: u8,
    /// 优先度。
    priority: i8,
    /// 附加效果列表。
    effects: Vec<MoveEffect>,
    /// 天气对命中的修正标记。
    weather_accuracy: Option<WeatherAccuracyModifier>,
    /// 天气对招式威力与属性的修正标记。
    weather_move: Option<WeatherMoveModifier>,
}

/// 五项战斗能力值。
pub struct BattleStats {
    /// 攻击。
    attack: u16,
    /// 防御。
    defense: u16,
    /// 特攻。
    special_attack: u16,
    /// 特防。
    special_defense: u16,
    /// 速度。
    speed: u16,
}
```

其余值对象与枚举：`Side`、`PokemonType`、`MoveCategory`、`MoveEffect`、`Accuracy`、`Ability`、`MajorStatus`、`MajorStatusKind`、`Weather`、`WeatherState`、`WeatherAccuracyModifier`、`WeatherMoveModifier`、`BattleStat`、`StatStages`、`StageChanges`、`EffectTarget`、`FixedDamage`。全部不可变、按值比较。

## 能力值类型

```rust
/// 通用六项能力值块。
pub struct StatBlock<T> {
    /// HP。
    pub hp: T,
    /// 攻击。
    pub attack: T,
    /// 防御。
    pub defense: T,
    /// 特攻。
    pub special_attack: T,
    /// 特防。
    pub special_defense: T,
    /// 速度。
    pub speed: T,
}

/// 个体值（六项，各不超过 31）。
pub struct IndividualValues(StatBlock<u8>);

/// 努力值（六项，单项不超过 255，总和不超过 510）。
pub struct EffortValues(StatBlock<u8>);

/// 性格（能力修正）：可提高一项、降低另一项，或中性。
pub struct Nature {
    /// 提高的能力项。
    raised: Option<NonHpStat>,
    /// 降低的能力项。
    lowered: Option<NonHpStat>,
}

/// 参与能力值投影的个体值、努力值和性格。
pub struct TrainingValues {
    /// 个体值。
    ivs: IndividualValues,
    /// 努力值。
    evs: EffortValues,
    /// 性格。
    nature: Nature,
}

/// 从基础种族值投影出的最大 HP 和战斗能力值。
pub struct CalculatedStats {
    /// 最大 HP。
    max_hp: u32,
    /// 五项战斗能力值。
    battle: BattleStats,
}
```

## 校验边界

非法数据在进入本层的边界校验：装配层构造 `Species` 与 `BattleUnit` 时，等级范围、HP 上限、招式数量与标识唯一性由构造入口保证，返回类型化错误。`Battle` 构造时校验队伍存活与跨队标识唯一。

## 计算逻辑归属

以下逻辑**不属于本层**：

- 特性对属性、能力值的修正（如攻击特性加成）
- 属性克制倍率
- 伤害公式
- 异常状态结算与回合推进

它们由上层规则层基于本层的数据实体计算。本层实体不携带这些逻辑，保持"一坨数据"的纯粹形态。
