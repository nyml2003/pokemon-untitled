//! Deterministic roster construction from explicitly supplied game data and seed.

use std::collections::BTreeSet;

use battle_application::{
    Ability, Accuracy, BattleStat, BattleState, BattleUnit, BattleUnitId, EffectTarget, FormId,
    MAX_MOVES, MajorStatusKind, Move, MoveCategory, MoveEffect, MoveId, NationalDexId, PokemonType,
    Species, StageChanges, StatBlock, StatProjectionError, StatStages, TEAM_SIZE, Team,
    TrainingValues, ValidationError, Weather, WeatherAccuracyModifier, WeatherMoveModifier,
    calculate_gen3_stats,
};
use battle_ruleset::{BattleRuleset, RulesetError};
use game_data::{
    CurrentDataSet, DamageClass as DataDamageClass, MoveId as DataMoveId, PokemonFormId,
    TypeId as DataTypeId,
};
use game_foundation::{BattleId, CreatureId, ThinSliceContent, TrainerId};

use crate::{BattleSource, BattleStartRequest};

const ROSTER_SIZE: usize = TEAM_SIZE * 2;
const DEMO_LEVEL: u8 = 50;
const FIRST_NATIONAL_POKEMON: u32 = 1;
const LAST_KANTO_POKEMON: u32 = 151;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RosterError {
    NotEnoughEligiblePokemon {
        required: usize,
        actual: usize,
    },
    MissingPokemon(PokemonFormId),
    MissingMove(DataMoveId),
    MoveNotLearnable {
        pokemon: PokemonFormId,
        battle_move: DataMoveId,
    },
    MissingType(DataTypeId),
    UnsupportedType {
        id: DataTypeId,
        identifier: String,
    },
    MissingMovePower(DataMoveId),
    MissingMovePp(DataMoveId),
    MissingPokemonSpecies(String),
    MissingTrainer(TrainerId),
    MissingWildOpponent(BattleId),
    InvalidProductPartySize {
        actual: usize,
    },
    UnsupportedTrainerRoster {
        trainer: TrainerId,
        actual: usize,
    },
    ProductParticipantNotFirst(CreatureId),
    NoUsableProductMove {
        pokemon: PokemonFormId,
        level: u8,
        required_pp: u8,
    },
    InvalidPresetSize {
        actual: usize,
    },
    UnknownMoveIdentifier {
        identifier: String,
    },
    Ruleset(RulesetError),
    InvalidBattleModel(ValidationError),
    InvalidTraining(StatProjectionError),
}

impl From<ValidationError> for RosterError {
    fn from(error: ValidationError) -> Self {
        Self::InvalidBattleModel(error)
    }
}

impl From<StatProjectionError> for RosterError {
    fn from(error: StatProjectionError) -> Self {
        Self::InvalidTraining(error)
    }
}

