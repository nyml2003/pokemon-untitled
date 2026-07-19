use crate::{BattleStats, ValidationError};

pub const MAX_INDIVIDUAL_VALUE: u8 = 31;
pub const MAX_EFFORT_VALUE: u16 = 255;
pub const MAX_TOTAL_EFFORT_VALUE: u16 = 510;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatName {
    Hp,
    Attack,
    Defense,
    SpecialAttack,
    SpecialDefense,
    Speed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NonHpStat {
    Attack,
    Defense,
    SpecialAttack,
    SpecialDefense,
    Speed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatBlock<T> {
    pub hp: T,
    pub attack: T,
    pub defense: T,
    pub special_attack: T,
    pub special_defense: T,
    pub speed: T,
}

impl<T> StatBlock<T> {
    pub const fn new(
        hp: T,
        attack: T,
        defense: T,
        special_attack: T,
        special_defense: T,
        speed: T,
    ) -> Self {
        Self {
            hp,
            attack,
            defense,
            special_attack,
            special_defense,
            speed,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndividualValues(StatBlock<u8>);

impl IndividualValues {
    pub fn new(values: StatBlock<u8>) -> Result<Self, StatProjectionError> {
        for (stat, value) in named_values(values) {
            if value > MAX_INDIVIDUAL_VALUE {
                return Err(StatProjectionError::InvalidIndividualValue { stat, value });
            }
        }
        Ok(Self(values))
    }

    pub const fn perfect() -> Self {
        Self(StatBlock::new(31, 31, 31, 31, 31, 31))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EffortValues(StatBlock<u8>);

impl EffortValues {
    pub fn new(values: StatBlock<u16>) -> Result<Self, StatProjectionError> {
        for (stat, value) in named_values(values) {
            if value > MAX_EFFORT_VALUE {
                return Err(StatProjectionError::InvalidEffortValue { stat, value });
            }
        }
        let total = values.hp
            + values.attack
            + values.defense
            + values.special_attack
            + values.special_defense
            + values.speed;
        if total > MAX_TOTAL_EFFORT_VALUE {
            return Err(StatProjectionError::EffortTotalExceeded {
                total,
                max: MAX_TOTAL_EFFORT_VALUE,
            });
        }
        Ok(Self(StatBlock::new(
            values.hp as u8,
            values.attack as u8,
            values.defense as u8,
            values.special_attack as u8,
            values.special_defense as u8,
            values.speed as u8,
        )))
    }

    pub const fn untrained() -> Self {
        Self(StatBlock::new(0, 0, 0, 0, 0, 0))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Nature {
    raised: Option<NonHpStat>,
    lowered: Option<NonHpStat>,
}

impl Nature {
    pub const fn neutral() -> Self {
        Self {
            raised: None,
            lowered: None,
        }
    }

    pub fn adjusted(raised: NonHpStat, lowered: NonHpStat) -> Result<Self, StatProjectionError> {
        if raised == lowered {
            return Err(StatProjectionError::InvalidNature { raised, lowered });
        }
        Ok(Self {
            raised: Some(raised),
            lowered: Some(lowered),
        })
    }

    fn multiplier(self, stat: NonHpStat) -> u32 {
        if self.raised == Some(stat) {
            110
        } else if self.lowered == Some(stat) {
            90
        } else {
            100
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrainingValues {
    ivs: IndividualValues,
    evs: EffortValues,
    nature: Nature,
}

impl TrainingValues {
    pub const fn new(ivs: IndividualValues, evs: EffortValues, nature: Nature) -> Self {
        Self { ivs, evs, nature }
    }

    pub const fn perfect_untrained() -> Self {
        Self::new(
            IndividualValues::perfect(),
            EffortValues::untrained(),
            Nature::neutral(),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CalculatedStats {
    max_hp: u32,
    battle: BattleStats,
}

impl CalculatedStats {
    pub const fn max_hp(self) -> u32 {
        self.max_hp
    }

    pub const fn battle(self) -> BattleStats {
        self.battle
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatProjectionError {
    InvalidLevel {
        value: u8,
    },
    InvalidIndividualValue {
        stat: StatName,
        value: u8,
    },
    InvalidEffortValue {
        stat: StatName,
        value: u16,
    },
    EffortTotalExceeded {
        total: u16,
        max: u16,
    },
    InvalidNature {
        raised: NonHpStat,
        lowered: NonHpStat,
    },
    ZeroBaseStat {
        stat: StatName,
    },
    CalculatedStatOutOfRange {
        stat: StatName,
        value: u32,
    },
    InvalidBattleStats,
}

pub fn calculate_gen3_stats(
    base: StatBlock<u16>,
    level: u8,
    training: TrainingValues,
) -> Result<CalculatedStats, StatProjectionError> {
    if !(1..=100).contains(&level) {
        return Err(StatProjectionError::InvalidLevel { value: level });
    }
    for (stat, value) in named_values(base) {
        if value == 0 {
            return Err(StatProjectionError::ZeroBaseStat { stat });
        }
    }

    let ivs = training.ivs.0;
    let evs = training.evs.0;
    let max_hp = calculate_hp(base.hp, ivs.hp, evs.hp, level);
    let attack = calculate_non_hp(
        base.attack,
        ivs.attack,
        evs.attack,
        level,
        training.nature.multiplier(NonHpStat::Attack),
    );
    let defense = calculate_non_hp(
        base.defense,
        ivs.defense,
        evs.defense,
        level,
        training.nature.multiplier(NonHpStat::Defense),
    );
    let special_attack = calculate_non_hp(
        base.special_attack,
        ivs.special_attack,
        evs.special_attack,
        level,
        training.nature.multiplier(NonHpStat::SpecialAttack),
    );
    let special_defense = calculate_non_hp(
        base.special_defense,
        ivs.special_defense,
        evs.special_defense,
        level,
        training.nature.multiplier(NonHpStat::SpecialDefense),
    );
    let speed = calculate_non_hp(
        base.speed,
        ivs.speed,
        evs.speed,
        level,
        training.nature.multiplier(NonHpStat::Speed),
    );

    let attack = checked_stat(StatName::Attack, attack)?;
    let defense = checked_stat(StatName::Defense, defense)?;
    let special_attack = checked_stat(StatName::SpecialAttack, special_attack)?;
    let special_defense = checked_stat(StatName::SpecialDefense, special_defense)?;
    let speed = checked_stat(StatName::Speed, speed)?;
    let battle = BattleStats::new(attack, defense, special_attack, special_defense, speed)
        .map_err(|_: ValidationError| StatProjectionError::InvalidBattleStats)?;
    Ok(CalculatedStats { max_hp, battle })
}

fn calculate_hp(base: u16, iv: u8, effort: u8, level: u8) -> u32 {
    let base_part = 2 * u32::from(base) + u32::from(iv) + u32::from(effort / 4);
    base_part * u32::from(level) / 100 + u32::from(level) + 10
}

fn calculate_non_hp(base: u16, iv: u8, effort: u8, level: u8, nature: u32) -> u32 {
    let base_part = 2 * u32::from(base) + u32::from(iv) + u32::from(effort / 4);
    let before_nature = base_part * u32::from(level) / 100 + 5;
    before_nature * nature / 100
}

fn checked_stat(stat: StatName, value: u32) -> Result<u16, StatProjectionError> {
    u16::try_from(value).map_err(|_| StatProjectionError::CalculatedStatOutOfRange { stat, value })
}

fn named_values<T: Copy>(values: StatBlock<T>) -> [(StatName, T); 6] {
    [
        (StatName::Hp, values.hp),
        (StatName::Attack, values.attack),
        (StatName::Defense, values.defense),
        (StatName::SpecialAttack, values.special_attack),
        (StatName::SpecialDefense, values.special_defense),
        (StatName::Speed, values.speed),
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        EffortValues, IndividualValues, Nature, NonHpStat, StatBlock, StatName,
        StatProjectionError, TrainingValues, calculate_gen3_stats,
    };

    const BULBASAUR: StatBlock<u16> = StatBlock::new(45, 49, 49, 65, 65, 45);

    #[test]
    fn projects_bulbasaur_with_perfect_ivs_and_no_evs() {
        let stats =
            calculate_gen3_stats(BULBASAUR, 50, TrainingValues::perfect_untrained()).unwrap();
        assert_eq!(stats.max_hp(), 120);
        assert_eq!(stats.battle().attack(), 69);
        assert_eq!(stats.battle().defense(), 69);
        assert_eq!(stats.battle().special_attack(), 85);
        assert_eq!(stats.battle().special_defense(), 85);
        assert_eq!(stats.battle().speed(), 65);
    }

    #[test]
    fn validates_iv_ev_level_and_total_boundaries() {
        assert_eq!(
            IndividualValues::new(StatBlock::new(32, 0, 0, 0, 0, 0)),
            Err(StatProjectionError::InvalidIndividualValue {
                stat: StatName::Hp,
                value: 32,
            })
        );
        assert_eq!(
            EffortValues::new(StatBlock::new(256, 0, 0, 0, 0, 0)),
            Err(StatProjectionError::InvalidEffortValue {
                stat: StatName::Hp,
                value: 256,
            })
        );
        assert!(EffortValues::new(StatBlock::new(255, 255, 0, 0, 0, 0)).is_ok());
        assert_eq!(
            EffortValues::new(StatBlock::new(255, 255, 1, 0, 0, 0)),
            Err(StatProjectionError::EffortTotalExceeded {
                total: 511,
                max: 510,
            })
        );
        assert_eq!(
            calculate_gen3_stats(BULBASAUR, 0, TrainingValues::perfect_untrained()),
            Err(StatProjectionError::InvalidLevel { value: 0 })
        );
        assert_eq!(
            calculate_gen3_stats(
                StatBlock::new(0, 49, 49, 65, 65, 45),
                50,
                TrainingValues::perfect_untrained()
            ),
            Err(StatProjectionError::ZeroBaseStat { stat: StatName::Hp })
        );
        assert!(calculate_gen3_stats(BULBASAUR, 100, TrainingValues::perfect_untrained()).is_ok());
    }

    #[test]
    fn effort_contribution_floors_at_multiples_of_four() {
        let ivs = IndividualValues::new(StatBlock::new(0, 0, 0, 0, 0, 0)).unwrap();
        let ev_three = EffortValues::new(StatBlock::new(0, 3, 0, 0, 0, 0)).unwrap();
        let ev_four = EffortValues::new(StatBlock::new(0, 4, 0, 0, 0, 0)).unwrap();
        let base = StatBlock::new(100, 100, 100, 100, 100, 100);
        let three = calculate_gen3_stats(
            base,
            100,
            TrainingValues::new(ivs, ev_three, Nature::neutral()),
        )
        .unwrap();
        let four = calculate_gen3_stats(
            base,
            100,
            TrainingValues::new(ivs, ev_four, Nature::neutral()),
        )
        .unwrap();
        assert_eq!(three.battle().attack(), 205);
        assert_eq!(four.battle().attack(), 206);
    }

    #[test]
    fn nature_uses_integer_raise_and_lower_multipliers() {
        let ivs = IndividualValues::new(StatBlock::new(0, 0, 0, 0, 0, 0)).unwrap();
        let nature = Nature::adjusted(NonHpStat::Attack, NonHpStat::Speed).unwrap();
        let stats = calculate_gen3_stats(
            StatBlock::new(100, 100, 100, 100, 100, 100),
            50,
            TrainingValues::new(ivs, EffortValues::untrained(), nature),
        )
        .unwrap();
        assert_eq!(stats.battle().attack(), 115);
        assert_eq!(stats.battle().defense(), 105);
        assert_eq!(stats.battle().speed(), 94);
        assert_eq!(
            Nature::adjusted(NonHpStat::Attack, NonHpStat::Attack),
            Err(StatProjectionError::InvalidNature {
                raised: NonHpStat::Attack,
                lowered: NonHpStat::Attack,
            })
        );
    }
}
