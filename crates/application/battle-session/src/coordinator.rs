use battle_application::{
    Action, BattleApplication, BattleError, BattleObservation, BattlePerspective, BattleTransition,
};

pub trait OpponentPolicy {
    fn choose_action(
        &self,
        observation: &BattleObservation,
        legal_actions: &[Action],
    ) -> Option<Action>;
}

pub struct BattleCoordinator<P> {
    application: BattleApplication,
    player: BattlePerspective,
    opponent: BattlePerspective,
    opponent_policy: P,
}

impl<P: OpponentPolicy> BattleCoordinator<P> {
    pub fn new(application: BattleApplication, opponent_policy: P) -> Self {
        let (player, opponent) = application.perspectives();
        Self {
            application,
            player,
            opponent,
            opponent_policy,
        }
    }

    pub fn player_observation(&self) -> BattleObservation {
        self.application.observe(&self.player)
    }

    pub fn player_legal_actions(&self) -> Vec<Action> {
        self.application.legal_actions(&self.player)
    }

    pub fn resolve_player_action(
        mut self,
        action: Action,
    ) -> (Self, Result<BattleTransition, CoordinatorError>) {
        let checkpoint = self.application.checkpoint(&self.player);
        let outcome = match self.application.submit(&self.player, action) {
            Ok(outcome) => outcome,
            Err(error) => return (self, Err(error.into())),
        };
        if outcome.is_waiting_for_opponent()
            && let Err(error) = self.submit_opponent()
        {
            return (self, Err(error));
        }
        if let Err(error) = self.resolve_opponent_only_replacements() {
            return (self, Err(error));
        }
        let transition = self
            .application
            .transition_since(checkpoint)
            .expect("a coordinator checkpoint belongs to its application and event log");
        (self, Ok(transition))
    }

    fn resolve_opponent_only_replacements(&mut self) -> Result<(), CoordinatorError> {
        loop {
            let player = self.application.observe(&self.player);
            let opponent = self.application.observe(&self.opponent);
            let player_required = player.phase().requires_replacement(player.viewer());
            let opponent_required = opponent.phase().requires_replacement(opponent.viewer());
            if !opponent_required || player_required {
                return Ok(());
            }
            self.submit_opponent()?;
        }
    }

    fn submit_opponent(&mut self) -> Result<(), CoordinatorError> {
        let observation = self.application.observe(&self.opponent);
        let legal_actions = self.application.legal_actions(&self.opponent);
        let action = self
            .opponent_policy
            .choose_action(&observation, &legal_actions)
            .ok_or(CoordinatorError::OpponentActionUnavailable)?;
        self.application.submit(&self.opponent, action)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoordinatorError {
    Battle(BattleError),
    OpponentActionUnavailable,
}

impl From<BattleError> for CoordinatorError {
    fn from(error: BattleError) -> Self {
        Self::Battle(error)
    }
}