impl From<RulesetError> for RosterError {
    fn from(error: RulesetError) -> Self {
        Self::Ruleset(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RosterMember {
    pokemon_form_id: PokemonFormId,
    level: u8,
    move_ids: Vec<DataMoveId>,
    training: TrainingValues,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EligiblePokemon {
    pokemon_form_id: PokemonFormId,
    move_ids: Vec<DataMoveId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DemoSpriteManifest {
    player: Vec<PokemonFormId>,
    opponent: Vec<PokemonFormId>,
}

impl DemoSpriteManifest {
    pub fn player(&self) -> &[PokemonFormId] {
        &self.player
    }

    pub fn opponent(&self) -> &[PokemonFormId] {
        &self.opponent
    }
}

pub fn demo_teams(data: &CurrentDataSet, seed: u64) -> Result<(Team, Team), RosterError> {
    let members = random_members(data, seed)?;
    Ok((
        build_team(data, "player", &members[..TEAM_SIZE])?,
        build_team(data, "rival", &members[TEAM_SIZE..])?,
    ))
}

/// 用给定种子生成一支六人随机队伍。
pub fn random_team(data: &CurrentDataSet, seed: u64, prefix: &str) -> Result<Team, RosterError> {
    let members = random_members(data, seed)?;
    let roster = members.into_iter().take(TEAM_SIZE).collect::<Vec<_>>();
    build_team(data, prefix, &roster)
}

/// 从双方队伍派生精灵资源清单，供战斗资源加载使用。
pub fn demo_manifest_from_teams(player: &Team, opponent: &Team) -> DemoSpriteManifest {
    DemoSpriteManifest {
        player: player
            .members()
            .iter()
            .map(|unit| PokemonFormId(unit.species().form_id().value()))
            .collect(),
        opponent: opponent
            .members()
            .iter()
            .map(|unit| PokemonFormId(unit.species().form_id().value()))
            .collect(),
    }
}

pub(crate) fn product_teams(
    data: &CurrentDataSet,
    content: &ThinSliceContent,
    ruleset: &BattleRuleset,
    request: &BattleStartRequest,
) -> Result<(Team, Team), RosterError> {
    if request.player_party().len() != 1 {
        return Err(RosterError::InvalidProductPartySize {
            actual: request.player_party().len(),
        });
    }
    let player = request
        .player_party()
        .first()
        .ok_or(RosterError::InvalidProductPartySize { actual: 0 })?;
    if player.id() != request.participant() {
        return Err(RosterError::ProductParticipantNotFirst(
            request.participant().clone(),
        ));
    }
    let template = content
        .creature(player.template())
        .ok_or_else(|| RosterError::MissingPokemonSpecies(player.template().as_str().to_owned()))?;
    let player_form = ruleset.resolve_default_form(data, template.species())?;
    let builder = ProductRosterBuilder { data, ruleset };
    let player_team = builder.team(
        "product-player",
        player_form,
        PRODUCT_PLAYER_LEVEL,
        u32::from(template.max_hp()),
        u32::from(player.hp()),
        Some(player.pp()),
    )?;

    let (opponent_species, opponent_level) = match request.source() {
        BattleSource::Wild { .. } => {
            let definition = content
                .battle(request.battle())
                .ok_or_else(|| RosterError::MissingWildOpponent(request.battle().clone()))?;
            let opponent = definition
                .wild_opponent()
                .ok_or_else(|| RosterError::MissingWildOpponent(request.battle().clone()))?;
            (opponent.species(), opponent.level())
        }
        BattleSource::Trainer { trainer, .. } => {
            let definition = content
                .trainer(trainer)
                .ok_or_else(|| RosterError::MissingTrainer(trainer.clone()))?;
            if definition.pokemon().len() != 1 {
                return Err(RosterError::UnsupportedTrainerRoster {
                    trainer: trainer.clone(),
                    actual: definition.pokemon().len(),
                });
            }
            let member = definition.pokemon().first().ok_or_else(|| {
                RosterError::UnsupportedTrainerRoster {
                    trainer: trainer.clone(),
                    actual: 0,
                }
            })?;
            (member.species(), member.level())
        }
    };
    let opponent_form = ruleset.resolve_default_form(data, opponent_species)?;
    let opponent_team = builder.team(
        "product-opponent",
        opponent_form,
        opponent_level,
        calculated_max_hp(data, opponent_form, opponent_level)?,
        calculated_max_hp(data, opponent_form, opponent_level)?,
        None,
    )?;
    Ok((player_team, opponent_team))
}

pub(crate) fn validate_product_content(
    data: &CurrentDataSet,
    content: &ThinSliceContent,
    ruleset: &BattleRuleset,
) -> Result<(), RosterError> {
    for creature in content.creatures() {
        ruleset.validate_member(data, creature.species(), PRODUCT_PLAYER_LEVEL)?;
    }
    for battle in content.battles() {
        if let Some(opponent) = battle.wild_opponent() {
            ruleset.validate_member(data, opponent.species(), opponent.level())?;
        }
    }
    for trainer in content.trainers() {
        for member in trainer.pokemon() {
            ruleset.validate_member(data, member.species(), member.level())?;
        }
    }
    Ok(())
}

pub fn sprite_manifest(
    data: &CurrentDataSet,
    seed: u64,
) -> Result<DemoSpriteManifest, RosterError> {
    let members = random_members(data, seed)?;
    Ok(DemoSpriteManifest {
        player: members[..TEAM_SIZE]
            .iter()
            .map(|member| member.pokemon_form_id)
            .collect(),
        opponent: members[TEAM_SIZE..]
            .iter()
            .map(|member| member.pokemon_form_id)
            .collect(),
    })
}

/// 调试队伍中的一只宝可梦：全国图鉴编号与固定招式 identifier。
pub struct DebugPokemon {
    pub dex: u32,
    pub moves: &'static [&'static str],
}

/// 一组预置调试队伍，招式固定以便针对特定技能组合测试。
pub struct DebugTeamPreset {
    pub name: &'static str,
    pub player: &'static [DebugPokemon],
    pub opponent: &'static [DebugPokemon],
}

/// 晴天队：晴天手开晴后日光束瞬发、火系威力翻倍。
const SUNNY_ROSTER: &[DebugPokemon] = &[
    fire(
        6,
        &["overheat", "fire-blast", "flamethrower", "double-edge"],
    ),
    fire(59, &["sunny-day", "overheat", "fire-blast", "flamethrower"]),
    fire(38, &["sunny-day", "flamethrower", "fire-blast", "overheat"]),
    fire(78, &["sunny-day", "solar-beam", "fire-blast", "overheat"]),
    grass(3, &["sunny-day", "solar-beam", "giga-drain", "sludge-bomb"]),
    grass(103, &["sunny-day", "solar-beam", "giga-drain", "psychic"]),
];

/// 雨天队：雨天手开雨后水系威力翻倍、打雷必中。
const RAINY_ROSTER: &[DebugPokemon] = &[
    water(9, &["rain-dance", "surf", "blizzard", "waterfall"]),
    water(130, &["rain-dance", "hydro-pump", "surf", "waterfall"]),
    water(131, &["rain-dance", "surf", "ice-beam", "blizzard"]),
    water(134, &["rain-dance", "surf", "waterfall", "blizzard"]),
    electric(
        25,
        &["rain-dance", "thunder", "thunderbolt", "thunder-punch"],
    ),
    electric(145, &["rain-dance", "thunder", "thunderbolt", "drill-peck"]),
];

/// 沙暴队：沙暴手开沙后岩石系特防提升，地面系免疫沙暴伤害。
const SAND_ROSTER: &[DebugPokemon] = &[
    ground(
        112,
        &["sandstorm", "earthquake", "rock-slide", "brick-break"],
    ),
    rock(
        76,
        &["sandstorm", "earthquake", "rock-slide", "double-edge"],
    ),
    rock(142, &["sandstorm", "rock-slide", "aerial-ace", "fly"]),
    ground(28, &["sandstorm", "earthquake", "slash", "double-edge"]),
    ground(51, &["earthquake", "dig", "rock-slide", "double-edge"]),
    rock(95, &["sandstorm", "rock-slide", "earthquake", "dig"]),
];

/// 冰雹队：冰雹手开雹后冰系受益、暴风雪必中。
const HAIL_ROSTER: &[DebugPokemon] = &[
    ice(144, &["hail", "blizzard", "ice-beam", "fly"]),
    ice(87, &["hail", "ice-beam", "blizzard", "surf"]),
    water(131, &["hail", "blizzard", "ice-beam", "surf"]),
    ice(124, &["hail", "ice-beam", "blizzard", "psychic"]),
    ice(91, &["hail", "ice-beam", "surf", "blizzard"]),
    water(9, &["hail", "ice-beam", "blizzard", "surf"]),
];

/// 调试窗口可切换的预置测试队伍组合。
pub const DEBUG_PRESETS: &[DebugTeamPreset] = &[
    DebugTeamPreset {
        name: "晴天队",
        player: SUNNY_ROSTER,
        opponent: RAINY_ROSTER,
    },
    DebugTeamPreset {
        name: "雨天队",
        player: RAINY_ROSTER,
        opponent: SAND_ROSTER,
    },
    DebugTeamPreset {
        name: "沙暴队",
        player: SAND_ROSTER,
        opponent: HAIL_ROSTER,
    },
    DebugTeamPreset {
        name: "冰雹队",
        player: HAIL_ROSTER,
        opponent: SUNNY_ROSTER,
    },
    DebugTeamPreset {
        name: "传说队",
        player: &[
            psychic(150, &["psychic", "calm-mind", "barrier", "hyper-beam"]),
            dragon(149, &["dragon-claw", "dragon-rage", "aerial-ace", "fly"]),
            water(130, &["hydro-pump", "surf", "waterfall", "hyper-beam"]),
            ghost(94, &["shadow-ball", "sludge-bomb", "lick", "night-shade"]),
            fire(6, &["overheat", "fire-blast", "flamethrower", "fly"]),
            electric(
                25,
                &["thunderbolt", "thunder", "thunder-punch", "thunder-wave"],
            ),
        ],
        opponent: &[
            psychic(151, &["psychic", "swift", "calm-mind", "hyper-beam"]),
            psychic(65, &["psychic", "confusion", "calm-mind", "recover"]),
            ground(
                112,
                &["earthquake", "rock-slide", "double-edge", "brick-break"],
            ),
            fighting(
                68,
                &["focus-punch", "cross-chop", "karate-chop", "brick-break"],
            ),
            normal(143, &["body-slam", "hyper-beam", "earthquake", "rest"]),
            fire(59, &["overheat", "fire-blast", "flamethrower", "ember"]),
        ],
    },
];

const fn fire(dex: u32, moves: &'static [&'static str]) -> DebugPokemon {
    DebugPokemon { dex, moves }
}

const fn water(dex: u32, moves: &'static [&'static str]) -> DebugPokemon {
    DebugPokemon { dex, moves }
}

const fn electric(dex: u32, moves: &'static [&'static str]) -> DebugPokemon {
    DebugPokemon { dex, moves }
}

const fn grass(dex: u32, moves: &'static [&'static str]) -> DebugPokemon {
    DebugPokemon { dex, moves }
}

