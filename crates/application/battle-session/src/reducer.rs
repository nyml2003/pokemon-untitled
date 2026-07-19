use battle_application::{
    Ability, BattleEvent, BattleObservation, BattleStat, BattleTransition, MajorStatus,
    MajorStatusKind, ObservedBattleOutcome, Participant, Pokemon, PokemonId, PokemonType,
    RevealedCombatant, RevealedPokemonObservation, StatStages, TypeEffectiveness, UsedMove,
    Weather, WeatherState,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatantCondition {
    Able,
    Fainted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CombatantScene {
    id: PokemonId,
    name: String,
    level: u8,
    primary_type: PokemonType,
    secondary_type: Option<PokemonType>,
    current_hp: u32,
    max_hp: u32,
    substitute_hp: Option<u32>,
    major_status: Option<MajorStatus>,
    stages: StatStages,
    condition: CombatantCondition,
}

impl CombatantScene {
    pub const fn id(&self) -> &PokemonId {
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

    pub const fn current_hp(&self) -> u32 {
        self.current_hp
    }

    pub const fn max_hp(&self) -> u32 {
        self.max_hp
    }

    pub const fn substitute_hp(&self) -> Option<u32> {
        self.substitute_hp
    }

    pub const fn condition(&self) -> CombatantCondition {
        self.condition
    }

    pub const fn major_status(&self) -> Option<MajorStatus> {
        self.major_status
    }

    pub const fn stages(&self) -> StatStages {
        self.stages
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleScene {
    own: CombatantScene,
    opponent: CombatantScene,
    weather: Option<WeatherState>,
}

impl BattleScene {
    pub const fn own(&self) -> &CombatantScene {
        &self.own
    }

    pub const fn opponent(&self) -> &CombatantScene {
        &self.opponent
    }

    pub const fn weather(&self) -> Option<WeatherState> {
        self.weather
    }

    fn combatant_mut(&mut self, participant: Participant) -> &mut CombatantScene {
        match participant {
            Participant::Own => &mut self.own,
            Participant::Opponent => &mut self.opponent,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BattleCue {
    TurnStarted {
        turn: u32,
    },
    Switched {
        participant: Participant,
    },
    MoveUsed {
        participant: Participant,
        used_move: UsedMove,
    },
    DamageApplied {
        participant: Participant,
        amount: u32,
    },
    StatusApplied {
        participant: Participant,
        status: MajorStatus,
    },
    StatusFailed {
        participant: Participant,
        target: Participant,
        status: MajorStatusKind,
    },
    StatusPreventsAction {
        participant: Participant,
        status: MajorStatus,
    },
    StatusCured {
        participant: Participant,
        status: MajorStatusKind,
    },
    StatStageChanged {
        participant: Participant,
        stat: BattleStat,
        change: i8,
        stage: i8,
    },
    Healed {
        participant: Participant,
        amount: u32,
    },
    EffectFailed {
        participant: Participant,
        target: Participant,
    },
    ProtectionActivated {
        participant: Participant,
    },
    ProtectionFailed {
        participant: Participant,
    },
    MoveBlocked {
        participant: Participant,
        target: Participant,
    },
    SubstituteCreated {
        participant: Participant,
        substitute_hp: u32,
    },
    SubstituteBlocked {
        participant: Participant,
        target: Participant,
    },
    SubstituteDamaged {
        participant: Participant,
        amount: u32,
        remaining_hp: u32,
    },
    SubstituteBroke {
        participant: Participant,
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
        participant: Participant,
        ability: Ability,
    },
    Flinched {
        participant: Participant,
    },
    Missed {
        participant: Participant,
    },
    Critical {
        participant: Participant,
    },
    Effectiveness {
        participant: Participant,
        effectiveness: TypeEffectiveness,
    },
    Fainted {
        participant: Participant,
    },
    ReplacementRequired {
        participant: Participant,
    },
    BattleFinished {
        outcome: ObservedBattleOutcome,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlaybackStep {
    scene: BattleScene,
    cue: BattleCue,
}

impl PlaybackStep {
    pub const fn scene(&self) -> &BattleScene {
        &self.scene
    }

    pub const fn cue(&self) -> &BattleCue {
        &self.cue
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayError {
    InvalidStatStage {
        participant: Participant,
        stage: i8,
    },
    EventTargetsInactivePokemon {
        participant: Participant,
        expected: PokemonId,
        actual: PokemonId,
    },
    FaintedWithHp {
        participant: Participant,
        current_hp: u32,
    },
    FinalSceneMismatch {
        reduced: Box<BattleScene>,
        expected: Box<BattleScene>,
    },
}

pub fn scene_from_observation(observation: &BattleObservation) -> BattleScene {
    let own = &observation.own().members()[observation.own().active_slot().index()];
    BattleScene {
        own: scene_from_pokemon(own),
        opponent: scene_from_revealed(observation.opponent().active()),
        weather: observation.weather(),
    }
}

pub fn reduce_transition(transition: &BattleTransition) -> Result<Vec<PlaybackStep>, ReplayError> {
    reduce_events(
        scene_from_observation(transition.before()),
        transition.events(),
        scene_from_observation(transition.after()),
    )
}

fn reduce_events(
    scene: BattleScene,
    events: &[BattleEvent],
    expected: BattleScene,
) -> Result<Vec<PlaybackStep>, ReplayError> {
    let mut reducer = BattleSceneReducer { scene };
    let mut steps = Vec::new();
    for event in events {
        if let Some(step) = reducer.apply(event)? {
            steps.push(step);
        }
    }
    if reducer.scene != expected {
        return Err(ReplayError::FinalSceneMismatch {
            reduced: Box::new(reducer.scene),
            expected: Box::new(expected),
        });
    }
    Ok(steps)
}

struct BattleSceneReducer {
    scene: BattleScene,
}

impl BattleSceneReducer {
    fn apply(&mut self, event: &BattleEvent) -> Result<Option<PlaybackStep>, ReplayError> {
        let cue = match event {
            BattleEvent::OwnCommandAccepted { .. }
            | BattleEvent::OpponentCommandCommitted
            | BattleEvent::OwnPpSpent { .. } => return Ok(None),
            BattleEvent::TurnStarted { turn } => BattleCue::TurnStarted { turn: *turn },
            BattleEvent::OwnSwitched { pokemon, .. } => {
                self.scene.own = scene_from_combatant(pokemon);
                BattleCue::Switched {
                    participant: Participant::Own,
                }
            }
            BattleEvent::OpponentSwitched { pokemon } => {
                self.scene.opponent = scene_from_combatant(pokemon);
                BattleCue::Switched {
                    participant: Participant::Opponent,
                }
            }
            BattleEvent::MoveUsed {
                participant,
                pokemon,
                used_move,
            } => {
                self.ensure_active(*participant, pokemon)?;
                BattleCue::MoveUsed {
                    participant: *participant,
                    used_move: used_move.clone(),
                }
            }
            BattleEvent::Damage {
                target,
                pokemon,
                amount,
                remaining_hp,
                ..
            } => {
                self.ensure_active(*target, pokemon)?;
                let combatant = self.scene.combatant_mut(*target);
                combatant.current_hp = *remaining_hp;
                combatant.condition = if *remaining_hp == 0 {
                    CombatantCondition::Fainted
                } else {
                    CombatantCondition::Able
                };
                BattleCue::DamageApplied {
                    participant: *target,
                    amount: *amount,
                }
            }
            BattleEvent::StatusApplied {
                participant,
                pokemon,
                status,
            } => {
                self.ensure_active(*participant, pokemon)?;
                self.scene.combatant_mut(*participant).major_status = Some(*status);
                BattleCue::StatusApplied {
                    participant: *participant,
                    status: *status,
                }
            }
            BattleEvent::StatusFailed {
                participant,
                target,
                pokemon,
                status,
            } => {
                self.ensure_active(*target, pokemon)?;
                BattleCue::StatusFailed {
                    participant: *participant,
                    target: *target,
                    status: *status,
                }
            }
            BattleEvent::StatusPreventsAction {
                participant,
                pokemon,
                status,
            } => {
                self.ensure_active(*participant, pokemon)?;
                self.scene.combatant_mut(*participant).major_status = Some(*status);
                BattleCue::StatusPreventsAction {
                    participant: *participant,
                    status: *status,
                }
            }
            BattleEvent::StatusCured {
                participant,
                pokemon,
                status,
            } => {
                self.ensure_active(*participant, pokemon)?;
                self.scene.combatant_mut(*participant).major_status = None;
                BattleCue::StatusCured {
                    participant: *participant,
                    status: *status,
                }
            }
            BattleEvent::StatusAdvanced {
                participant,
                pokemon,
                status,
            } => {
                self.ensure_active(*participant, pokemon)?;
                self.scene.combatant_mut(*participant).major_status = Some(*status);
                return Ok(None);
            }
            BattleEvent::StatStageChanged {
                participant,
                pokemon,
                stat,
                change,
                stage,
            } => {
                self.ensure_active(*participant, pokemon)?;
                self.scene
                    .combatant_mut(*participant)
                    .stages
                    .set(*stat, *stage)
                    .map_err(|_| ReplayError::InvalidStatStage {
                        participant: *participant,
                        stage: *stage,
                    })?;
                BattleCue::StatStageChanged {
                    participant: *participant,
                    stat: *stat,
                    change: *change,
                    stage: *stage,
                }
            }
            BattleEvent::Healed {
                participant,
                pokemon,
                amount,
                current_hp,
            } => {
                self.ensure_active(*participant, pokemon)?;
                self.scene.combatant_mut(*participant).current_hp = *current_hp;
                BattleCue::Healed {
                    participant: *participant,
                    amount: *amount,
                }
            }
            BattleEvent::EffectFailed {
                participant,
                target,
                pokemon,
            } => {
                self.ensure_active(*target, pokemon)?;
                BattleCue::EffectFailed {
                    participant: *participant,
                    target: *target,
                }
            }
            BattleEvent::ProtectionActivated {
                participant,
                pokemon,
            } => {
                self.ensure_active(*participant, pokemon)?;
                BattleCue::ProtectionActivated {
                    participant: *participant,
                }
            }
            BattleEvent::ProtectionFailed {
                participant,
                pokemon,
            } => {
                self.ensure_active(*participant, pokemon)?;
                BattleCue::ProtectionFailed {
                    participant: *participant,
                }
            }
            BattleEvent::MoveBlocked {
                participant,
                target,
                pokemon,
            } => {
                self.ensure_active(*target, pokemon)?;
                BattleCue::MoveBlocked {
                    participant: *participant,
                    target: *target,
                }
            }
            BattleEvent::SubstituteCreated {
                participant,
                pokemon,
                substitute_hp,
                current_hp,
            } => {
                self.ensure_active(*participant, pokemon)?;
                let combatant = self.scene.combatant_mut(*participant);
                combatant.substitute_hp = Some(*substitute_hp);
                combatant.current_hp = *current_hp;
                BattleCue::SubstituteCreated {
                    participant: *participant,
                    substitute_hp: *substitute_hp,
                }
            }
            BattleEvent::SubstituteBlocked {
                participant,
                target,
                pokemon,
            } => {
                self.ensure_active(*target, pokemon)?;
                BattleCue::SubstituteBlocked {
                    participant: *participant,
                    target: *target,
                }
            }
            BattleEvent::SubstituteDamaged {
                participant,
                pokemon,
                amount,
                remaining_hp,
            } => {
                self.ensure_active(*participant, pokemon)?;
                self.scene.combatant_mut(*participant).substitute_hp =
                    (*remaining_hp > 0).then_some(*remaining_hp);
                BattleCue::SubstituteDamaged {
                    participant: *participant,
                    amount: *amount,
                    remaining_hp: *remaining_hp,
                }
            }
            BattleEvent::SubstituteBroke {
                participant,
                pokemon,
            } => {
                self.ensure_active(*participant, pokemon)?;
                self.scene.combatant_mut(*participant).substitute_hp = None;
                BattleCue::SubstituteBroke {
                    participant: *participant,
                }
            }
            BattleEvent::WeatherStarted {
                weather,
                turns_remaining,
            } => {
                self.scene.weather = Some(match turns_remaining {
                    Some(turns) => WeatherState::with_turns(*weather, *turns),
                    None => WeatherState::permanent(*weather),
                });
                BattleCue::WeatherStarted {
                    weather: *weather,
                    turns_remaining: *turns_remaining,
                }
            }
            BattleEvent::WeatherUpdated {
                weather,
                turns_remaining,
            } => {
                self.scene.weather = Some(WeatherState::with_turns(*weather, *turns_remaining));
                BattleCue::WeatherUpdated {
                    weather: *weather,
                    turns_remaining: *turns_remaining,
                }
            }
            BattleEvent::WeatherEnded { weather } => {
                self.scene.weather = None;
                BattleCue::WeatherEnded { weather: *weather }
            }
            BattleEvent::AbilityActivated {
                participant,
                pokemon,
                ability,
            } => {
                self.ensure_active(*participant, pokemon)?;
                BattleCue::AbilityActivated {
                    participant: *participant,
                    ability: *ability,
                }
            }
            BattleEvent::Flinched {
                participant,
                pokemon,
            } => {
                self.ensure_active(*participant, pokemon)?;
                BattleCue::Flinched {
                    participant: *participant,
                }
            }
            BattleEvent::Missed { participant, .. } => BattleCue::Missed {
                participant: *participant,
            },
            BattleEvent::Critical {
                target, pokemon, ..
            } => {
                self.ensure_active(*target, pokemon)?;
                BattleCue::Critical {
                    participant: *target,
                }
            }
            BattleEvent::Effectiveness {
                target,
                pokemon,
                effectiveness,
                ..
            } => {
                self.ensure_active(*target, pokemon)?;
                BattleCue::Effectiveness {
                    participant: *target,
                    effectiveness: *effectiveness,
                }
            }
            BattleEvent::Fainted {
                participant,
                pokemon,
            } => {
                self.ensure_active(*participant, pokemon)?;
                let combatant = self.scene.combatant_mut(*participant);
                if combatant.current_hp != 0 {
                    return Err(ReplayError::FaintedWithHp {
                        participant: *participant,
                        current_hp: combatant.current_hp,
                    });
                }
                combatant.condition = CombatantCondition::Fainted;
                BattleCue::Fainted {
                    participant: *participant,
                }
            }
            BattleEvent::ForcedReplacement { participant } => BattleCue::ReplacementRequired {
                participant: *participant,
            },
            BattleEvent::BattleFinished { outcome } => {
                BattleCue::BattleFinished { outcome: *outcome }
            }
        };
        Ok(Some(PlaybackStep {
            scene: self.scene.clone(),
            cue,
        }))
    }

    fn ensure_active(
        &self,
        participant: Participant,
        pokemon: &PokemonId,
    ) -> Result<(), ReplayError> {
        let active = match participant {
            Participant::Own => &self.scene.own,
            Participant::Opponent => &self.scene.opponent,
        };
        if active.id == *pokemon {
            Ok(())
        } else {
            Err(ReplayError::EventTargetsInactivePokemon {
                participant,
                expected: active.id.clone(),
                actual: pokemon.clone(),
            })
        }
    }
}

fn scene_from_pokemon(pokemon: &Pokemon) -> CombatantScene {
    CombatantScene {
        id: pokemon.id().clone(),
        name: pokemon.name().to_owned(),
        level: pokemon.level(),
        primary_type: pokemon.primary_type(),
        secondary_type: pokemon.secondary_type(),
        current_hp: pokemon.current_hp(),
        max_hp: pokemon.max_hp(),
        substitute_hp: pokemon.substitute_hp(),
        major_status: pokemon.major_status(),
        stages: pokemon.stages(),
        condition: condition(pokemon.current_hp()),
    }
}

fn scene_from_revealed(pokemon: &RevealedPokemonObservation) -> CombatantScene {
    CombatantScene {
        id: pokemon.id().clone(),
        name: pokemon.name().to_owned(),
        level: pokemon.level(),
        primary_type: pokemon.primary_type(),
        secondary_type: pokemon.secondary_type(),
        current_hp: pokemon.current_hp(),
        max_hp: pokemon.max_hp(),
        substitute_hp: pokemon.substitute_hp(),
        major_status: pokemon.major_status(),
        stages: pokemon.stages(),
        condition: condition(pokemon.current_hp()),
    }
}

fn scene_from_combatant(pokemon: &RevealedCombatant) -> CombatantScene {
    CombatantScene {
        id: pokemon.id().clone(),
        name: pokemon.name().to_owned(),
        level: pokemon.level(),
        primary_type: pokemon.primary_type(),
        secondary_type: pokemon.secondary_type(),
        current_hp: pokemon.current_hp(),
        max_hp: pokemon.max_hp(),
        substitute_hp: pokemon.substitute_hp(),
        major_status: pokemon.major_status(),
        stages: pokemon.stages(),
        condition: condition(pokemon.current_hp()),
    }
}

const fn condition(current_hp: u32) -> CombatantCondition {
    if current_hp == 0 {
        CombatantCondition::Fainted
    } else {
        CombatantCondition::Able
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combatant(id: &str, hp: u32) -> CombatantScene {
        CombatantScene {
            id: PokemonId::new(id).unwrap(),
            name: id.into(),
            level: 50,
            primary_type: PokemonType::Normal,
            secondary_type: None,
            current_hp: hp,
            max_hp: 100,
            substitute_hp: None,
            major_status: None,
            stages: StatStages::neutral(),
            condition: condition(hp),
        }
    }

    fn scene() -> BattleScene {
        BattleScene {
            own: combatant("own", 100),
            opponent: combatant("opponent", 100),
            weather: None,
        }
    }

    #[test]
    fn reducer_rejects_inactive_targets_fainting_with_hp_and_final_mismatch() {
        let mut reducer = BattleSceneReducer { scene: scene() };
        let inactive = reducer
            .apply(&BattleEvent::MoveUsed {
                participant: Participant::Own,
                pokemon: PokemonId::new("bench").unwrap(),
                used_move: UsedMove::Struggle,
            })
            .unwrap_err();
        assert!(matches!(
            inactive,
            ReplayError::EventTargetsInactivePokemon { .. }
        ));

        let fainted = reducer
            .apply(&BattleEvent::Fainted {
                participant: Participant::Opponent,
                pokemon: PokemonId::new("opponent").unwrap(),
            })
            .unwrap_err();
        assert!(matches!(fainted, ReplayError::FaintedWithHp { .. }));

        let critical = reducer
            .apply(&BattleEvent::Critical {
                participant: Participant::Opponent,
                target: Participant::Own,
                pokemon: PokemonId::new("own").unwrap(),
            })
            .unwrap()
            .unwrap();
        assert!(matches!(
            critical.cue(),
            BattleCue::Critical {
                participant: Participant::Own
            }
        ));

        let mut expected = scene();
        expected.own.current_hp = 99;
        let mismatch = reduce_events(scene(), &[], expected).unwrap_err();
        assert!(matches!(mismatch, ReplayError::FinalSceneMismatch { .. }));
        assert_eq!(reducer.scene.own.condition(), CombatantCondition::Able);
    }
}
