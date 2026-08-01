use crate::error::ValidationError;
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Side {
    One,
    Two,
}

impl Side {
    /// 返回另一支队伍的标识。
    pub const fn opponent(self) -> Self {
        match self {
            Self::One => Self::Two,
            Self::Two => Self::One,
        }
    }
}

/// 队伍内零基且已验证的成员位置。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PokemonType {
    Normal,
    Fighting,
    Flying,
    Poison,
    Ground,
    Rock,
    Bug,
    Ghost,
    Steel,
    Fire,
    Water,
    Grass,
    Electric,
    Psychic,
    Ice,
    Dragon,
    Dark,
}

/// 招式在伤害计算中的类别。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoveCategory {
    Physical,
    Special,
    Status,
}

/// 一只宝可梦同时最多拥有一种的主要异常状态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MajorStatus {
    Burn,
    BadlyPoisoned { stage: u8 },
    Freeze,
    Paralysis,
    Poison,
    Sleep { turns_remaining: u8 },
}

/// 不含睡眠或剧毒回合数的主要异常状态分类。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MajorStatusKind {
    Burn,
    BadlyPoisoned,
    Freeze,
    Paralysis,
    Poison,
    Sleep,
}

/// 不使用常规伤害公式的固定伤害来源。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixedDamage {
    Amount(u16),
    UserLevel,
}

/// 当前领域模型已实现或需要保留的特性。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ability {
    AirLock,
    ArenaTrap,
    BattleArmor,
    Blaze,
    Chlorophyll,
    ClearBody,
    CloudNine,
    CompoundEyes,
    Drizzle,
    Drought,
    EarlyBird,
    FlashFire,
    Guts,
    HugePower,
    HyperCutter,
    Hustle,
    Immunity,
    Intimidate,
    InnerFocus,
    Insomnia,
    Levitate,
    Limber,
    LiquidOoze,
    MagmaArmor,
    MarvelScale,
    KeenEye,
    NaturalCure,
    Overgrow,
    Pressure,
    PurePower,
    RainDish,
    RockHead,
    SandStream,
    SandVeil,
    SereneGrace,
    ShellArmor,
    ShedSkin,
    ShieldDust,
    ShadowTag,
    Synchronize,
    SpeedBoost,
    SwiftSwim,
    Swarm,
    ThickFat,
    Torrent,
    VitalSpirit,
    VoltAbsorb,
    WaterAbsorb,
    WaterVeil,
    WhiteSmoke,
}

/// 能力阶级允许的最小值。
pub const MIN_STAT_STAGE: i8 = -6;
/// 能力阶级允许的最大值。
pub const MAX_STAT_STAGE: i8 = 6;

/// 可被能力阶级修改的战斗能力值。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BattleStat {
    Attack,
    Defense,
    SpecialAttack,
    SpecialDefense,
    Speed,
    Accuracy,
    Evasion,
}

impl BattleStat {
    /// 按稳定顺序列出全部可修改能力值。
    pub const ALL: [Self; 7] = [
        Self::Attack,
        Self::Defense,
        Self::SpecialAttack,
        Self::SpecialDefense,
        Self::Speed,
        Self::Accuracy,
        Self::Evasion,
    ];
}

/// 一只宝可梦当前七项能力阶级。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatStages {
    attack: i8,
    defense: i8,
    special_attack: i8,
    special_defense: i8,
    speed: i8,
    accuracy: i8,
    evasion: i8,
}

impl StatStages {
    /// 返回所有能力阶级为零的状态。
    pub const fn neutral() -> Self {
        Self {
            attack: 0,
            defense: 0,
            special_attack: 0,
            special_defense: 0,
            speed: 0,
            accuracy: 0,
            evasion: 0,
        }
    }

    /// 返回指定能力值当前的阶级。
    pub const fn get(self, stat: BattleStat) -> i8 {
        match stat {
            BattleStat::Attack => self.attack,
            BattleStat::Defense => self.defense,
            BattleStat::SpecialAttack => self.special_attack,
            BattleStat::SpecialDefense => self.special_defense,
            BattleStat::Speed => self.speed,
            BattleStat::Accuracy => self.accuracy,
            BattleStat::Evasion => self.evasion,
        }
    }

    /// 将指定能力值设为一个有效阶级。
    ///
    /// 阶级必须在 [`MIN_STAT_STAGE`] 至 [`MAX_STAT_STAGE`] 之间。
    pub fn set(&mut self, stat: BattleStat, stage: i8) -> Result<(), ValidationError> {
        if !(MIN_STAT_STAGE..=MAX_STAT_STAGE).contains(&stage) {
            return Err(ValidationError::InvalidStageChange);
        }
        let value = match stat {
            BattleStat::Attack => &mut self.attack,
            BattleStat::Defense => &mut self.defense,
            BattleStat::SpecialAttack => &mut self.special_attack,
            BattleStat::SpecialDefense => &mut self.special_defense,
            BattleStat::Speed => &mut self.speed,
            BattleStat::Accuracy => &mut self.accuracy,
            BattleStat::Evasion => &mut self.evasion,
        };
        *value = stage;
        Ok(())
    }
}