const fn psychic(dex: u32, moves: &'static [&'static str]) -> DebugPokemon {
    DebugPokemon { dex, moves }
}

const fn dragon(dex: u32, moves: &'static [&'static str]) -> DebugPokemon {
    DebugPokemon { dex, moves }
}

const fn ghost(dex: u32, moves: &'static [&'static str]) -> DebugPokemon {
    DebugPokemon { dex, moves }
}

const fn ground(dex: u32, moves: &'static [&'static str]) -> DebugPokemon {
    DebugPokemon { dex, moves }
}

const fn fighting(dex: u32, moves: &'static [&'static str]) -> DebugPokemon {
    DebugPokemon { dex, moves }
}

const fn normal(dex: u32, moves: &'static [&'static str]) -> DebugPokemon {
    DebugPokemon { dex, moves }
}
const fn rock(dex: u32, moves: &'static [&'static str]) -> DebugPokemon {
    DebugPokemon { dex, moves }
}

const fn ice(dex: u32, moves: &'static [&'static str]) -> DebugPokemon {
    DebugPokemon { dex, moves }
}

/// 用一组预置队伍构建双方队伍与精灵清单。
pub fn debug_teams(
    data: &CurrentDataSet,
    preset: &DebugTeamPreset,
) -> Result<(Team, Team, DemoSpriteManifest), RosterError> {
    let player = fixed_team(data, preset.player)?;
    let opponent = fixed_team(data, preset.opponent)?;
    let manifest = DemoSpriteManifest {
        player: preset
            .player
            .iter()
            .map(|entry| PokemonFormId(entry.dex))
            .collect(),
        opponent: preset
            .opponent
            .iter()
            .map(|entry| PokemonFormId(entry.dex))
            .collect(),
    };
    Ok((player, opponent, manifest))
}

