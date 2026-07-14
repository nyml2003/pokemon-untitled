use battle_application::{
    Accuracy, BattleApplication, BattleStats, Move, MoveId, Pokemon, PokemonId, PokemonType,
    TEAM_SIZE, Team,
};

use super::*;

#[derive(Default)]
struct FirstMovePolicy;

impl OpponentPolicy for FirstMovePolicy {
    fn choose_action(
        &self,
        _observation: &BattleObservation,
        legal_actions: &[Action],
    ) -> Option<Action> {
        legal_actions
            .iter()
            .copied()
            .find(|action| matches!(action, Action::UseMove(_)))
            .or_else(|| legal_actions.first().copied())
    }
}

struct NoActionPolicy;

impl OpponentPolicy for NoActionPolicy {
    fn choose_action(
        &self,
        _observation: &BattleObservation,
        _legal_actions: &[Action],
    ) -> Option<Action> {
        None
    }
}

struct MoveOnlyPolicy;

impl OpponentPolicy for MoveOnlyPolicy {
    fn choose_action(
        &self,
        _observation: &BattleObservation,
        legal_actions: &[Action],
    ) -> Option<Action> {
        legal_actions
            .iter()
            .copied()
            .find(|action| matches!(action, Action::UseMove(_)))
    }
}

fn battle_move(name: &str, power: u16) -> Move {
    Move::new(
        MoveId::new(name).unwrap(),
        name,
        PokemonType::Normal,
        power,
        Accuracy::AlwaysHit,
        20,
        20,
        0,
    )
    .unwrap()
}

fn pokemon(name: &str, hp: u32, attack: u16, defense: u16, speed: u16, power: u16) -> Pokemon {
    Pokemon::new(
        PokemonId::new(name).unwrap(),
        name,
        50,
        PokemonType::Normal,
        None,
        hp,
        hp,
        BattleStats::new(attack, defense, attack, defense, speed).unwrap(),
        vec![battle_move(&format!("{name}-move"), power)],
    )
    .unwrap()
}

fn team(prefix: &str, lead: Pokemon, bench_hp: u32) -> Team {
    let mut members = vec![lead];
    for index in 1..TEAM_SIZE {
        members.push(pokemon(
            &format!("{prefix}-{index}"),
            bench_hp,
            50,
            50,
            50,
            40,
        ));
    }
    Team::new(members).unwrap()
}

fn session(player: Team, opponent: Team, seed: u64) -> BattleSession<FirstMovePolicy> {
    let application = BattleApplication::new(player, opponent, seed).unwrap();
    BattleSession::new(BattleCoordinator::new(application, FirstMovePolicy))
}

fn submit(
    session: BattleSession<FirstMovePolicy>,
    action: Action,
) -> BattleSession<FirstMovePolicy> {
    let (session, result) = session.submit(action);
    result.unwrap();
    session
}

fn advance(session: BattleSession<FirstMovePolicy>) -> (BattleSession<FirstMovePolicy>, bool) {
    session.advance()
}

fn drain(mut session: BattleSession<FirstMovePolicy>) -> BattleSession<FirstMovePolicy> {
    while session.has_pending_playback() {
        let (next, advanced) = advance(session);
        session = next;
        assert!(advanced);
    }
    session
}

#[test]
fn input_and_policy_failures_leave_explicit_session_states() {
    let player = team("player", pokemon("player-lead", 100, 50, 50, 50, 40), 100);
    let opponent = team(
        "opponent",
        pokemon("opponent-lead", 100, 50, 50, 50, 40),
        100,
    );
    let initial = session(player.clone(), opponent.clone(), 1);
    let (initial, advanced) = initial.advance();
    assert!(!advanced);
    let before = initial.snapshot();
    let (initial, rejected) = initial.submit(Action::Switch(TeamSlot::new(0).unwrap()));
    assert!(matches!(
        rejected,
        Err(SessionError::ActionNotOffered { .. })
    ));
    assert_eq!(initial.snapshot(), before);

    let application = BattleApplication::new(player.clone(), opponent.clone(), 1).unwrap();
    let coordinator = BattleCoordinator::new(application, NoActionPolicy);
    let locked = BattleSession::new(coordinator);
    let action = locked.legal_actions()[0];
    let (_locked, result) = locked.submit(action);
    assert_eq!(result, Err(SessionError::OpponentActionUnavailable));

    let application = BattleApplication::new(player, opponent, 1).unwrap();
    let coordinator = BattleCoordinator::new(application, FirstMovePolicy);
    let invalid = Action::Switch(TeamSlot::new(0).unwrap());
    let (_coordinator, result) = coordinator.resolve_player_action(invalid);
    assert!(matches!(
        result,
        Err(coordinator::CoordinatorError::Battle(_))
    ));
}

#[test]
fn opponent_only_replacement_requires_a_policy_action() {
    let player = team("player", pokemon("killer", 100, 500, 50, 100, 500), 100);
    let opponent = team("opponent", pokemon("victim", 10, 10, 10, 1, 1), 100);
    let application = BattleApplication::new(player, opponent, 3).unwrap();
    let coordinator = BattleCoordinator::new(application, MoveOnlyPolicy);
    let action = coordinator
        .player_legal_actions()
        .into_iter()
        .find(|action| matches!(action, Action::UseMove(_)))
        .unwrap();
    let (_coordinator, result) = coordinator.resolve_player_action(action);
    assert_eq!(
        result,
        Err(coordinator::CoordinatorError::OpponentActionUnavailable)
    );
}

