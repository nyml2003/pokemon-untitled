use battle_application::{Action, TEAM_SIZE};
use game_data::CurrentDataSet;

use super::{FactoryCommand, FactoryError, FactoryPhase, FactorySession};

fn session() -> Result<FactorySession, String> {
    let data = CurrentDataSet::embedded().map_err(|error| format!("data: {error:?}"))?;
    Ok(FactorySession::new(data))
}

fn transition(session: FactorySession, command: FactoryCommand) -> Result<FactorySession, String> {
    let (session, result) = session.transition(command);
    result
        .map(|_| session)
        .map_err(|error| format!("command rejected: {error:?}"))
}

fn start(session: FactorySession) -> Result<FactorySession, String> {
    transition(
        session,
        FactoryCommand::StartRun {
            seed: 0x5EED,
            target_streak: 3,
        },
    )
}

fn finish_battle(mut session: FactorySession) -> Result<FactorySession, String> {
    for _ in 0..2_000 {
        let snapshot = session.snapshot();
        if snapshot.phase() != FactoryPhase::Battle {
            break;
        }
        if session.is_finished() {
            return transition(session, FactoryCommand::LeaveFinishedBattle);
        }
        let actions = session.legal_player_actions();
        if actions.is_empty() {
            session = transition(session, FactoryCommand::AdvanceBattlePlayback)?;
            continue;
        }
        let action = actions
            .iter()
            .copied()
            .find(|action| matches!(action, Action::UseMove(_)))
            .or_else(|| actions.first().copied())
            .ok_or_else(|| String::from("battle offered no action"))?;
        session = transition(session, FactoryCommand::SubmitBattleAction(action))?;
    }
    Err(String::from(
        "battle did not complete within deterministic bound",
    ))
}

fn battle_outcome(session: FactorySession) -> Result<(FactorySession, bool), String> {
    let streak_before = session.snapshot().streak();
    let session = finish_battle(session)?;
    let won = session.snapshot().streak() > streak_before;
    Ok((session, won))
}

#[test]
fn starts_a_run_with_a_full_rental_team() -> Result<(), String> {
    let session = start(session()?)?;
    let snapshot = session.snapshot();
    assert_eq!(snapshot.phase(), FactoryPhase::Ready);
    assert_eq!(snapshot.streak(), 0);
    assert_eq!(snapshot.target_streak(), 3);
    assert_eq!(snapshot.rental().len(), TEAM_SIZE);
    for member in snapshot.rental() {
        assert_eq!(member.level(), 50);
        assert!(!member.name().is_empty());
        assert!(member.current_hp() > 0);
    }
    Ok(())
}

#[test]
fn rejects_zero_target_streak() -> Result<(), String> {
    let (session, result) = session()?.transition(FactoryCommand::StartRun {
        seed: 1,
        target_streak: 0,
    });
    assert!(matches!(result, Err(FactoryError::InvalidTargetStreak(0))));
    assert_eq!(session.snapshot().phase(), FactoryPhase::Ready);
    Ok(())
}

#[test]
fn swap_commands_require_swap_offer_phase() -> Result<(), String> {
    let (session, result) = session()?.transition(FactoryCommand::ConfirmSwap {
        rental_slot: 0,
        opponent_slot: 0,
    });
    assert!(matches!(
        result,
        Err(FactoryError::WrongPhase {
            expected: FactoryPhase::SwapOffer,
            actual: FactoryPhase::Ready
        })
    ));
    let (_, result) = session.transition(FactoryCommand::SkipSwap);
    assert!(matches!(
        result,
        Err(FactoryError::WrongPhase {
            expected: FactoryPhase::SwapOffer,
            actual: FactoryPhase::Ready
        })
    ));
    Ok(())
}

#[test]
fn same_seed_produces_the_same_rental_team() -> Result<(), String> {
    let left = start(session()?)?.snapshot().rental().to_vec();
    let right = start(session()?)?.snapshot().rental().to_vec();
    assert_eq!(left, right);
    Ok(())
}

#[test]
fn winning_battle_moves_to_swap_offer_and_increments_streak() -> Result<(), String> {
    for seed in 1..=64 {
        let session = transition(
            session()?,
            FactoryCommand::StartRun {
                seed,
                target_streak: 5,
            },
        )?;
        let session = transition(session, FactoryCommand::StartNextBattle)?;
        let (session, won) = battle_outcome(session)?;
        if !won {
            continue;
        }
        let snapshot = session.snapshot();
        assert_eq!(snapshot.phase(), FactoryPhase::SwapOffer);
        assert_eq!(snapshot.streak(), 1);
        assert!(snapshot.opponent().is_some());
        return Ok(());
    }
    Err(String::from("no winning seed found in range"))
}