/// 从固定配置构建一支队伍，招式按 identifier 解析并校验该物种可学。
fn fixed_team(data: &CurrentDataSet, roster: &[DebugPokemon]) -> Result<Team, RosterError> {
    if roster.len() != TEAM_SIZE {
        return Err(RosterError::InvalidPresetSize {
            actual: roster.len(),
        });
    }
    let members = roster
        .iter()
        .map(|entry| {
            let form = PokemonFormId(entry.dex);
            let move_ids = entry
                .moves
                .iter()
                .map(|identifier| {
                    let record = data
                        .move_iter()
                        .find(|record| record.identifier == *identifier)
                        .ok_or_else(|| RosterError::UnknownMoveIdentifier {
                            identifier: (*identifier).to_owned(),
                        })?;
                    if !data.can_learn_at_level(form, record.id, DEMO_LEVEL) {
                        return Err(RosterError::MoveNotLearnable {
                            pokemon: form,
                            battle_move: record.id,
                        });
                    }
                    Ok(record.id)
                })
                .collect::<Result<Vec<_>, _>>()?;
            if move_ids.is_empty() {
                return Err(RosterError::MissingPokemon(form));
            }
            Ok(RosterMember {
                pokemon_form_id: form,
                level: DEMO_LEVEL,
                move_ids,
                training: TrainingValues::perfect_untrained(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    build_team(data, "preset", &members)
}

fn random_members(data: &CurrentDataSet, seed: u64) -> Result<Vec<RosterMember>, RosterError> {
    let mut seen_names = BTreeSet::new();
    let mut eligible = data
        .pokemon_iter()
        .filter_map(|pokemon| {
            if !is_kanto_default_form(pokemon.species_id.0, pokemon.id.0)
                || !pokemon.types.iter().all(|id| is_supported_type(data, *id))
                || !seen_names.insert(pokemon.display_name.localized.clone())
            {
                return None;
            }
            let move_ids = compatible_move_ids(data, pokemon.id);
            (move_ids.len() >= MAX_MOVES).then_some(EligiblePokemon {
                pokemon_form_id: pokemon.id,
                move_ids,
            })
        })
        .collect::<Vec<_>>();
    if eligible.len() < ROSTER_SIZE {
        return Err(RosterError::NotEnoughEligiblePokemon {
            required: ROSTER_SIZE,
            actual: eligible.len(),
        });
    }

    let mut rng = RosterRng::new(seed);
    rng.shuffle(&mut eligible);
    eligible
        .into_iter()
        .take(ROSTER_SIZE)
        .map(|mut pokemon| {
            rng.shuffle(&mut pokemon.move_ids);
            pokemon.move_ids.truncate(MAX_MOVES);
            Ok(RosterMember {
                pokemon_form_id: pokemon.pokemon_form_id,
                level: DEMO_LEVEL,
                move_ids: pokemon.move_ids,
                training: TrainingValues::perfect_untrained(),
            })
        })
        .collect()
}

/// 只接受全国图鉴 1-151（关都）范围内的默认形态。
const fn is_kanto_default_form(species_id: u32, form_id: u32) -> bool {
    species_id >= FIRST_NATIONAL_POKEMON
        && species_id <= LAST_KANTO_POKEMON
        && form_id == species_id
}

fn compatible_move_ids(data: &CurrentDataSet, pokemon: PokemonFormId) -> Vec<DataMoveId> {
    data.learnset(pokemon)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            data.can_learn_at_level(pokemon, entry.move_id, DEMO_LEVEL)
                .then_some(())?;
            let battle_move = data.move_by_id(entry.move_id)?;
            battle_move.pp.filter(|pp| *pp > 0)?;
            (battle_move.power.is_some() || move_effect(battle_move).is_some()).then_some(())?;
            is_supported_type(data, battle_move.move_type).then_some(entry.move_id)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn is_supported_type(data: &CurrentDataSet, id: DataTypeId) -> bool {
    data.type_by_id(id)
        .is_some_and(|record| is_supported_type_name(&record.identifier))
}

fn is_supported_type_name(identifier: &str) -> bool {
    matches!(
        identifier,
        "normal"
            | "fighting"
            | "flying"
            | "poison"
            | "ground"
            | "rock"
            | "bug"
            | "ghost"
            | "steel"
            | "fire"
            | "water"
            | "grass"
            | "electric"
            | "psychic"
            | "ice"
            | "dragon"
            | "dark"
    )
}

fn build_team(
    data: &CurrentDataSet,
    prefix: &str,
    members: &[RosterMember],
) -> Result<Team, RosterError> {
    let members = members
        .iter()
        .map(|member| build_pokemon(data, prefix, member))
        .collect::<Result<Vec<_>, _>>()?;
    Team::new(members).map_err(Into::into)
}

fn build_pokemon(
    data: &CurrentDataSet,
    prefix: &str,
    member: &RosterMember,
) -> Result<BattleUnit, RosterError> {
    let record = data
        .pokemon(member.pokemon_form_id)
        .ok_or(RosterError::MissingPokemon(member.pokemon_form_id))?;
    let moves = member
        .move_ids
        .iter()
        .copied()
        .map(|id| {
            if !data.can_learn_at_level(member.pokemon_form_id, id, member.level) {
                return Err(RosterError::MoveNotLearnable {
                    pokemon: member.pokemon_form_id,
                    battle_move: id,
                });
            }
            battle_move(data, id)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let stats = record.base_stats;
    let calculated = calculate_gen3_stats(
        StatBlock::new(
            stats.hp,
            stats.attack,
            stats.defense,
            stats.special_attack,
            stats.special_defense,
            stats.speed,
        ),
        member.level,
        member.training,
    )?;
    let ability = record
        .abilities
        .iter()
        .filter(|entry| !entry.is_hidden)
        .find_map(|entry| data.ability_by_id(entry.ability_id))
        .and_then(battle_ability);
    let build = |ability: Option<Ability>| -> Result<BattleUnit, RosterError> {
        let base = record.base_stats;
        let base_stats = StatBlock::new(
            base.hp,
            base.attack,
            base.defense,
            base.special_attack,
            base.special_defense,
            base.speed,
        );
        let types = record
            .types
            .iter()
            .map(|id| battle_type(data, *id))
            .collect::<Result<Vec<_>, _>>()?;
        let default_abilities = record
            .abilities
            .iter()
            .filter(|entry| !entry.is_hidden)
            .filter_map(|entry| data.ability_by_id(entry.ability_id))
            .filter_map(battle_ability)
            .collect();
        let species = Species::new(
            &record.display_name.localized,
            base_stats,
            NationalDexId::new(record.id.0 as u16),
            FormId::new(record.id.0),
            types,
            default_abilities,
        )?;
        let state = BattleState::new(
            member.level,
            calculated.battle(),
            calculated.max_hp(),
            calculated.max_hp(),
            moves,
            ability.into_iter().collect(),
            None,
            StatStages::neutral(),
        )?;
        Ok(BattleUnit::new(
            species,
            BattleUnitId::new(format!("{prefix}-form-{}", member.pokemon_form_id.0))?,
            state,
        )?)
    };
    build(ability)
}

fn battle_ability(record: &game_data::AbilityRecord) -> Option<Ability> {
    match record.identifier.as_str() {
        "air-lock" => Some(Ability::AirLock),
        "arena-trap" => Some(Ability::ArenaTrap),
        "battle-armor" => Some(Ability::BattleArmor),
        "blaze" => Some(Ability::Blaze),
        "chlorophyll" => Some(Ability::Chlorophyll),
        "clear-body" => Some(Ability::ClearBody),
        "cloud-nine" => Some(Ability::CloudNine),
        "compound-eyes" => Some(Ability::CompoundEyes),
        "drizzle" => Some(Ability::Drizzle),
        "drought" => Some(Ability::Drought),
        "early-bird" => Some(Ability::EarlyBird),
        "flash-fire" => Some(Ability::FlashFire),
        "guts" => Some(Ability::Guts),
        "huge-power" => Some(Ability::HugePower),
        "hyper-cutter" => Some(Ability::HyperCutter),
        "hustle" => Some(Ability::Hustle),
        "immunity" => Some(Ability::Immunity),
        "intimidate" => Some(Ability::Intimidate),
        "inner-focus" => Some(Ability::InnerFocus),
        "keen-eye" => Some(Ability::KeenEye),
        "insomnia" => Some(Ability::Insomnia),
        "levitate" => Some(Ability::Levitate),
        "limber" => Some(Ability::Limber),
        "liquid-ooze" => Some(Ability::LiquidOoze),
        "magma-armor" => Some(Ability::MagmaArmor),
        "marvel-scale" => Some(Ability::MarvelScale),
        "natural-cure" => Some(Ability::NaturalCure),
        "overgrow" => Some(Ability::Overgrow),
        "pressure" => Some(Ability::Pressure),
        "pure-power" => Some(Ability::PurePower),
        "rain-dish" => Some(Ability::RainDish),
        "rock-head" => Some(Ability::RockHead),
        "sand-stream" => Some(Ability::SandStream),
        "sand-veil" => Some(Ability::SandVeil),
        "serene-grace" => Some(Ability::SereneGrace),
        "shell-armor" => Some(Ability::ShellArmor),
        "shed-skin" => Some(Ability::ShedSkin),
        "shield-dust" => Some(Ability::ShieldDust),
        "shadow-tag" => Some(Ability::ShadowTag),
        "synchronize" => Some(Ability::Synchronize),
        "speed-boost" => Some(Ability::SpeedBoost),
        "swift-swim" => Some(Ability::SwiftSwim),
        "swarm" => Some(Ability::Swarm),
        "thick-fat" => Some(Ability::ThickFat),
        "torrent" => Some(Ability::Torrent),
        "vital-spirit" => Some(Ability::VitalSpirit),
        "volt-absorb" => Some(Ability::VoltAbsorb),
        "water-absorb" => Some(Ability::WaterAbsorb),
        "water-veil" => Some(Ability::WaterVeil),
        "white-smoke" => Some(Ability::WhiteSmoke),
        _ => None,
    }
}

fn battle_move(data: &CurrentDataSet, id: DataMoveId) -> Result<Move, RosterError> {
    let record = data.move_by_id(id).ok_or(RosterError::MissingMove(id))?;
    let pp = record.pp.ok_or(RosterError::MissingMovePp(id))?;
    battle_move_with_pp(data, id, pp)
}

fn battle_move_with_pp(
    data: &CurrentDataSet,
    id: DataMoveId,
    current_pp: u8,
) -> Result<Move, RosterError> {
    let record = data.move_by_id(id).ok_or(RosterError::MissingMove(id))?;
    let pp = record.pp.ok_or(RosterError::MissingMovePp(id))?;
    let category = battle_move_category(record.damage_class);
    let effect = move_effect(record).unwrap_or(MoveEffect::None);
    let power = match category {
        MoveCategory::Status => 0,
        MoveCategory::Physical | MoveCategory::Special => record
            .power
            .or_else(|| effect.permits_zero_power().then_some(0))
            .ok_or(RosterError::MissingMovePower(id))?,
    };
    let accuracy = record
        .accuracy
        .map(Accuracy::percent)
        .transpose()?
        .unwrap_or(Accuracy::AlwaysHit);
    Move::new_with_category_and_effect(
        MoveId::new(format!("pokeapi-move-{}", id.0))?,
        &record.display_name.localized,
        vec![battle_type(data, record.move_type)?],
        category,
        power,
        accuracy,
        pp,
        current_pp,
        record.priority,
        vec![effect],
    )
    .map(|battle_move| {
        let battle_move = match weather_accuracy(record) {
            Some(modifier) => battle_move.with_weather_accuracy(modifier),
            None => battle_move,
        };
        match weather_move(record) {
            Some(modifier) => battle_move.with_weather_move(modifier),
            None => battle_move,
        }
    })
    .map_err(Into::into)
}

fn weather_accuracy(record: &game_data::MoveRecord) -> Option<WeatherAccuracyModifier> {
    match record.identifier.as_str() {
        "thunder" => Some(WeatherAccuracyModifier::Thunder),
        _ => None,
    }
}

fn weather_move(record: &game_data::MoveRecord) -> Option<WeatherMoveModifier> {
    match record.identifier.as_str() {
        "weather-ball" => Some(WeatherMoveModifier::WeatherBall),
        _ => None,
    }
}

fn move_effect(record: &game_data::MoveRecord) -> Option<MoveEffect> {
    match record.effect_id? {
        2 => major_status_effect(record, MajorStatusKind::Sleep),
        3 => major_status_effect(record, MajorStatusKind::Poison),
        34 => major_status_effect(record, MajorStatusKind::BadlyPoisoned),
        5 | 168 => major_status_effect(record, MajorStatusKind::Burn),
        6 => major_status_effect(record, MajorStatusKind::Freeze),
        7 | 68 => major_status_effect(record, MajorStatusKind::Paralysis),
        11 => stage_effect(EffectTarget::User, BattleStat::Attack, 1),
        12 => stage_effect(EffectTarget::User, BattleStat::Defense, 1),
        17 => stage_effect(EffectTarget::User, BattleStat::Evasion, 1),
        19 => stage_effect(EffectTarget::Opponent, BattleStat::Attack, -1),
        20 => stage_effect(EffectTarget::Opponent, BattleStat::Defense, -1),
        21 => stage_effect(EffectTarget::Opponent, BattleStat::Speed, -1),
        24 => stage_effect(EffectTarget::Opponent, BattleStat::Accuracy, -1),
        25 => stage_effect(EffectTarget::Opponent, BattleStat::Evasion, -1),
        26 => Some(MoveEffect::haze()),
        32 | 159 => MoveEffect::flinch_target(record.effect_chance.unwrap_or(100)).ok(),
        42 => Some(MoveEffect::fixed_damage_amount(40)),
        69 => stage_effect_with_chance(record, EffectTarget::Opponent, BattleStat::Attack, -1),
        70 => stage_effect_with_chance(record, EffectTarget::Opponent, BattleStat::Defense, -1),
        71 => stage_effect_with_chance(record, EffectTarget::Opponent, BattleStat::Speed, -1),
        72 => stage_effect_with_chance(
            record,
            EffectTarget::Opponent,
            BattleStat::SpecialAttack,
            -1,
        ),
        73 => stage_effect_with_chance(
            record,
            EffectTarget::Opponent,
            BattleStat::SpecialDefense,
            -1,
        ),
        74 => stage_effect_with_chance(record, EffectTarget::Opponent, BattleStat::Accuracy, -1),
        33 => MoveEffect::heal_user(1, 2).ok(),
        38 => Some(MoveEffect::rest()),
        4 => MoveEffect::drain_user(1, 2).ok(),
        52 => stage_effect(EffectTarget::User, BattleStat::Defense, 2),
        53 => stage_effect(EffectTarget::User, BattleStat::Speed, 2),
        54 => stage_effect(EffectTarget::User, BattleStat::SpecialAttack, 2),
        55 => stage_effect(EffectTarget::User, BattleStat::SpecialDefense, 2),
        59 => stage_effect(EffectTarget::Opponent, BattleStat::Attack, -2),
        60 => stage_effect(EffectTarget::Opponent, BattleStat::Defense, -2),
        61 => stage_effect(EffectTarget::Opponent, BattleStat::Speed, -2),
        62 => stage_effect(EffectTarget::Opponent, BattleStat::SpecialAttack, -2),
        63 => stage_effect(EffectTarget::Opponent, BattleStat::SpecialDefense, -2),
        80 => Some(MoveEffect::create_substitute()),
        88 => Some(MoveEffect::fixed_damage_user_level()),
        49 => MoveEffect::recoil_user(1, 4).ok(),
        112 => Some(MoveEffect::protect_user()),
        116 => Some(MoveEffect::start_weather(Weather::Sandstorm)),
        137 => Some(MoveEffect::start_weather(Weather::Rain)),
        138 => Some(MoveEffect::start_weather(Weather::Sun)),
        144 => Some(MoveEffect::copy_target_stages()),
        165 => Some(MoveEffect::start_weather(Weather::Hail)),
        199 => MoveEffect::recoil_user(1, 3).ok(),
        194 => Some(MoveEffect::refresh()),
        131 => Some(MoveEffect::fixed_damage_amount(20)),
        212 => StageChanges::new(0, 0, 1, 1, 0, 0, 0)
            .ok()
            .map(|changes| MoveEffect::change_stages(EffectTarget::User, changes)),
        278 => StageChanges::new(1, 0, 0, 0, 0, 1, 0)
            .ok()
            .map(|changes| MoveEffect::change_stages(EffectTarget::User, changes)),
        // Growth raises Special Attack by one stage in generation three.
        317 => stage_effect(EffectTarget::User, BattleStat::SpecialAttack, 1),
        328 => StageChanges::new(1, 0, 1, 0, 0, 0, 0)
            .ok()
            .map(|changes| MoveEffect::change_stages(EffectTarget::User, changes)),
        _ => None,
    }
}

fn major_status_effect(
    record: &game_data::MoveRecord,
    status: MajorStatusKind,
) -> Option<MoveEffect> {
    MoveEffect::inflict_major_status(status, record.effect_chance.unwrap_or(100)).ok()
}

fn stage_effect(target: EffectTarget, stat: BattleStat, amount: i8) -> Option<MoveEffect> {
    StageChanges::single(stat, amount)
        .ok()
        .map(|changes| MoveEffect::change_stages(target, changes))
}

fn stage_effect_with_chance(
    record: &game_data::MoveRecord,
    target: EffectTarget,
    stat: BattleStat,
    amount: i8,
) -> Option<MoveEffect> {
    MoveEffect::change_stages_with_chance(
        target,
        StageChanges::single(stat, amount).ok()?,
        record.effect_chance.unwrap_or(100),
    )
    .ok()
}

const fn battle_move_category(damage_class: DataDamageClass) -> MoveCategory {
    match damage_class {
        DataDamageClass::Physical => MoveCategory::Physical,
        DataDamageClass::Special => MoveCategory::Special,
        DataDamageClass::Status => MoveCategory::Status,
    }
}

fn battle_type(data: &CurrentDataSet, id: DataTypeId) -> Result<PokemonType, RosterError> {
    let record = data.type_by_id(id).ok_or(RosterError::MissingType(id))?;
    match record.identifier.as_str() {
        "normal" => Ok(PokemonType::Normal),
        "fighting" => Ok(PokemonType::Fighting),
        "flying" => Ok(PokemonType::Flying),
        "poison" => Ok(PokemonType::Poison),
        "ground" => Ok(PokemonType::Ground),
        "rock" => Ok(PokemonType::Rock),
        "bug" => Ok(PokemonType::Bug),
        "ghost" => Ok(PokemonType::Ghost),
        "steel" => Ok(PokemonType::Steel),
        "fire" => Ok(PokemonType::Fire),
        "water" => Ok(PokemonType::Water),
        "grass" => Ok(PokemonType::Grass),
        "electric" => Ok(PokemonType::Electric),
        "psychic" => Ok(PokemonType::Psychic),
        "ice" => Ok(PokemonType::Ice),
        "dragon" => Ok(PokemonType::Dragon),
        "dark" => Ok(PokemonType::Dark),
        identifier => Err(RosterError::UnsupportedType {
            id,
            identifier: identifier.to_owned(),
        }),
    }
}

const PRODUCT_PLAYER_LEVEL: u8 = 5;

struct ProductRosterBuilder<'a> {
    data: &'a CurrentDataSet,
    ruleset: &'a BattleRuleset,
}

impl ProductRosterBuilder<'_> {
    fn team(
        &self,
        prefix: &str,
        form: PokemonFormId,
        level: u8,
        max_hp: u32,
        current_hp: u32,
        current_pp: Option<u8>,
    ) -> Result<Team, RosterError> {
        let mut members = Vec::with_capacity(TEAM_SIZE);
        members.push(self.pokemon(prefix, form, level, max_hp, current_hp, current_pp)?);
        for slot in 1..TEAM_SIZE {
            members.push(self.pokemon(
                &format!("{prefix}-reserve-{slot}"),
                form,
                level,
                max_hp,
                0,
                Some(0),
            )?);
        }
        Team::new(members).map_err(Into::into)
    }

    fn pokemon(
        &self,
        prefix: &str,
        form: PokemonFormId,
        level: u8,
        max_hp: u32,
        current_hp: u32,
        current_pp: Option<u8>,
    ) -> Result<BattleUnit, RosterError> {
        self.ruleset.validate_form(self.data, form)?;
        let record = self
            .data
            .pokemon(form)
            .ok_or(RosterError::MissingPokemon(form))?;
        let candidate = self
            .data
            .learnset(form)
            .into_iter()
            .flatten()
            .filter(|entry| self.data.can_learn_at_level(form, entry.move_id, level))
            .find_map(|entry| {
                let record = self.data.move_by_id(entry.move_id)?;
                let maximum = record.pp?;
                let current = current_pp.unwrap_or(maximum);
                (maximum >= current
                    && self
                        .ruleset
                        .validate_move(self.data, form, entry.move_id, level)
                        .is_ok())
                .then_some((entry.move_id, current))
            })
            .ok_or(RosterError::NoUsableProductMove {
                pokemon: form,
                level,
                required_pp: current_pp.unwrap_or(0),
            })?;
        let moves = vec![battle_move_with_pp(self.data, candidate.0, candidate.1)?];
        let calculated = calculate_gen3_stats(
            StatBlock::new(
                record.base_stats.hp,
                record.base_stats.attack,
                record.base_stats.defense,
                record.base_stats.special_attack,
                record.base_stats.special_defense,
                record.base_stats.speed,
            ),
            level,
            TrainingValues::perfect_untrained(),
        )?;
        let ability = self.ruleset.select_ability(self.data, form)?;
        let id = BattleUnitId::new(format!("{prefix}-form-{}", form.0))?;
        let types = record
            .types
            .iter()
            .map(|id| battle_type(self.data, *id))
            .collect::<Result<Vec<_>, _>>()?;
        let species = Species::new(
            &record.display_name.localized,
            StatBlock::new(
                record.base_stats.hp,
                record.base_stats.attack,
                record.base_stats.defense,
                record.base_stats.special_attack,
                record.base_stats.special_defense,
                record.base_stats.speed,
            ),
            NationalDexId::new(form.0 as u16),
            FormId::new(form.0),
            types,
            vec![],
        )?;
        let state = BattleState::new(
            level,
            calculated.battle(),
            max_hp,
            current_hp,
            moves,
            vec![ability],
            None,
            StatStages::neutral(),
        )?;
        BattleUnit::new(species, id, state).map_err(Into::into)
    }
}

fn calculated_max_hp(
    data: &CurrentDataSet,
    form: PokemonFormId,
    level: u8,
) -> Result<u32, RosterError> {
    let record = data
        .pokemon(form)
        .ok_or(RosterError::MissingPokemon(form))?;
    Ok(calculate_gen3_stats(
        StatBlock::new(
            record.base_stats.hp,
            record.base_stats.attack,
            record.base_stats.defense,
            record.base_stats.special_attack,
            record.base_stats.special_defense,
            record.base_stats.speed,
        ),
        level,
        TrainingValues::perfect_untrained(),
    )?
    .max_hp())
}

struct RosterRng {
    state: u64,
}

impl RosterRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    fn next(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.state = value;
        value.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for upper in (1..values.len()).rev() {
            let index = (self.next() % (upper as u64 + 1)) as usize;
            values.swap(upper, index);
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/roster.rs"]
mod tests;
