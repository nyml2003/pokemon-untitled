use crate::battle_unit::BattleUnit;
use crate::error::ValidationError;
use crate::id::{TEAM_SIZE, TeamSlot};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Team {
    /// 六名成员。
    pub members: [BattleUnit; TEAM_SIZE],
}

impl Team {
    /// 从恰好六只且标识互不重复的战斗单位创建队伍。
    pub fn new(members: Vec<BattleUnit>) -> Result<Self, ValidationError> {
        if members.len() != TEAM_SIZE {
            return Err(ValidationError::InvalidTeamSize {
                count: members.len(),
            });
        }
        for left in 0..members.len() {
            for right in (left + 1)..members.len() {
                if members[left].id() == members[right].id() {
                    return Err(ValidationError::DuplicatePokemonId {
                        id: members[left].id().clone(),
                    });
                }
            }
        }
        let members = members.try_into().map_err(|members: Vec<BattleUnit>| {
            ValidationError::InvalidTeamSize {
                count: members.len(),
            }
        })?;
        Ok(Self { members })
    }

    /// 返回按队伍槽位顺序排列的全部成员。
    pub fn members(&self) -> &[BattleUnit; TEAM_SIZE] {
        &self.members
    }

    /// 返回指定有效槽位中的成员。
    pub fn member(&self, slot: TeamSlot) -> &BattleUnit {
        &self.members[slot.index()]
    }

    pub(crate) fn first_living_slot(&self) -> Option<TeamSlot> {
        self.members
            .iter()
            .position(|unit| !unit.is_fainted())
            .map(TeamSlot::from_valid_index)
    }
}