#[test]
fn losing_battle_ends_the_run() -> Result<(), String> {
    for seed in 1..=64 {
        let session = transition(
            session()?,
            FactoryCommand::StartRun {
                seed,
                target_streak: 5,
            },
        )?;
        let session = transition(session, FactoryCommand::StartNextBattle)?;
        let (session, won) = battle_outcome(session)?;
        if won {
            continue;
        }
        let snapshot = session.snapshot();
        assert_eq!(snapshot.phase(), FactoryPhase::Finished);
        assert_eq!(snapshot.streak(), 0);
        return Ok(());
    }
    Err(String::from("no losing seed found in range"))
}

#[test]
fn swap_replaces_rental_member_and_returns_to_ready() -> Result<(), String> {
    for seed in 1..=256 {
        let session = transition(
            session()?,
            FactoryCommand::StartRun {
                seed,
                target_streak: 5,
            },
        )?;
        let session = transition(session, FactoryCommand::StartNextBattle)?;
        let (session, won) = battle_outcome(session)?;
        if !won {
            continue;
        }
        let snapshot = session.snapshot();
        let before_first = snapshot
            .rental()
            .first()
            .map(|member| member.name().to_owned())
            .ok_or_else(|| String::from("rental missing after win"))?;
        let opponent = snapshot
            .opponent()
            .ok_or_else(|| String::from("opponent missing after win"))?;
        let (opponent_slot, opponent_name) = opponent
            .iter()
            .enumerate()
            .find_map(|(index, member)| {
                (member.name() != before_first).then_some((index, member.name().to_owned()))
            })
            .ok_or_else(|| String::from("all opponent members match rental first"))?;
        let session = transition(
            session,
            FactoryCommand::ConfirmSwap {
                rental_slot: 0,
                opponent_slot,
            },
        )?;
        let snapshot = session.snapshot();
        assert_eq!(snapshot.phase(), FactoryPhase::Ready);
        let after_first = snapshot
            .rental()
            .first()
            .map(|member| member.name().to_owned());
        assert_eq!(after_first.as_deref(), Some(opponent_name.as_str()));
        assert_ne!(after_first.as_deref(), Some(before_first.as_str()));
        assert_eq!(snapshot.rental().len(), TEAM_SIZE);
        return Ok(());
    }
    Err(String::from("no winning seed found in range"))
}

#[test]
fn skip_swap_keeps_rental_and_returns_to_ready() -> Result<(), String> {
    for seed in 1..=128 {
        let session = transition(
            session()?,
            FactoryCommand::StartRun {
                seed,
                target_streak: 5,
            },
        )?;
        let session = transition(session, FactoryCommand::StartNextBattle)?;
        let (session, won) = battle_outcome(session)?;
        if !won {
            continue;
        }
        let before = session.snapshot().rental().to_vec();
        let session = transition(session, FactoryCommand::SkipSwap)?;
        let snapshot = session.snapshot();
        assert_eq!(snapshot.phase(), FactoryPhase::Ready);
        assert_eq!(snapshot.rental(), &before);
        return Ok(());
    }
    Err(String::from("no winning seed found in range"))
}

#[test]
fn swap_rejects_invalid_slots() -> Result<(), String> {
    for seed in 1..=128 {
        let session = transition(
            session()?,
            FactoryCommand::StartRun {
                seed,
                target_streak: 5,
            },
        )?;
        let session = transition(session, FactoryCommand::StartNextBattle)?;
        let (session, won) = battle_outcome(session)?;
        if !won {
            continue;
        }
        let (session, result) = session.transition(FactoryCommand::ConfirmSwap {
            rental_slot: TEAM_SIZE,
            opponent_slot: 0,
        });
        assert!(matches!(result, Err(FactoryError::InvalidRentalSlot(_))));
        let snapshot = session.snapshot();
        assert_eq!(snapshot.phase(), FactoryPhase::SwapOffer);
        return Ok(());
    }
    Err(String::from("no winning seed found in range"))
}

#[test]
fn clearing_target_streak_ends_run_as_cleared() -> Result<(), String> {
    for seed in 1..=256 {
        let mut session = transition(
            session()?,
            FactoryCommand::StartRun {
                seed,
                target_streak: 3,
            },
        )?;
        let mut cleared = false;
        for _ in 0..3 {
            session = transition(session, FactoryCommand::StartNextBattle)?;
            let (next, won) = battle_outcome(session)?;
            session = next;
            if !won {
                break;
            }
            if session.snapshot().phase() == FactoryPhase::Finished {
                cleared = true;
                break;
            }
            session = transition(session, FactoryCommand::SkipSwap)?;
        }
        if cleared {
            assert_eq!(session.snapshot().phase(), FactoryPhase::Finished);
            assert_eq!(session.snapshot().streak(), 3);
            return Ok(());
        }
    }
    Err(String::from("no seed cleared the streak in range"))
}