#[test]
fn active_switch_uses_entry_hp_until_the_damage_event() {
    let player = team("player", pokemon("player-lead", 100, 50, 50, 10, 40), 100);
    let opponent = team(
        "opponent",
        pokemon("opponent-lead", 100, 80, 50, 90, 40),
        100,
    );
    let mut session = session(player, opponent, 4);
    let switch = Action::Switch(TeamSlot::new(1).unwrap());

    session = submit(session, switch);
    assert_eq!(
        session.snapshot().scene().own().id().as_str(),
        "player-lead"
    );

    let mut saw_switch = false;
    let mut saw_damage = false;
    while session.has_pending_playback() {
        (session, _) = advance(session);
        let snapshot = session.snapshot();
        match snapshot.cue() {
            Some(BattleCue::Switched {
                participant: Participant::Own,
            }) => {
                saw_switch = true;
                assert_eq!(snapshot.scene().own().id().as_str(), "player-1");
                assert_eq!(snapshot.scene().own().current_hp(), 100);
            }
            Some(BattleCue::MoveUsed {
                participant: Participant::Opponent,
                ..
            }) if saw_switch => assert_eq!(snapshot.scene().own().current_hp(), 100),
            Some(BattleCue::DamageApplied {
                participant: Participant::Own,
                ..
            }) => {
                saw_damage = true;
                assert!(snapshot.scene().own().current_hp() < 100);
            }
            _ => {}
        }
    }
    assert!(saw_switch && saw_damage);
}

#[test]
fn knocked_out_opponent_remains_visible_until_its_switch_event() {
    let player = team("player", pokemon("killer", 100, 500, 50, 100, 500), 100);
    let opponent = team("opponent", pokemon("victim", 10, 10, 10, 1, 1), 100);
    let mut session = session(player, opponent, 3);
    let action = session
        .legal_actions()
        .iter()
        .copied()
        .find(|action| matches!(action, Action::UseMove(_)))
        .unwrap();

    session = submit(session, action);
    let mut saw_faint = false;
    let mut saw_switch = false;
    while session.has_pending_playback() {
        (session, _) = advance(session);
        let snapshot = session.snapshot();
        match snapshot.cue() {
            Some(BattleCue::Fainted {
                participant: Participant::Opponent,
            }) => {
                saw_faint = true;
                assert_eq!(snapshot.scene().opponent().id().as_str(), "victim");
                assert_eq!(snapshot.scene().opponent().current_hp(), 0);
            }
            Some(BattleCue::Switched {
                participant: Participant::Opponent,
            }) => {
                assert!(saw_faint);
                saw_switch = true;
                assert_eq!(snapshot.scene().opponent().id().as_str(), "opponent-1");
                assert_eq!(snapshot.scene().opponent().current_hp(), 100);
            }
            _ => {
                if saw_faint && !saw_switch {
                    assert_eq!(snapshot.scene().opponent().id().as_str(), "victim");
                }
            }
        }
    }
    assert!(saw_faint && saw_switch);
}

#[test]
fn forced_replacement_prompt_opens_only_after_faint_playback() {
    let player = team("player", pokemon("victim", 10, 10, 10, 1, 1), 100);
    let opponent = team("opponent", pokemon("killer", 100, 500, 50, 100, 500), 100);
    let mut session = session(player, opponent, 8);
    let action = session
        .legal_actions()
        .iter()
        .copied()
        .find(|action| matches!(action, Action::UseMove(_)))
        .unwrap();

    session = submit(session, action);
    let mut saw_faint = false;
    while session.has_pending_playback() {
        assert!(matches!(
            session.snapshot().interaction(),
            BattleInteraction::PlaybackLocked
        ));
        (session, _) = advance(session);
        saw_faint |= matches!(
            session.snapshot().cue(),
            Some(BattleCue::Fainted {
                participant: Participant::Own
            })
        );
    }
    assert!(saw_faint);
    assert!(matches!(
        session.snapshot().interaction(),
        BattleInteraction::ChooseReplacement(_)
    ));
    let snapshot = session.snapshot();
    let BattleInteraction::ChooseReplacement(prompt) = snapshot.interaction() else {
        panic!("the player should be choosing a replacement");
    };
    assert_eq!(prompt.observation().viewer(), battle_application::Side::One);
    assert!(!prompt.legal_actions().is_empty());
}

#[test]
fn playback_rejects_new_input() {
    let player = team("player", pokemon("player-lead", 100, 50, 50, 100, 40), 100);
    let opponent = team(
        "opponent",
        pokemon("opponent-lead", 100, 50, 50, 10, 40),
        100,
    );
    let mut session = session(player, opponent, 2);
    let action = session.legal_actions()[0];
    session = submit(session, action);

    let (next, result) = session.submit(action);
    session = next;
    assert_eq!(result, Err(SessionError::InputLocked));
    session = drain(session);
    assert!(matches!(
        session.phase(),
        BattleSessionPhase::AwaitingAction(_)
    ));
}
