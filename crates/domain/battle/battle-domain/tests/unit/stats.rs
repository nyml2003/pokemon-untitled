use super::{
    EffortValues, IndividualValues, Nature, NonHpStat, StatBlock, StatName, StatProjectionError,
    TrainingValues, calculate_gen3_stats,
};

const BULBASAUR: StatBlock<u16> = StatBlock::new(45, 49, 49, 65, 65, 45);

#[test]
fn projects_bulbasaur_with_perfect_ivs_and_no_evs() {
    let stats = calculate_gen3_stats(BULBASAUR, 50, TrainingValues::perfect_untrained()).unwrap();
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
