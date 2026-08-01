use crate::{
    Ability, BattleStat, BattleUnit, BattleUnitId, MajorStatus, MajorStatusKind, MoveId, MoveSlot,
    Side, Team, TeamSlot, TypeEffectiveness, Weather, WeatherState,
};

/// 一方在当前对战阶段可以提交的动作。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    UseMove(MoveSlot),
    Switch(TeamSlot),
    Run,
    Struggle,
}

/// 一方提交给对战状态机的动作。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BattleCommand {
    side: Side,
    action: Action,
}

impl BattleCommand {
    /// 创建指定阵营的一条命令。
    pub const fn new(side: Side, action: Action) -> Self {
        Self { side, action }
    }

    /// 提交命令的阵营。
    pub const fn side(&self) -> Side {
        self.side
    }

    /// 要执行的动作。
    pub const fn action(&self) -> Action {
        self.action
    }
}

/// 已结束对战的结果。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BattleOutcome {
    Winner(Side),
    Escaped(Side),
    Draw,
}

/// 对战状态机当前接受的命令类型。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BattlePhase {
    Turn,
    ForcedReplacement(ReplacementSides),
    Finished(BattleOutcome),
}

impl BattlePhase {
    /// 返回此阶段是否要求指定阵营先替换倒下的出战宝可梦。
    pub const fn requires_replacement(self, side: Side) -> bool {
        match self {
            Self::ForcedReplacement(sides) => sides.contains(side),
            Self::Turn | Self::Finished(_) => false,
        }
    }
}

/// 需要在强制替换阶段提交换人命令的阵营集合。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplacementSides {
    One,
    Two,
    Both,
}

impl ReplacementSides {
    /// 返回指定阵营是否在此集合中。
    pub const fn contains(self, side: Side) -> bool {
        match self {
            Self::One => matches!(side, Side::One),
            Self::Two => matches!(side, Side::Two),
            Self::Both => true,
        }
    }
}

/// 事件中记录的实际出招。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UsedMove {
    Move { slot: MoveSlot, id: MoveId },
    Struggle,
}

/// 伤害事件的直接来源。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DamageSource {
    Move {
        side: Side,
        pokemon: BattleUnitId,
        used_move: UsedMove,
    },
    Recoil {
        side: Side,
        pokemon: BattleUnitId,
    },
    Ability {
        side: Side,
        pokemon: BattleUnitId,
        ability: Ability,
    },
    Status {
        side: Side,
        pokemon: BattleUnitId,
        status: MajorStatus,
    },
    Weather {
        weather: Weather,
    },
}

/// 对战结算产生的有序事实记录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BattleEvent {
    CommandAccepted {
        side: Side,
        action: Action,
    },
    TurnStarted {
        turn: u32,
    },
    Switched {
        side: Side,
        from: TeamSlot,
        to: TeamSlot,
        pokemon: BattleUnitId,
        current_hp: u32,
    },
    MoveUsed {
        side: Side,
        pokemon: BattleUnitId,
        used_move: UsedMove,
    },
    PpSpent {
        side: Side,
        pokemon: BattleUnitId,
        move_slot: MoveSlot,
        remaining: u8,
    },
    StatusApplied {
        side: Side,
        pokemon: BattleUnitId,
        status: MajorStatus,
    },
    StatusFailed {
        side: Side,
        target_side: Side,
        target: BattleUnitId,
        status: MajorStatusKind,
    },
    StatusPreventsAction {
        side: Side,
        pokemon: BattleUnitId,
        status: MajorStatus,
    },
    StatusCured {
        side: Side,
        pokemon: BattleUnitId,
        status: MajorStatusKind,
    },
    StatusAdvanced {
        side: Side,
        pokemon: BattleUnitId,
        status: MajorStatus,
    },
    ProtectionActivated {
        side: Side,
        pokemon: BattleUnitId,
    },
    ProtectionFailed {
        side: Side,
        pokemon: BattleUnitId,
    },
    MoveBlocked {
        side: Side,
        target_side: Side,
        target: BattleUnitId,
    },
    SubstituteCreated {
        side: Side,
        pokemon: BattleUnitId,
        substitute_hp: u32,
        current_hp: u32,
    },
    SubstituteBlocked {
        side: Side,
        target_side: Side,
        target: BattleUnitId,
    },
    SubstituteDamaged {
        side: Side,
        pokemon: BattleUnitId,
        amount: u32,
        remaining_hp: u32,
    },
    SubstituteBroke {
        side: Side,
        pokemon: BattleUnitId,
    },
    WeatherStarted {
        weather: Weather,
        turns_remaining: Option<u8>,
    },
    WeatherUpdated {
        weather: Weather,
        turns_remaining: u8,
    },
    WeatherEnded {
        weather: Weather,
    },
    AbilityActivated {
        side: Side,
        pokemon: BattleUnitId,
        ability: Ability,
    },
    Flinched {
        side: Side,
        pokemon: BattleUnitId,
    },
    StatStageChanged {
        side: Side,
        pokemon: BattleUnitId,
        stat: BattleStat,
        change: i8,
        stage: i8,
    },
    Healed {
        side: Side,
        pokemon: BattleUnitId,
        amount: u32,
        current_hp: u32,
    },
    EffectFailed {
        side: Side,
        target_side: Side,
        target: BattleUnitId,
    },
    Missed {
        side: Side,
        target_side: Side,
        target: BattleUnitId,
    },
    Critical {
        side: Side,
        target_side: Side,
        target: BattleUnitId,
    },
    Effectiveness {
        side: Side,
        target_side: Side,
        target: BattleUnitId,
        effectiveness: TypeEffectiveness,
    },
    Damage {
        source: DamageSource,
        target_side: Side,
        target: BattleUnitId,
        amount: u32,
        remaining_hp: u32,
    },
    Fainted {
        side: Side,
        pokemon: BattleUnitId,
    },
    ForcedReplacement {
        side: Side,
    },
    BattleFinished {
        outcome: BattleOutcome,
    },
}

