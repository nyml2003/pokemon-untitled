pub const TEAM_SIZE: usize = 6;
pub const MAX_MOVES: usize = 4;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Side {
    One,
    Two,
}

impl Side {
    pub const fn opponent(self) -> Self {
        match self {
            Self::One => Self::Two,
            Self::Two => Self::One,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TeamSlot(u8);

impl TeamSlot {
    pub fn new(index: usize) -> Result<Self, ValidationError> {
        if index < TEAM_SIZE {
            Ok(Self(index as u8))
        } else {
            Err(ValidationError::InvalidTeamSlot { index })
        }
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub(crate) const fn from_valid_index(index: usize) -> Self {
        Self(index as u8)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MoveSlot(u8);

impl MoveSlot {
    pub fn new(index: usize) -> Result<Self, ValidationError> {
        if index < MAX_MOVES {
            Ok(Self(index as u8))
        } else {
            Err(ValidationError::InvalidMoveSlot { index })
        }
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub(crate) const fn from_valid_index(index: usize) -> Self {
        Self(index as u8)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PokemonId(String);

impl PokemonId {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        Self::from_string(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn from_string(value: String) -> Result<Self, ValidationError> {
        if value.trim().is_empty() {
            Err(ValidationError::EmptyPokemonId)
        } else {
            Ok(Self(value))
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MoveId(String);

impl MoveId {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        Self::from_string(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn from_string(value: String) -> Result<Self, ValidationError> {
        if value.trim().is_empty() {
            Err(ValidationError::EmptyMoveId)
        } else {
            Ok(Self(value))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PokemonType {
    Normal,
    Fighting,
    Flying,
    Poison,
    Ground,
    Rock,
    Bug,
    Ghost,
    Steel,
    Fire,
    Water,
    Grass,
    Electric,
    Psychic,
    Ice,
    Dragon,
    Dark,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoveCategory {
    Physical,
    Special,
    Status,
}

impl MoveCategory {
    pub const fn for_gen3_type(move_type: PokemonType) -> Self {
        match move_type {
            PokemonType::Normal
            | PokemonType::Fighting
            | PokemonType::Flying
            | PokemonType::Poison
            | PokemonType::Ground
            | PokemonType::Rock
            | PokemonType::Bug
            | PokemonType::Ghost
            | PokemonType::Steel => Self::Physical,
            PokemonType::Fire
            | PokemonType::Water
            | PokemonType::Grass
            | PokemonType::Electric
            | PokemonType::Psychic
            | PokemonType::Ice
            | PokemonType::Dragon
            | PokemonType::Dark => Self::Special,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Accuracy {
    Percent(u8),
    AlwaysHit,
}

impl Accuracy {
    pub fn percent(value: u8) -> Result<Self, ValidationError> {
        if (1..=100).contains(&value) {
            Ok(Self::Percent(value))
        } else {
            Err(ValidationError::InvalidAccuracy { value })
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BattleStats {
    attack: u16,
    defense: u16,
    special_attack: u16,
    special_defense: u16,
    speed: u16,
}

impl BattleStats {
    pub fn new(
        attack: u16,
        defense: u16,
        special_attack: u16,
        special_defense: u16,
        speed: u16,
    ) -> Result<Self, ValidationError> {
        if attack == 0 {
            return Err(ValidationError::ZeroStat { stat: "attack" });
        }
        if defense == 0 {
            return Err(ValidationError::ZeroStat { stat: "defense" });
        }
        if special_attack == 0 {
            return Err(ValidationError::ZeroStat {
                stat: "special_attack",
            });
        }
        if special_defense == 0 {
            return Err(ValidationError::ZeroStat {
                stat: "special_defense",
            });
        }
        if speed == 0 {
            return Err(ValidationError::ZeroStat { stat: "speed" });
        }
        Ok(Self {
            attack,
            defense,
            special_attack,
            special_defense,
            speed,
        })
    }

    pub const fn attack(self) -> u16 {
        self.attack
    }

    pub const fn defense(self) -> u16 {
        self.defense
    }

    pub const fn special_attack(self) -> u16 {
        self.special_attack
    }

    pub const fn special_defense(self) -> u16 {
        self.special_defense
    }

    pub const fn speed(self) -> u16 {
        self.speed
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Move {
    id: MoveId,
    name: String,
    move_type: PokemonType,
    category: MoveCategory,
    power: u16,
    accuracy: Accuracy,
    max_pp: u8,
    current_pp: u8,
    priority: i8,
}

impl Move {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: MoveId,
        name: impl Into<String>,
        move_type: PokemonType,
        power: u16,
        accuracy: Accuracy,
        max_pp: u8,
        current_pp: u8,
        priority: i8,
    ) -> Result<Self, ValidationError> {
        Self::new_with_category(
            id,
            name,
            move_type,
            MoveCategory::for_gen3_type(move_type),
            power,
            accuracy,
            max_pp,
            current_pp,
            priority,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_category(
        id: MoveId,
        name: impl Into<String>,
        move_type: PokemonType,
        category: MoveCategory,
        power: u16,
        accuracy: Accuracy,
        max_pp: u8,
        current_pp: u8,
        priority: i8,
    ) -> Result<Self, ValidationError> {
        Self::from_parts(
            id,
            name.into(),
            move_type,
            category,
            power,
            accuracy,
            max_pp,
            current_pp,
            priority,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_parts(
        id: MoveId,
        name: String,
        move_type: PokemonType,
        category: MoveCategory,
        power: u16,
        accuracy: Accuracy,
        max_pp: u8,
        current_pp: u8,
        priority: i8,
    ) -> Result<Self, ValidationError> {
        if name.trim().is_empty() {
            return Err(ValidationError::EmptyMoveName);
        }
        if power == 0 {
            return Err(ValidationError::ZeroMovePower);
        }
        if let Accuracy::Percent(value) = accuracy
            && !(1..=100).contains(&value)
        {
            return Err(ValidationError::InvalidAccuracy { value });
        }
        if max_pp == 0 {
            return Err(ValidationError::ZeroMaxPp);
        }
        if current_pp > max_pp {
            return Err(ValidationError::CurrentPpExceedsMax {
                current: current_pp,
                max: max_pp,
            });
        }
        Ok(Self {
            id,
            name,
            move_type,
            category,
            power,
            accuracy,
            max_pp,
            current_pp,
            priority,
        })
    }

    pub fn id(&self) -> &MoveId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn move_type(&self) -> PokemonType {
        self.move_type
    }

    pub const fn category(&self) -> MoveCategory {
        self.category
    }

    pub const fn power(&self) -> u16 {
        self.power
    }

    pub const fn accuracy(&self) -> Accuracy {
        self.accuracy
    }

    pub const fn max_pp(&self) -> u8 {
        self.max_pp
    }

    pub const fn current_pp(&self) -> u8 {
        self.current_pp
    }

    pub const fn priority(&self) -> i8 {
        self.priority
    }

    pub(crate) fn spend_pp(&mut self) {
        self.current_pp -= 1;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pokemon {
    id: PokemonId,
    name: String,
    level: u8,
    primary_type: PokemonType,
    secondary_type: Option<PokemonType>,
    max_hp: u32,
    current_hp: u32,
    stats: BattleStats,
    moves: Vec<Move>,
}

impl Pokemon {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: PokemonId,
        name: impl Into<String>,
        level: u8,
        primary_type: PokemonType,
        secondary_type: Option<PokemonType>,
        max_hp: u32,
        current_hp: u32,
        stats: BattleStats,
        moves: Vec<Move>,
    ) -> Result<Self, ValidationError> {
        Self::from_parts(
            id,
            name.into(),
            level,
            primary_type,
            secondary_type,
            max_hp,
            current_hp,
            stats,
            moves,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_parts(
        id: PokemonId,
        name: String,
        level: u8,
        primary_type: PokemonType,
        secondary_type: Option<PokemonType>,
        max_hp: u32,
        current_hp: u32,
        stats: BattleStats,
        moves: Vec<Move>,
    ) -> Result<Self, ValidationError> {
        if name.trim().is_empty() {
            return Err(ValidationError::EmptyPokemonName);
        }
        if !(1..=100).contains(&level) {
            return Err(ValidationError::InvalidLevel { level });
        }
        if secondary_type == Some(primary_type) {
            return Err(ValidationError::DuplicatePokemonType { primary_type });
        }
        if max_hp == 0 {
            return Err(ValidationError::ZeroMaxHp);
        }
        if current_hp > max_hp {
            return Err(ValidationError::CurrentHpExceedsMax {
                current: current_hp,
                max: max_hp,
            });
        }
        if moves.is_empty() || moves.len() > MAX_MOVES {
            return Err(ValidationError::InvalidMoveCount { count: moves.len() });
        }
        for left in 0..moves.len() {
            for right in (left + 1)..moves.len() {
                if moves[left].id == moves[right].id {
                    return Err(ValidationError::DuplicateMoveId {
                        id: moves[left].id.clone(),
                    });
                }
            }
        }
        Ok(Self {
            id,
            name,
            level,
            primary_type,
            secondary_type,
            max_hp,
            current_hp,
            stats,
            moves,
        })
    }

    pub fn id(&self) -> &PokemonId {
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

    pub const fn max_hp(&self) -> u32 {
        self.max_hp
    }

    pub const fn current_hp(&self) -> u32 {
        self.current_hp
    }

    pub const fn stats(&self) -> BattleStats {
        self.stats
    }

    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    pub const fn is_fainted(&self) -> bool {
        self.current_hp == 0
    }

    pub(crate) fn move_mut(&mut self, slot: MoveSlot) -> Option<&mut Move> {
        self.moves.get_mut(slot.index())
    }

    pub(crate) fn apply_damage(&mut self, damage: u64) -> u32 {
        let actual = damage.min(u64::from(self.current_hp)) as u32;
        self.current_hp -= actual;
        actual
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Team {
    members: [Pokemon; TEAM_SIZE],
}

impl Team {
    pub fn new(members: Vec<Pokemon>) -> Result<Self, ValidationError> {
        if members.len() != TEAM_SIZE {
            return Err(ValidationError::InvalidTeamSize {
                count: members.len(),
            });
        }
        for left in 0..members.len() {
            for right in (left + 1)..members.len() {
                if members[left].id == members[right].id {
                    return Err(ValidationError::DuplicatePokemonId {
                        id: members[left].id.clone(),
                    });
                }
            }
        }
        let mut members = members.into_iter();
        let members = std::array::from_fn(|_| {
            members
                .next()
                .expect("team length was validated before array construction")
        });
        Ok(Self { members })
    }

    pub fn members(&self) -> &[Pokemon; TEAM_SIZE] {
        &self.members
    }

    pub fn member(&self, slot: TeamSlot) -> &Pokemon {
        &self.members[slot.index()]
    }

    pub(crate) fn member_mut(&mut self, slot: TeamSlot) -> &mut Pokemon {
        &mut self.members[slot.index()]
    }

    pub(crate) fn first_living_slot(&self) -> Option<TeamSlot> {
        self.members
            .iter()
            .position(|pokemon| !pokemon.is_fainted())
            .map(TeamSlot::from_valid_index)
    }

    pub(crate) fn has_living(&self) -> bool {
        self.members.iter().any(|pokemon| !pokemon.is_fainted())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationError {
    InvalidTeamSlot { index: usize },
    InvalidMoveSlot { index: usize },
    EmptyPokemonId,
    EmptyMoveId,
    EmptyPokemonName,
    EmptyMoveName,
    InvalidLevel { level: u8 },
    DuplicatePokemonType { primary_type: PokemonType },
    ZeroMaxHp,
    CurrentHpExceedsMax { current: u32, max: u32 },
    ZeroStat { stat: &'static str },
    ZeroMovePower,
    InvalidAccuracy { value: u8 },
    ZeroMaxPp,
    CurrentPpExceedsMax { current: u8, max: u8 },
    InvalidMoveCount { count: usize },
    InvalidTeamSize { count: usize },
    DuplicateMoveId { id: MoveId },
    DuplicatePokemonId { id: PokemonId },
}