/// 一次招式效果要施加到七项能力阶级上的增减量。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StageChanges {
    attack: i8,
    defense: i8,
    special_attack: i8,
    special_defense: i8,
    speed: i8,
    accuracy: i8,
    evasion: i8,
}

impl StageChanges {
    /// 创建至少修改一项且每项都在有效范围内的阶级变化。
    pub fn new(
        attack: i8,
        defense: i8,
        special_attack: i8,
        special_defense: i8,
        speed: i8,
        accuracy: i8,
        evasion: i8,
    ) -> Result<Self, ValidationError> {
        let changes = [
            attack,
            defense,
            special_attack,
            special_defense,
            speed,
            accuracy,
            evasion,
        ];
        if changes
            .iter()
            .any(|change| *change < MIN_STAT_STAGE || *change > MAX_STAT_STAGE)
        {
            return Err(ValidationError::InvalidStageChange);
        }
        if changes.iter().all(|change| *change == 0) {
            return Err(ValidationError::EmptyStageChanges);
        }
        Ok(Self {
            attack,
            defense,
            special_attack,
            special_defense,
            speed,
            accuracy,
            evasion,
        })
    }

    /// 创建只修改一项能力值的阶级变化。
    pub fn single(stat: BattleStat, amount: i8) -> Result<Self, ValidationError> {
        match stat {
            BattleStat::Attack => Self::new(amount, 0, 0, 0, 0, 0, 0),
            BattleStat::Defense => Self::new(0, amount, 0, 0, 0, 0, 0),
            BattleStat::SpecialAttack => Self::new(0, 0, amount, 0, 0, 0, 0),
            BattleStat::SpecialDefense => Self::new(0, 0, 0, amount, 0, 0, 0),
            BattleStat::Speed => Self::new(0, 0, 0, 0, amount, 0, 0),
            BattleStat::Accuracy => Self::new(0, 0, 0, 0, 0, amount, 0),
            BattleStat::Evasion => Self::new(0, 0, 0, 0, 0, 0, amount),
        }
    }

    pub const fn get(self, stat: BattleStat) -> i8 {
        match stat {
            BattleStat::Attack => self.attack,
            BattleStat::Defense => self.defense,
            BattleStat::SpecialAttack => self.special_attack,
            BattleStat::SpecialDefense => self.special_defense,
            BattleStat::Speed => self.speed,
            BattleStat::Accuracy => self.accuracy,
            BattleStat::Evasion => self.evasion,
        }
    }
}

/// 招式效果施加到使用者或对手的目标选择。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectTarget {
    User,
    Opponent,
}

/// 会持续多个回合的天气种类。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Weather {
    Hail,
    Rain,
    Sandstorm,
    Sun,
}

/// 当前天气及其持续时间。
///
/// `None` 表示永久天气，`Some(0)` 只会在结算阶段短暂存在。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WeatherState {
    weather: Weather,
    turns_remaining: Option<u8>,
}

impl WeatherState {
    /// 返回当前天气种类。
    pub const fn weather(self) -> Weather {
        self.weather
    }

    /// 返回临时天气剩余回合数，或永久天气的 `None`。
    pub const fn turns_remaining(self) -> Option<u8> {
        self.turns_remaining
    }

    /// 创建在指定回合数后结束的天气。
    pub const fn with_turns(weather: Weather, turns_remaining: u8) -> Self {
        Self {
            weather,
            turns_remaining: Some(turns_remaining),
        }
    }

    /// 创建不会自行结束的天气。
    pub const fn permanent(weather: Weather) -> Self {
        Self {
            weather,
            turns_remaining: None,
        }
    }
}

impl MajorStatus {
    /// 返回不包含状态内部回合数的分类。
    pub const fn kind(self) -> MajorStatusKind {
        match self {
            Self::Burn => MajorStatusKind::Burn,
            Self::BadlyPoisoned { .. } => MajorStatusKind::BadlyPoisoned,
            Self::Freeze => MajorStatusKind::Freeze,
            Self::Paralysis => MajorStatusKind::Paralysis,
            Self::Poison => MajorStatusKind::Poison,
            Self::Sleep { .. } => MajorStatusKind::Sleep,
        }
    }
}