/// 动作在当前状态下不合法的具体原因。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IllegalActionReason {
    WrongPhase,
    MoveDoesNotExist,
    MoveHasNoPp,
    StruggleNotRequired,
    SwitchToActive,
    SwitchTargetFainted,
    SwitchPrevented,
    StateInconsistent,
}

/// 构造或推进对战时违反的领域规则。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BattleError {
    NoLivingPokemon {
        side: Side,
    },
    DuplicatePokemonId {
        id: BattleUnitId,
    },
    CommandAlreadySubmitted {
        side: Side,
    },
    ActionNotLegal {
        side: Side,
        action: Action,
        reason: IllegalActionReason,
    },
    BattleAlreadyFinished {
        outcome: BattleOutcome,
    },
    StateInconsistent {
        detail: &'static str,
    },
}

/// 单次成功提交后产生的增量结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubmitOutcome {
    pub events: Vec<BattleEvent>,
    pub phase: BattlePhase,
    pub waiting_for_opponent: bool,
}

impl SubmitOutcome {
    /// 返回本次提交新增的事件。
    pub fn events(&self) -> &[BattleEvent] {
        &self.events
    }

    /// 返回命令处理后的对战阶段。
    pub const fn phase(&self) -> BattlePhase {
        self.phase
    }

    /// 返回对战是否仍在等待另一方提交命令。
    pub fn is_waiting_for_opponent(&self) -> bool {
        self.waiting_for_opponent
    }
}

/// 维护回合、队伍和事件历史的确定性双人对战数据。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Battle {
    /// 双方队伍。
    pub teams: [Team; 2],
    /// 双方当前出战槽位。
    pub active: [TeamSlot; 2],
    /// 当前命令阶段。
    pub phase: BattlePhase,
    /// 双方待处理命令。
    pub pending: [Option<PendingCommand>; 2],
    /// 确定性伪随机源。
    pub rng: DeterministicRng,
    /// 从一递增的回合编号。
    pub turn: u32,
    /// 从开始累积的有序事件历史。
    pub events: Vec<BattleEvent>,
    /// 各阵营本回合畏缩标记。
    pub flinched: [bool; 2],
    /// 各阵营本回合守住标记。
    pub protected: [bool; 2],
    /// 各阵营引火已触发标记。
    pub flash_fire: [bool; 2],
    /// 当前天气及其剩余回合。
    pub weather: Option<WeatherState>,
}

impl Battle {
    /// 用两支各含存活成员的队伍和确定性种子创建对战。
    ///
    /// 两队不得包含相同的 `BattleUnitId`。
    pub fn new(team_one: Team, team_two: Team, seed: u64) -> Result<Self, BattleError> {
        let active_one = team_one
            .first_living_slot()
            .ok_or(BattleError::NoLivingPokemon { side: Side::One })?;
        let active_two = team_two
            .first_living_slot()
            .ok_or(BattleError::NoLivingPokemon { side: Side::Two })?;
        for first in team_one.members() {
            if team_two
                .members()
                .iter()
                .any(|second| first.id() == second.id())
            {
                return Err(BattleError::DuplicatePokemonId {
                    id: first.id().clone(),
                });
            }
        }
        Ok(Self {
            teams: [team_one, team_two],
            active: [active_one, active_two],
            phase: BattlePhase::Turn,
            pending: [None, None],
            rng: DeterministicRng::new(seed),
            turn: 1,
            events: Vec::new(),
            flinched: [false; 2],
            protected: [false; 2],
            flash_fire: [false; 2],
            weather: None,
        })
    }

    /// 返回当前需要提交的命令阶段。
    pub const fn phase(&self) -> BattlePhase {
        self.phase
    }

    /// 返回从一开始递增的当前回合编号。
    pub const fn turn_number(&self) -> u32 {
        self.turn
    }

    /// 返回对战开始以来累积的全部事件。
    pub fn events(&self) -> &[BattleEvent] {
        &self.events
    }

    /// 返回当前天气及其剩余回合数。
    pub const fn weather(&self) -> Option<WeatherState> {
        self.weather
    }

    /// 返回指定阵营的完整队伍。
    pub fn team(&self, side: Side) -> &Team {
        &self.teams[side_index(side)]
    }

    /// 返回指定阵营当前出战成员的队伍槽位。
    pub fn active_slot(&self, side: Side) -> TeamSlot {
        self.active[side_index(side)]
    }

    /// 返回指定阵营当前出战的战斗单位。
    pub fn active(&self, side: Side) -> &BattleUnit {
        self.team(side).member(self.active_slot(side))
    }
}

/// 待结算的命令与替换候选。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingCommand {
    /// 待结算命令。
    pub command: BattleCommand,
    /// 可选的替换候选。
    pub replacement: Option<TeamSlot>,
}

const fn side_index(side: Side) -> usize {
    match side {
        Side::One => 0,
        Side::Two => 1,
    }
}

/// 确定性伪随机源，由种子推进状态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// 推进并返回下一个伪随机值。
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    /// 返回闭区间内的伪随机值。
    pub fn range_inclusive(&mut self, minimum: u64, maximum: u64) -> u64 {
        debug_assert!(minimum <= maximum);
        minimum + self.next_u64() % (maximum - minimum + 1)
    }

    /// 返回一个伪随机布尔值。
    pub fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 0
    }
}