#[test]
fn submitting_during_playback_is_rejected() -> Result<(), String> {
    let session = start(session()?)?;
    let session = transition(session, FactoryCommand::StartNextBattle)?;
    let actions = session.legal_player_actions();
    let action = actions
        .iter()
        .copied()
        .find(|action| matches!(action, Action::UseMove(_)))
        .or_else(|| actions.first().copied())
        .ok_or_else(|| String::from("battle offered no action"))?;
    let session = transition(session, FactoryCommand::SubmitBattleAction(action))?;
    assert!(session.has_pending_playback());
    let (_, result) = session.transition(FactoryCommand::SubmitBattleAction(action));
    assert!(matches!(result, Err(FactoryError::PlayerActionUnavailable)));
    Ok(())
}

#[test]
fn start_next_battle_outside_ready_is_rejected() -> Result<(), String> {
    let session = start(session()?)?;
    let session = transition(session, FactoryCommand::StartNextBattle)?;
    let (_, result) = session.transition(FactoryCommand::StartNextBattle);
    assert!(matches!(
        result,
        Err(FactoryError::WrongPhase {
            expected: FactoryPhase::Ready,
            actual: FactoryPhase::Battle
        })
    ));
    Ok(())
}

#[test]
fn start_run_during_battle_is_rejected() -> Result<(), String> {
    let session = start(session()?)?;
    let session = transition(session, FactoryCommand::StartNextBattle)?;
    let (_, result) = session.transition(FactoryCommand::StartRun {
        seed: 99,
        target_streak: 3,
    });
    assert!(matches!(
        result,
        Err(FactoryError::WrongPhase {
            expected: FactoryPhase::Ready,
            actual: FactoryPhase::Battle
        })
    ));
    Ok(())
}

#[test]
fn leaving_unfinished_battle_is_rejected() -> Result<(), String> {
    let session = start(session()?)?;
    let session = transition(session, FactoryCommand::StartNextBattle)?;
    let (_, result) = session.transition(FactoryCommand::LeaveFinishedBattle);
    assert!(matches!(result, Err(FactoryError::BattleNotFinished)));
    Ok(())
}

#[test]
fn submitting_without_battle_is_rejected() -> Result<(), String> {
    let session = start(session()?)?;
    let action = session
        .legal_player_actions()
        .first()
        .copied()
        .unwrap_or(Action::Run);
    let (_, result) = session.transition(FactoryCommand::SubmitBattleAction(action));
    assert!(matches!(result, Err(FactoryError::BattleMissing)));
    Ok(())
}

#[test]
fn sprite_manifest_is_none_until_opponent_generated() -> Result<(), String> {
    let session = start(session()?)?;
    assert!(
        session
            .sprite_manifest()
            .map_err(|error| format!("manifest: {error:?}"))?
            .is_none()
    );
    let session = transition(session, FactoryCommand::StartNextBattle)?;
    assert!(
        session
            .sprite_manifest()
            .map_err(|error| format!("manifest: {error:?}"))?
            .is_some()
    );
    Ok(())
}

#[test]
fn first_opponent_differs_from_rental_team() -> Result<(), String> {
    let session = start(session()?)?;
    let rental = session
        .snapshot()
        .rental()
        .iter()
        .map(|member| member.form())
        .collect::<Vec<_>>();
    let session = transition(session, FactoryCommand::StartNextBattle)?;
    let opponent = session
        .snapshot()
        .opponent()
        .ok_or_else(|| String::from("opponent missing"))?
        .iter()
        .map(|member| member.form())
        .collect::<Vec<_>>();
    assert_ne!(rental, opponent);
    Ok(())
}

#[test]
fn opponent_sequence_is_deterministic_per_seed() -> Result<(), String> {
    let first = opponent_forms(session()?)?;
    let second = opponent_forms(session()?)?;
    assert_eq!(first, second);
    Ok(())
}

fn opponent_forms(session: FactorySession) -> Result<Vec<game_data::PokemonFormId>, String> {
    let session = start(session)?;
    let session = transition(session, FactoryCommand::StartNextBattle)?;
    Ok(session
        .snapshot()
        .opponent()
        .ok_or_else(|| String::from("opponent missing"))?
        .iter()
        .map(|member| member.form())
        .collect())
}

#[test]
fn healed_rental_after_loss_has_full_hp() -> Result<(), String> {
    for seed in 1..=64 {
        let session = transition(
            session()?,
            FactoryCommand::StartRun {
                seed,
                target_streak: 5,
            },
        )?;
        let session = transition(session, FactoryCommand::StartNextBattle)?;
        let (session, won) = battle_outcome(session)?;
        if won {
            continue;
        }
        for member in session.snapshot().rental() {
            assert_eq!(member.current_hp(), member.max_hp());
            assert!(!member.fainted());
        }
        return Ok(());
    }
    Err(String::from("no losing seed found in range"))
}