/// 附加在招式上的非基础伤害效果。
///
/// 通过构造函数创建时，会校验概率和比例参数。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoveEffect {
    None,
    InflictMajorStatus {
        status: MajorStatusKind,
        chance: u8,
    },
    ChangeStages {
        target: EffectTarget,
        changes: StageChanges,
    },
    ChangeStagesWithChance {
        target: EffectTarget,
        changes: StageChanges,
        chance: u8,
    },
    HealUser {
        numerator: u8,
        denominator: u8,
    },
    DrainUser {
        numerator: u8,
        denominator: u8,
    },
    RecoilUser {
        numerator: u8,
        denominator: u8,
    },
    FixedDamage(FixedDamage),
    FlinchTarget {
        chance: u8,
    },
    CopyTargetStages,
    Haze,
    Rest,
    Refresh,
    CreateSubstitute,
    ProtectUser,
    StartWeather(Weather),
}

impl MoveEffect {
    /// 返回没有额外效果的招式效果。
    pub const fn none() -> Self {
        Self::None
    }

    /// 创建以给定概率施加主要异常状态的效果。
    pub fn inflict_major_status(
        status: MajorStatusKind,
        chance: u8,
    ) -> Result<Self, ValidationError> {
        if !(1..=100).contains(&chance) {
            return Err(ValidationError::InvalidEffectChance { value: chance });
        }
        Ok(Self::InflictMajorStatus { status, chance })
    }

    /// 创建无概率改变能力阶级的效果；`StageChanges` 已由构造时校验保证有效。
    pub const fn change_stages(target: EffectTarget, changes: StageChanges) -> Self {
        Self::ChangeStages { target, changes }
    }

    /// 创建以给定概率改变能力阶级的效果。
    pub fn change_stages_with_chance(
        target: EffectTarget,
        changes: StageChanges,
        chance: u8,
    ) -> Result<Self, ValidationError> {
        if !(1..=100).contains(&chance) {
            return Err(ValidationError::InvalidEffectChance { value: chance });
        }
        Ok(Self::ChangeStagesWithChance {
            target,
            changes,
            chance,
        })
    }

    /// 创建按造成伤害比例回复使用者 HP 的效果。
    pub fn heal_user(numerator: u8, denominator: u8) -> Result<Self, ValidationError> {
        validate_fraction(numerator, denominator)?;
        Ok(Self::HealUser {
            numerator,
            denominator,
        })
    }

    /// 创建按造成伤害比例回复使用者 HP 的吸取效果。
    pub fn drain_user(numerator: u8, denominator: u8) -> Result<Self, ValidationError> {
        validate_fraction(numerator, denominator)?;
        Ok(Self::DrainUser {
            numerator,
            denominator,
        })
    }

    /// 创建按造成伤害比例伤害使用者的反伤效果。
    pub fn recoil_user(numerator: u8, denominator: u8) -> Result<Self, ValidationError> {
        validate_fraction(numerator, denominator)?;
        Ok(Self::RecoilUser {
            numerator,
            denominator,
        })
    }

    pub const fn fixed_damage_amount(amount: u16) -> Self {
        Self::FixedDamage(FixedDamage::Amount(amount))
    }

    pub const fn fixed_damage_user_level() -> Self {
        Self::FixedDamage(FixedDamage::UserLevel)
    }

    pub const fn fixed_damage_for(self, user_level: u8) -> Option<u64> {
        match self {
            Self::FixedDamage(FixedDamage::Amount(amount)) => Some(amount as u64),
            Self::FixedDamage(FixedDamage::UserLevel) => Some(user_level as u64),
            _ => None,
        }
    }

    /// 返回效果是否允许招式威力为 0（固定伤害类效果）。
    pub const fn permits_zero_power(self) -> bool {
        matches!(self, Self::FixedDamage(_))
    }

    /// 返回效果是否为不造成伤害的次要效果。
    pub const fn is_non_damaging_secondary_effect(self) -> bool {
        matches!(
            self,
            Self::InflictMajorStatus { .. }
                | Self::ChangeStages { .. }
                | Self::ChangeStagesWithChance { .. }
        )
    }

    /// 创建以给定概率使目标畏缩的效果。
    pub fn flinch_target(chance: u8) -> Result<Self, ValidationError> {
        if !(1..=100).contains(&chance) {
            return Err(ValidationError::InvalidEffectChance { value: chance });
        }
        Ok(Self::FlinchTarget { chance })
    }

    pub const fn protect_user() -> Self {
        Self::ProtectUser
    }

    pub const fn create_substitute() -> Self {
        Self::CreateSubstitute
    }

    pub const fn haze() -> Self {
        Self::Haze
    }

    pub const fn copy_target_stages() -> Self {
        Self::CopyTargetStages
    }

