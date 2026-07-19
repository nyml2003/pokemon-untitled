pub const TEAM_SIZE: usize = 6;
pub const MAX_MOVES: usize = 4;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Side {
    One,
    Two,
}

impl Side {
    pub const fn opponent(self) -> Self {
        match self {
            Self::One => Self::Two,
            Self::Two => Self::One,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TeamSlot(u8);

impl TeamSlot {
    pub fn new(index: usize) -> Result<Self, ValidationError> {
        if index < TEAM_SIZE {
            Ok(Self(index as u8))
        } else {
            Err(ValidationError::InvalidTeamSlot { index })
        }
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub(crate) const fn from_valid_index(index: usize) -> Self {
        Self(index as u8)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MoveSlot(u8);

impl MoveSlot {
    pub fn new(index: usize) -> Result<Self, ValidationError> {
        if index < MAX_MOVES {
            Ok(Self(index as u8))
        } else {
            Err(ValidationError::InvalidMoveSlot { index })
        }
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub(crate) const fn from_valid_index(index: usize) -> Self {
        Self(index as u8)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PokemonId(String);

impl PokemonId {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        Self::from_string(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn from_string(value: String) -> Result<Self, ValidationError> {
        if value.trim().is_empty() {
            Err(ValidationError::EmptyPokemonId)
        } else {
            Ok(Self(value))
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MoveId(String);

impl MoveId {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        Self::from_string(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn from_string(value: String) -> Result<Self, ValidationError> {
        if value.trim().is_empty() {
            Err(ValidationError::EmptyMoveId)
        } else {
            Ok(Self(value))
        }
    }
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoveCategory {
    Physical,
    Special,
    Status,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MajorStatus {
    Burn,
    BadlyPoisoned { stage: u8 },
    Freeze,
    Paralysis,
    Poison,
    Sleep { turns_remaining: u8 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MajorStatusKind {
    Burn,
    BadlyPoisoned,
    Freeze,
    Paralysis,
    Poison,
    Sleep,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixedDamage {
    Amount(u16),
    UserLevel,
}

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

pub const MIN_STAT_STAGE: i8 = -6;
pub const MAX_STAT_STAGE: i8 = 6;

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

    fn change(&mut self, stat: BattleStat, amount: i8) -> Option<i8> {
        let stage = match stat {
            BattleStat::Attack => &mut self.attack,
            BattleStat::Defense => &mut self.defense,
            BattleStat::SpecialAttack => &mut self.special_attack,
            BattleStat::SpecialDefense => &mut self.special_defense,
            BattleStat::Speed => &mut self.speed,
            BattleStat::Accuracy => &mut self.accuracy,
            BattleStat::Evasion => &mut self.evasion,
        };
        let next = (*stage)
            .saturating_add(amount)
            .clamp(MIN_STAT_STAGE, MAX_STAT_STAGE);
        if next == *stage {
            None
        } else {
            *stage = next;
            Some(next)
        }
    }
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectTarget {
    User,
    Opponent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Weather {
    Hail,
    Rain,
    Sandstorm,
    Sun,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WeatherState {
    weather: Weather,
    turns_remaining: Option<u8>,
}

impl WeatherState {
    pub const fn weather(self) -> Weather {
        self.weather
    }

    pub const fn turns_remaining(self) -> Option<u8> {
        self.turns_remaining
    }

    pub const fn with_turns(weather: Weather, turns_remaining: u8) -> Self {
        Self {
            weather,
            turns_remaining: Some(turns_remaining),
        }
    }

    pub const fn permanent(weather: Weather) -> Self {
        Self {
            weather,
            turns_remaining: None,
        }
    }

    pub(crate) fn elapse(&mut self) -> Option<bool> {
        let turns_remaining = self.turns_remaining?;
        let next = turns_remaining.saturating_sub(1);
        self.turns_remaining = Some(next);
        Some(next > 0)
    }
}

impl MajorStatus {
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
    pub const fn none() -> Self {
        Self::None
    }

    pub fn inflict_major_status(
        status: MajorStatusKind,
        chance: u8,
    ) -> Result<Self, ValidationError> {
        if !(1..=100).contains(&chance) {
            return Err(ValidationError::InvalidEffectChance { value: chance });
        }
        Ok(Self::InflictMajorStatus { status, chance })
    }

    pub const fn change_stages(target: EffectTarget, changes: StageChanges) -> Self {
        Self::ChangeStages { target, changes }
    }

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

    pub fn heal_user(numerator: u8, denominator: u8) -> Result<Self, ValidationError> {
        validate_fraction(numerator, denominator)?;
        Ok(Self::HealUser {
            numerator,
            denominator,
        })
    }

    pub fn drain_user(numerator: u8, denominator: u8) -> Result<Self, ValidationError> {
        validate_fraction(numerator, denominator)?;
        Ok(Self::DrainUser {
            numerator,
            denominator,
        })
    }

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

    pub const fn permits_zero_power(self) -> bool {
        matches!(self, Self::FixedDamage(_))
    }

    pub const fn is_non_damaging_secondary_effect(self) -> bool {
        matches!(
            self,
            Self::InflictMajorStatus { .. }
                | Self::ChangeStages { .. }
                | Self::ChangeStagesWithChance { .. }
        )
    }

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Accuracy {
    Percent(u8),
    AlwaysHit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeatherAccuracyModifier {
    Thunder,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeatherMoveModifier {
    WeatherBall,
}

impl Accuracy {
    pub fn percent(value: u8) -> Result<Self, ValidationError> {
        if (1..=100).contains(&value) {
            Ok(Self::Percent(value))
        } else {
            Err(ValidationError::InvalidAccuracy { value })
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BattleStats {
    attack: u16,
    defense: u16,
    special_attack: u16,
    special_defense: u16,
    speed: u16,
}

impl BattleStats {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Move {
    id: MoveId,
    name: String,
    move_type: PokemonType,
    category: MoveCategory,
    power: u16,
    accuracy: Accuracy,
    max_pp: u8,
    current_pp: u8,
    priority: i8,
    effect: MoveEffect,
    weather_accuracy: Option<WeatherAccuracyModifier>,
    weather_move: Option<WeatherMoveModifier>,
}

impl Move {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: MoveId,
        name: impl Into<String>,
        move_type: PokemonType,
        power: u16,
        accuracy: Accuracy,
        max_pp: u8,
        current_pp: u8,
        priority: i8,
    ) -> Result<Self, ValidationError> {
        Self::new_with_category(
            id,
            name,
            move_type,
            MoveCategory::for_gen3_type(move_type),
            power,
            accuracy,
            max_pp,
            current_pp,
            priority,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_category(
        id: MoveId,
        name: impl Into<String>,
        move_type: PokemonType,
        category: MoveCategory,
        power: u16,
        accuracy: Accuracy,
        max_pp: u8,
        current_pp: u8,
        priority: i8,
    ) -> Result<Self, ValidationError> {
        Self::new_with_category_and_effect(
            id,
            name,
            move_type,
            category,
            power,
            accuracy,
            max_pp,
            current_pp,
            priority,
            MoveEffect::None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_category_and_effect(
        id: MoveId,
        name: impl Into<String>,
        move_type: PokemonType,
        category: MoveCategory,
        power: u16,
        accuracy: Accuracy,
        max_pp: u8,
        current_pp: u8,
        priority: i8,
        effect: MoveEffect,
    ) -> Result<Self, ValidationError> {
        Self::from_parts(
            id,
            name.into(),
            move_type,
            category,
            power,
            accuracy,
            max_pp,
            current_pp,
            priority,
            effect,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_parts(
        id: MoveId,
        name: String,
        move_type: PokemonType,
        category: MoveCategory,
        power: u16,
        accuracy: Accuracy,
        max_pp: u8,
        current_pp: u8,
        priority: i8,
        effect: MoveEffect,
    ) -> Result<Self, ValidationError> {
        if name.trim().is_empty() {
            return Err(ValidationError::EmptyMoveName);
        }
        if power == 0 && category != MoveCategory::Status && !effect.permits_zero_power() {
            return Err(ValidationError::ZeroMovePower);
        }
        if let Accuracy::Percent(value) = accuracy
            && !(1..=100).contains(&value)
        {
            return Err(ValidationError::InvalidAccuracy { value });
        }
        if max_pp == 0 {
            return Err(ValidationError::ZeroMaxPp);
        }
        if current_pp > max_pp {
            return Err(ValidationError::CurrentPpExceedsMax {
                current: current_pp,
                max: max_pp,
            });
        }
        Ok(Self {
            id,
            name,
            move_type,
            category,
            power,
            accuracy,
            max_pp,
            current_pp,
            priority,
            effect,
            weather_accuracy: None,
            weather_move: None,
        })
    }

    pub fn id(&self) -> &MoveId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn move_type(&self) -> PokemonType {
        self.move_type
    }

    pub const fn category(&self) -> MoveCategory {
        self.category
    }

    pub const fn power(&self) -> u16 {
        self.power
    }

    pub const fn accuracy(&self) -> Accuracy {
        self.accuracy
    }

    pub const fn max_pp(&self) -> u8 {
        self.max_pp
    }

    pub const fn current_pp(&self) -> u8 {
        self.current_pp
    }

    pub const fn priority(&self) -> i8 {
        self.priority
    }

    pub const fn effect(&self) -> MoveEffect {
        self.effect
    }

    pub const fn with_weather_accuracy(mut self, modifier: WeatherAccuracyModifier) -> Self {
        self.weather_accuracy = Some(modifier);
        self
    }

    pub const fn weather_accuracy(&self) -> Option<WeatherAccuracyModifier> {
        self.weather_accuracy
    }

    pub const fn with_weather_move(mut self, modifier: WeatherMoveModifier) -> Self {
        self.weather_move = Some(modifier);
        self
    }

    pub const fn weather_move(&self) -> Option<WeatherMoveModifier> {
        self.weather_move
    }

    pub(crate) fn spend_pp(&mut self) {
        self.current_pp = self.current_pp.saturating_sub(1);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pokemon {
    id: PokemonId,
    name: String,
    level: u8,
    primary_type: PokemonType,
    secondary_type: Option<PokemonType>,
    max_hp: u32,
    current_hp: u32,
    stats: BattleStats,
    moves: Vec<Move>,
    ability: Option<Ability>,
    substitute_hp: Option<u32>,
    major_status: Option<MajorStatus>,
    stages: StatStages,
    protect_streak: u8,
}

impl Pokemon {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: PokemonId,
        name: impl Into<String>,
        level: u8,
        primary_type: PokemonType,
        secondary_type: Option<PokemonType>,
        max_hp: u32,
        current_hp: u32,
        stats: BattleStats,
        moves: Vec<Move>,
    ) -> Result<Self, ValidationError> {
        Self::from_parts(
            id,
            name.into(),
            level,
            primary_type,
            secondary_type,
            max_hp,
            current_hp,
            stats,
            moves,
            None,
            None,
            None,
            StatStages::neutral(),
            0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_ability(
        id: PokemonId,
        name: impl Into<String>,
        level: u8,
        primary_type: PokemonType,
        secondary_type: Option<PokemonType>,
        max_hp: u32,
        current_hp: u32,
        stats: BattleStats,
        moves: Vec<Move>,
        ability: Ability,
    ) -> Result<Self, ValidationError> {
        Self::from_parts(
            id,
            name.into(),
            level,
            primary_type,
            secondary_type,
            max_hp,
            current_hp,
            stats,
            moves,
            Some(ability),
            None,
            None,
            StatStages::neutral(),
            0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_parts(
        id: PokemonId,
        name: String,
        level: u8,
        primary_type: PokemonType,
        secondary_type: Option<PokemonType>,
        max_hp: u32,
        current_hp: u32,
        stats: BattleStats,
        moves: Vec<Move>,
        ability: Option<Ability>,
        substitute_hp: Option<u32>,
        major_status: Option<MajorStatus>,
        stages: StatStages,
        protect_streak: u8,
    ) -> Result<Self, ValidationError> {
        if name.trim().is_empty() {
            return Err(ValidationError::EmptyPokemonName);
        }
        if !(1..=100).contains(&level) {
            return Err(ValidationError::InvalidLevel { level });
        }
        if secondary_type == Some(primary_type) {
            return Err(ValidationError::DuplicatePokemonType { primary_type });
        }
        if max_hp == 0 {
            return Err(ValidationError::ZeroMaxHp);
        }
        if current_hp > max_hp {
            return Err(ValidationError::CurrentHpExceedsMax {
                current: current_hp,
                max: max_hp,
            });
        }
        if moves.is_empty() || moves.len() > MAX_MOVES {
            return Err(ValidationError::InvalidMoveCount { count: moves.len() });
        }
        for left in 0..moves.len() {
            for right in (left + 1)..moves.len() {
                if moves[left].id == moves[right].id {
                    return Err(ValidationError::DuplicateMoveId {
                        id: moves[left].id.clone(),
                    });
                }
            }
        }
        Ok(Self {
            id,
            name,
            level,
            primary_type,
            secondary_type,
            max_hp,
            current_hp,
            stats,
            moves,
            ability,
            substitute_hp,
            major_status,
            stages,
            protect_streak,
        })
    }

    pub fn id(&self) -> &PokemonId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn level(&self) -> u8 {
        self.level
    }

    pub const fn primary_type(&self) -> PokemonType {
        self.primary_type
    }

    pub const fn secondary_type(&self) -> Option<PokemonType> {
        self.secondary_type
    }

    pub const fn max_hp(&self) -> u32 {
        self.max_hp
    }

    pub const fn current_hp(&self) -> u32 {
        self.current_hp
    }

    pub const fn stats(&self) -> BattleStats {
        self.stats
    }

    pub const fn physical_attack(&self) -> u16 {
        let attack = match (self.ability, self.major_status) {
            (Some(Ability::Guts), Some(_)) => self.stats.attack.saturating_mul(3) / 2,
            (_, Some(MajorStatus::Burn)) => self.stats.attack / 2,
            (
                _,
                Some(
                    MajorStatus::BadlyPoisoned { .. }
                    | MajorStatus::Freeze
                    | MajorStatus::Paralysis
                    | MajorStatus::Poison
                    | MajorStatus::Sleep { .. },
                ),
            )
            | (_, None) => self.stats.attack,
        };
        match self.ability {
            Some(Ability::HugePower | Ability::PurePower) => attack.saturating_mul(2),
            Some(Ability::Hustle) => attack.saturating_mul(3) / 2,
            _ => attack,
        }
    }

    pub fn physical_attack_ability_is_active(&self) -> bool {
        matches!(
            self.ability,
            Some(Ability::HugePower | Ability::PurePower | Ability::Hustle)
        ) || (self.ability == Some(Ability::Guts) && self.major_status.is_some())
    }

    pub const fn accuracy_ability(
        &self,
        category: MoveCategory,
        accuracy: Accuracy,
    ) -> Option<Ability> {
        match (self.ability, category, accuracy) {
            (Some(Ability::CompoundEyes), _, Accuracy::Percent(_)) => Some(Ability::CompoundEyes),
            (Some(Ability::Hustle), MoveCategory::Physical, Accuracy::Percent(_)) => {
                Some(Ability::Hustle)
            }
            _ => None,
        }
    }

    pub fn effective_attack(&self) -> u16 {
        stage_modified_stat(self.physical_attack(), self.stages.get(BattleStat::Attack))
    }

    pub fn effective_defense(&self) -> u16 {
        let defense = if self.ability == Some(Ability::MarvelScale) && self.major_status.is_some() {
            self.stats.defense.saturating_mul(3) / 2
        } else {
            self.stats.defense
        };
        stage_modified_stat(defense, self.stages.get(BattleStat::Defense))
    }

    pub fn defense_ability_is_active(&self) -> bool {
        self.ability == Some(Ability::MarvelScale) && self.major_status.is_some()
    }

    pub fn effective_special_attack(&self) -> u16 {
        stage_modified_stat(
            self.stats.special_attack,
            self.stages.get(BattleStat::SpecialAttack),
        )
    }

    pub fn effective_special_defense(&self) -> u16 {
        stage_modified_stat(
            self.stats.special_defense,
            self.stages.get(BattleStat::SpecialDefense),
        )
    }

    pub fn effective_speed(&self) -> u16 {
        let speed = match self.major_status {
            Some(MajorStatus::Paralysis) => (self.stats.speed / 4).max(1),
            Some(
                MajorStatus::BadlyPoisoned { .. }
                | MajorStatus::Burn
                | MajorStatus::Freeze
                | MajorStatus::Poison
                | MajorStatus::Sleep { .. },
            )
            | None => self.stats.speed,
        };
        stage_modified_stat(speed, self.stages.get(BattleStat::Speed))
    }

    pub fn effective_speed_in_weather(&self, weather: Option<Weather>) -> u16 {
        match (self.ability, weather) {
            (Some(Ability::Chlorophyll), Some(Weather::Sun))
            | (Some(Ability::SwiftSwim), Some(Weather::Rain)) => {
                self.effective_speed().saturating_mul(2)
            }
            _ => self.effective_speed(),
        }
    }

    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    pub const fn ability(&self) -> Option<Ability> {
        self.ability
    }

    pub const fn substitute_hp(&self) -> Option<u32> {
        self.substitute_hp
    }

    pub const fn major_status(&self) -> Option<MajorStatus> {
        self.major_status
    }

    pub const fn stages(&self) -> StatStages {
        self.stages
    }

    pub const fn is_fainted(&self) -> bool {
        self.current_hp == 0
    }

    pub(crate) fn move_mut(&mut self, slot: MoveSlot) -> Option<&mut Move> {
        self.moves.get_mut(slot.index())
    }

    pub(crate) fn apply_damage(&mut self, damage: u64) -> u32 {
        let actual = damage.min(u64::from(self.current_hp)) as u32;
        self.current_hp -= actual;
        actual
    }

    pub(crate) fn heal(&mut self, amount: u64) -> u32 {
        let missing = self.max_hp - self.current_hp;
        let actual = amount.min(u64::from(missing)) as u32;
        self.current_hp += actual;
        actual
    }

    pub(crate) fn create_substitute(&mut self) -> Option<u32> {
        if self.substitute_hp.is_some() {
            return None;
        }
        let cost = (self.max_hp / 4).max(1);
        if self.current_hp <= cost {
            return None;
        }
        self.current_hp -= cost;
        self.substitute_hp = Some(cost);
        Some(cost)
    }

    pub(crate) fn damage_substitute(&mut self, damage: u64) -> Option<(u32, u32, bool)> {
        let hp = self.substitute_hp?;
        let actual = damage.min(u64::from(hp)) as u32;
        let remaining = hp - actual;
        self.substitute_hp = (remaining > 0).then_some(remaining);
        Some((actual, remaining, remaining == 0))
    }

    pub(crate) fn change_stage(&mut self, stat: BattleStat, amount: i8) -> Option<i8> {
        self.stages.change(stat, amount)
    }

    pub(crate) fn reset_switch_modifiers(&mut self) {
        self.stages = StatStages::neutral();
        self.protect_streak = 0;
        self.substitute_hp = None;
        if matches!(self.major_status, Some(MajorStatus::BadlyPoisoned { .. })) {
            self.major_status = Some(MajorStatus::BadlyPoisoned { stage: 1 });
        }
    }

    pub(crate) const fn protect_streak(&self) -> u8 {
        self.protect_streak
    }

    pub(crate) fn record_protect_success(&mut self) {
        self.protect_streak = self.protect_streak.saturating_add(1);
    }

    pub(crate) fn reset_protect_streak(&mut self) {
        self.protect_streak = 0;
    }

    pub(crate) fn inflict_major_status(&mut self, status: MajorStatus) -> bool {
        if self.major_status.is_some() || self.is_immune_to(status.kind()) {
            return false;
        }
        self.major_status = Some(status);
        true
    }

    pub(crate) fn advance_sleep(&mut self) -> Option<u8> {
        let MajorStatus::Sleep { turns_remaining } = self.major_status? else {
            return None;
        };
        let next = turns_remaining.saturating_sub(1);
        self.major_status = (next > 0).then_some(MajorStatus::Sleep {
            turns_remaining: next,
        });
        Some(next)
    }

    pub(crate) fn advance_badly_poison(&mut self) -> Option<u8> {
        let MajorStatus::BadlyPoisoned { stage } = self.major_status? else {
            return None;
        };
        let next = stage.saturating_add(1);
        self.major_status = Some(MajorStatus::BadlyPoisoned { stage: next });
        Some(next)
    }

    pub(crate) fn rest(&mut self) -> Option<(u32, Option<MajorStatus>)> {
        if self.current_hp == self.max_hp && self.major_status.is_none() {
            return None;
        }
        let previous_status = self.major_status;
        let healed = self.max_hp - self.current_hp;
        self.current_hp = self.max_hp;
        // The action check decrements before deciding whether the Pokemon acts,
        // so three ticks produce the two skipped turns of generation-three Rest.
        self.major_status = Some(MajorStatus::Sleep { turns_remaining: 3 });
        Some((healed, previous_status))
    }

    pub(crate) fn refresh(&mut self) -> Option<MajorStatusKind> {
        let status = self.major_status?;
        if !matches!(
            status,
            MajorStatus::BadlyPoisoned { .. }
                | MajorStatus::Burn
                | MajorStatus::Paralysis
                | MajorStatus::Poison
        ) {
            return None;
        }
        self.major_status = None;
        Some(status.kind())
    }

    pub(crate) fn cure_major_status(&mut self) -> Option<MajorStatusKind> {
        let status = self.major_status?;
        self.major_status = None;
        Some(status.kind())
    }

    fn is_immune_to(&self, status: MajorStatusKind) -> bool {
        self.ability_blocks_status(status).is_some()
            || match status {
                MajorStatusKind::Burn => {
                    self.primary_type == PokemonType::Fire
                        || self.secondary_type == Some(PokemonType::Fire)
                }
                MajorStatusKind::Freeze => {
                    self.primary_type == PokemonType::Ice
                        || self.secondary_type == Some(PokemonType::Ice)
                }
                MajorStatusKind::Poison | MajorStatusKind::BadlyPoisoned => {
                    self.primary_type == PokemonType::Poison
                        || self.secondary_type == Some(PokemonType::Poison)
                        || self.primary_type == PokemonType::Steel
                        || self.secondary_type == Some(PokemonType::Steel)
                }
                MajorStatusKind::Paralysis | MajorStatusKind::Sleep => false,
            }
    }

    pub(crate) const fn ability_blocks_status(&self, status: MajorStatusKind) -> Option<Ability> {
        match (self.ability, status) {
            (Some(Ability::Immunity), MajorStatusKind::Poison | MajorStatusKind::BadlyPoisoned)
            | (Some(Ability::Limber), MajorStatusKind::Paralysis)
            | (Some(Ability::WaterVeil), MajorStatusKind::Burn)
            | (Some(Ability::Insomnia | Ability::VitalSpirit), MajorStatusKind::Sleep)
            | (Some(Ability::MagmaArmor), MajorStatusKind::Freeze) => self.ability,
            _ => None,
        }
    }

    pub(crate) const fn ability_blocks_move(&self, move_type: PokemonType) -> Option<Ability> {
        match (self.ability, move_type) {
            (Some(Ability::Levitate), PokemonType::Ground)
            | (Some(Ability::FlashFire), PokemonType::Fire)
            | (Some(Ability::WaterAbsorb), PokemonType::Water)
            | (Some(Ability::VoltAbsorb), PokemonType::Electric) => self.ability,
            _ => None,
        }
    }

    pub(crate) const fn ability_blocks_secondary_effect(&self) -> Option<Ability> {
        match self.ability {
            Some(Ability::ShieldDust) => Some(Ability::ShieldDust),
            _ => None,
        }
    }

    pub(crate) const fn ability_blocks_opponent_stat_drop(
        &self,
        stat: BattleStat,
    ) -> Option<Ability> {
        match (self.ability, stat) {
            (Some(Ability::ClearBody | Ability::WhiteSmoke), _)
            | (Some(Ability::HyperCutter), BattleStat::Attack)
            | (Some(Ability::KeenEye), BattleStat::Accuracy) => self.ability,
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Team {
    members: [Pokemon; TEAM_SIZE],
}

impl Team {
    pub fn new(members: Vec<Pokemon>) -> Result<Self, ValidationError> {
        if members.len() != TEAM_SIZE {
            return Err(ValidationError::InvalidTeamSize {
                count: members.len(),
            });
        }
        for left in 0..members.len() {
            for right in (left + 1)..members.len() {
                if members[left].id == members[right].id {
                    return Err(ValidationError::DuplicatePokemonId {
                        id: members[left].id.clone(),
                    });
                }
            }
        }
        let mut members = members.into_iter();
        let members = std::array::from_fn(|_| {
            members
                .next()
                .expect("team length was validated before array construction")
        });
        Ok(Self { members })
    }

    pub fn members(&self) -> &[Pokemon; TEAM_SIZE] {
        &self.members
    }

    pub fn member(&self, slot: TeamSlot) -> &Pokemon {
        &self.members[slot.index()]
    }

    pub(crate) fn member_mut(&mut self, slot: TeamSlot) -> &mut Pokemon {
        &mut self.members[slot.index()]
    }

    pub(crate) fn first_living_slot(&self) -> Option<TeamSlot> {
        self.members
            .iter()
            .position(|pokemon| !pokemon.is_fainted())
            .map(TeamSlot::from_valid_index)
    }

    pub(crate) fn has_living(&self) -> bool {
        self.members.iter().any(|pokemon| !pokemon.is_fainted())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationError {
    InvalidTeamSlot { index: usize },
    InvalidMoveSlot { index: usize },
    EmptyPokemonId,
    EmptyMoveId,
    EmptyPokemonName,
    EmptyMoveName,
    InvalidLevel { level: u8 },
    DuplicatePokemonType { primary_type: PokemonType },
    ZeroMaxHp,
    CurrentHpExceedsMax { current: u32, max: u32 },
    ZeroStat { stat: &'static str },
    ZeroMovePower,
    InvalidAccuracy { value: u8 },
    InvalidEffectChance { value: u8 },
    InvalidStageChange,
    EmptyStageChanges,
    InvalidHealFraction { numerator: u8, denominator: u8 },
    ZeroMaxPp,
    CurrentPpExceedsMax { current: u8, max: u8 },
    InvalidMoveCount { count: usize },
    InvalidTeamSize { count: usize },
    DuplicateMoveId { id: MoveId },
    DuplicatePokemonId { id: PokemonId },
}

fn stage_modified_stat(value: u16, stage: i8) -> u16 {
    let value = u32::from(value);
    let adjusted = if stage >= 0 {
        value * u32::from(2 + stage as u8) / 2
    } else {
        value * 2 / u32::from(2 + (-stage) as u8)
    };
    adjusted.max(1) as u16
}
