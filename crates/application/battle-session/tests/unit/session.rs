use super::*;
use battle_application::Side;

#[test]
fn outcome_and_error_mappings_cover_every_semantic_variant() {
    assert_eq!(
        observed_outcome(BattleOutcome::Draw, Side::One),
        ObservedBattleOutcome::Draw
    );
    assert_eq!(
        observed_outcome(BattleOutcome::Winner(Side::One), Side::One),
        ObservedBattleOutcome::Winner(Participant::Own)
    );
    assert_eq!(
        observed_outcome(BattleOutcome::Escaped(Side::Two), Side::One),
        ObservedBattleOutcome::Escaped(Participant::Opponent)
    );
    assert_eq!(
        FinishedPrompt {
            outcome: ObservedBattleOutcome::Draw,
        }
        .outcome(),
        ObservedBattleOutcome::Draw
    );

    let battle = BattleError::BattleAlreadyFinished {
        outcome: BattleOutcome::Draw,
    };
    assert!(matches!(
        SessionError::from(CoordinatorError::Battle(battle)),
        SessionError::Battle(_)
    ));
    assert_eq!(
        SessionError::from(CoordinatorError::OpponentActionUnavailable),
        SessionError::OpponentActionUnavailable
    );
}
