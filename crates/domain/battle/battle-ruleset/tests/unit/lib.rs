use battle_domain::{
    Accuracy, Action, BattleCommand, BattlePhase, BattleStats, Move, MoveId, MoveSlot, Pokemon,
    PokemonId, PokemonType, Side, TEAM_SIZE, Team,
};
use game_data::{CurrentDataSet, PokemonFormId, SpeciesId};

use crate::{BattleRuleset, LEGACY_GEN3_RULESET_ID, LEGACY_GEN3_RULESET_REVISION, RulesetError};

fn replay_team(
    prefix: &str,
    lead_hp: u32,
    lead_attack: u16,
    lead_speed: u16,
) -> Result<Team, String> {
    let mut members = Vec::with_capacity(TEAM_SIZE);
    for index in 0..TEAM_SIZE {
        let current_hp = if index == 0 { lead_hp } else { 0 };
        let move_ = Move::new(
            MoveId::new(format!("{prefix}-move-{index}")).map_err(|error| format!("{error:?}"))?,
            "replay-strike",
            PokemonType::Normal,
            240,
            Accuracy::AlwaysHit,
            10,
            10,
            0,
        )
        .map_err(|error| format!("{error:?}"))?;
        members.push(
            Pokemon::new(
                PokemonId::new(format!("{prefix}-{index}"))
                    .map_err(|error| format!("{error:?}"))?,
                "replay-member",
                50,
                PokemonType::Normal,
                None,
                lead_hp,
                current_hp,
                BattleStats::new(lead_attack, 50, 50, 50, lead_speed)
                    .map_err(|error| format!("{error:?}"))?,
                vec![move_],
            )
            .map_err(|error| format!("{error:?}"))?,
        );
    }
    Team::new(members).map_err(|error| format!("{error:?}"))
}

#[test]
fn legacy_ruleset_freezes_its_identity_and_gen3_species_boundary() -> Result<(), String> {
    let ruleset = BattleRuleset::legacy_gen3_r1().map_err(|error| error.to_string())?;
    assert_eq!(ruleset.reference().id().as_str(), LEGACY_GEN3_RULESET_ID);
    assert_eq!(ruleset.reference().revision(), LEGACY_GEN3_RULESET_REVISION);
    assert!(ruleset.supports_species(SpeciesId(1)));
    assert!(ruleset.supports_species(SpeciesId(386)));
    assert!(!ruleset.supports_species(SpeciesId(387)));
    Ok(())
}

#[test]
fn legacy_ruleset_rejects_non_default_forms_and_out_of_range_species() -> Result<(), String> {
    let data = CurrentDataSet::embedded().map_err(|error| error.to_string())?;
    let ruleset = BattleRuleset::legacy_gen3_r1().map_err(|error| error.to_string())?;
    let out_of_range = data
        .pokemon_iter()
        .find(|record| record.species_id.0 > 386)
        .ok_or_else(|| String::from("fixture data has no out-of-range species"))?;
    assert!(ruleset.validate_form(&data, out_of_range.id).is_err());
    assert!(ruleset.validate_form(&data, PokemonFormId(1)).is_ok());
    Ok(())
}

#[test]
fn current_first_region_members_are_validated_and_modern_type_drift_is_rejected()
-> Result<(), String> {
    let data = CurrentDataSet::embedded().map_err(|error| error.to_string())?;
    let ruleset = BattleRuleset::legacy_gen3_r1().map_err(|error| error.to_string())?;
    for (species, level) in [("Treecko", 5), ("Torchic", 3), ("Bulbasaur", 5)] {
        let member = ruleset
            .validate_member(&data, species, level)
            .map_err(|error| format!("{species}: {error}"))?;
        assert!(member.move_id().0 > 0);
    }
    assert!(matches!(
        ruleset.resolve_default_form(&data, "Ralts"),
        Err(RulesetError::UnsupportedType { .. })
    ));
    Ok(())
}

#[test]
fn fixed_commands_replay_to_identical_events_and_final_state() -> Result<(), String> {
    let ruleset = BattleRuleset::legacy_gen3_r1().map_err(|error| error.to_string())?;
    let commands = [
        BattleCommand::new(
            Side::One,
            Action::UseMove(MoveSlot::new(0).map_err(|error| format!("{error:?}"))?),
        ),
        BattleCommand::new(
            Side::Two,
            Action::UseMove(MoveSlot::new(0).map_err(|error| format!("{error:?}"))?),
        ),
    ];
    let first = ruleset
        .replay(
            replay_team("first", 100, 250, 200)?,
            replay_team("second", 1, 50, 10)?,
            7,
            &commands,
        )
        .map_err(|error| format!("first replay: {error:?}"))?;
    let second = ruleset
        .replay(
            replay_team("first", 100, 250, 200)?,
            replay_team("second", 1, 50, 10)?,
            7,
            &commands,
        )
        .map_err(|error| format!("second replay: {error:?}"))?;
    assert_eq!(first, second);
    assert_eq!(first.ruleset(), ruleset.reference());
    assert_eq!(first.commands(), commands);
    assert!(matches!(first.phase(), BattlePhase::Finished(_)));
    Ok(())
}