    pub const fn rest() -> Self {
        Self::Rest
    }

    pub const fn refresh() -> Self {
        Self::Refresh
    }

    pub const fn start_weather(weather: Weather) -> Self {
        Self::StartWeather(weather)
    }

    pub const fn targets_opponent(self) -> bool {
        match self {
            Self::None
            | Self::InflictMajorStatus { .. }
            | Self::FixedDamage(_)
            | Self::FlinchTarget { .. }
            | Self::DrainUser { .. }
            | Self::RecoilUser { .. } => true,
            Self::ChangeStages {
                target: EffectTarget::Opponent,
                ..
            }
            | Self::ChangeStagesWithChance {
                target: EffectTarget::Opponent,
                ..
            } => true,
            Self::ChangeStages {
                target: EffectTarget::User,
                ..
            }
            | Self::ChangeStagesWithChance {
                target: EffectTarget::User,
                ..
            }
            | Self::HealUser { .. }
            | Self::CreateSubstitute
            | Self::CopyTargetStages
            | Self::Haze
            | Self::Rest
            | Self::Refresh
            | Self::ProtectUser
            | Self::StartWeather(_) => false,
        }
    }
}

fn validate_fraction(numerator: u8, denominator: u8) -> Result<(), ValidationError> {
    if numerator == 0 || denominator == 0 || numerator > denominator {
        return Err(ValidationError::InvalidHealFraction {
            numerator,
            denominator,
        });
    }
    Ok(())
}

impl MoveCategory {
    /// 返回属性在第三世代规则中默认的伤害类别。
    pub const fn for_gen3_type(move_type: PokemonType) -> Self {
        match move_type {
            PokemonType::Normal
            | PokemonType::Fighting
            | PokemonType::Flying
            | PokemonType::Poison
            | PokemonType::Ground
            | PokemonType::Rock
            | PokemonType::Bug
            | PokemonType::Ghost
            | PokemonType::Steel => Self::Physical,
            PokemonType::Fire
            | PokemonType::Water
            | PokemonType::Grass
            | PokemonType::Electric
            | PokemonType::Psychic
            | PokemonType::Ice
            | PokemonType::Dragon
            | PokemonType::Dark => Self::Special,
        }
    }
}

/// 招式命中率。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Accuracy {
    Percent(u8),
    AlwaysHit,
}

/// 受天气影响的命中率规则。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeatherAccuracyModifier {
    Thunder,
}

/// 受天气影响的威力、属性或类别规则。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeatherMoveModifier {
    WeatherBall,
}

impl Accuracy {
    /// 创建 1 至 100 的百分比命中率。
    pub fn percent(value: u8) -> Result<Self, ValidationError> {
        if (1..=100).contains(&value) {
            Ok(Self::Percent(value))
        } else {
            Err(ValidationError::InvalidAccuracy { value })
        }
    }
}

/// 已验证且均大于零的五项战斗能力值。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BattleStats {
    /// 攻击。
    pub attack: u16,
    /// 防御。
    pub defense: u16,
    /// 特攻。
    pub special_attack: u16,
    /// 特防。
    pub special_defense: u16,
    /// 速度。
    pub speed: u16,
}

impl BattleStats {
    /// 创建五项战斗能力值。
    ///
    /// 任一能力值为零时返回错误，因为伤害和速度计算不能使用零值。
    pub fn new(
        attack: u16,
        defense: u16,
        special_attack: u16,
        special_defense: u16,
        speed: u16,
    ) -> Result<Self, ValidationError> {
        if attack == 0 {
            return Err(ValidationError::ZeroStat { stat: "attack" });
        }
        if defense == 0 {
            return Err(ValidationError::ZeroStat { stat: "defense" });
        }
        if special_attack == 0 {
            return Err(ValidationError::ZeroStat {
                stat: "special_attack",
            });
        }
        if special_defense == 0 {
            return Err(ValidationError::ZeroStat {
                stat: "special_defense",
            });
        }
        if speed == 0 {
            return Err(ValidationError::ZeroStat { stat: "speed" });
        }
        Ok(Self {
            attack,
            defense,
            special_attack,
            special_defense,
            speed,
        })
    }

    pub const fn attack(self) -> u16 {
        self.attack
    }

    pub const fn defense(self) -> u16 {
        self.defense
    }

    pub const fn special_attack(self) -> u16 {
        self.special_attack
    }

    pub const fn special_defense(self) -> u16 {
        self.special_defense
    }

    pub const fn speed(self) -> u16 {
        self.speed
    }
}

/// 攻击属性相对于一只宝可梦全部属性的倍率。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeEffectiveness {
    Immune,
    Quarter,
    Half,
    Normal,
    Double,
    Quadruple,
}
