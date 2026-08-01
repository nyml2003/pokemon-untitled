use crate::enums::{
    Accuracy, MoveCategory, MoveEffect, PokemonType, WeatherAccuracyModifier, WeatherMoveModifier,
};
use crate::error::ValidationError;
use crate::id::MoveId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Move {
    id: MoveId,
    name: String,
    move_types: Vec<PokemonType>,
    category: MoveCategory,
    power: u16,
    accuracy: Accuracy,
    max_pp: u8,
    current_pp: u8,
    priority: i8,
    effects: Vec<MoveEffect>,
    weather_accuracy: Option<WeatherAccuracyModifier>,
    weather_move: Option<WeatherMoveModifier>,
}

impl Move {
    /// 使用第三世代属性默认类别创建不含附加效果的招式。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: MoveId,
        name: impl Into<String>,
        move_types: Vec<PokemonType>,
        power: u16,
        accuracy: Accuracy,
        max_pp: u8,
        current_pp: u8,
        priority: i8,
    ) -> Result<Self, ValidationError> {
        let primary = move_types
            .first()
            .copied()
            .ok_or(ValidationError::EmptyMoveType)?;
        Self::new_with_category(
            id,
            name,
            move_types,
            MoveCategory::for_gen3_type(primary),
            power,
            accuracy,
            max_pp,
            current_pp,
            priority,
        )
    }

    /// 使用指定类别创建不含附加效果的招式。
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_category(
        id: MoveId,
        name: impl Into<String>,
        move_types: Vec<PokemonType>,
        category: MoveCategory,
        power: u16,
        accuracy: Accuracy,
        max_pp: u8,
        current_pp: u8,
        priority: i8,
    ) -> Result<Self, ValidationError> {
        Self::new_with_category_and_effect(
            id,
            name,
            move_types,
            category,
            power,
            accuracy,
            max_pp,
            current_pp,
            priority,
            Vec::new(),
        )
    }

    /// 使用完整招式定义创建招式。
    ///
    /// 名称、威力、PP 和效果参数必须满足 [`ValidationError`] 所列规则。
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_category_and_effect(
        id: MoveId,
        name: impl Into<String>,
        move_types: Vec<PokemonType>,
        category: MoveCategory,
        power: u16,
        accuracy: Accuracy,
        max_pp: u8,
        current_pp: u8,
        priority: i8,
        effects: Vec<MoveEffect>,
    ) -> Result<Self, ValidationError> {
        Self::from_parts(
            id,
            name.into(),
            move_types,
            category,
            power,
            accuracy,
            max_pp,
            current_pp,
            priority,
            effects,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_parts(
        id: MoveId,
        name: String,
        move_types: Vec<PokemonType>,
        category: MoveCategory,
        power: u16,
        accuracy: Accuracy,
        max_pp: u8,
        current_pp: u8,
        priority: i8,
        effects: Vec<MoveEffect>,
    ) -> Result<Self, ValidationError> {
        if name.trim().is_empty() {
            return Err(ValidationError::EmptyMoveName);
        }
        if move_types.is_empty() {
            return Err(ValidationError::EmptyMoveType);
        }
        if power == 0
            && category != MoveCategory::Status
            && !effects.iter().any(|effect| effect.permits_zero_power())
        {
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
            move_types,
            category,
            power,
            accuracy,
            max_pp,
            current_pp,
            priority,
            effects,
            weather_accuracy: None,
            weather_move: None,
        })
    }

    pub fn id(&self) -> &MoveId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn move_types(&self) -> &[PokemonType] {
        &self.move_types
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

    /// 消耗一点 PP。
    pub fn spend_pp(&mut self) {
        self.current_pp = self.current_pp.saturating_sub(1);
    }

    pub const fn priority(&self) -> i8 {
        self.priority
    }

    pub fn effects(&self) -> &[MoveEffect] {
        &self.effects
    }

    /// 返回新的招式值，并为其设置命中率天气修正。
    pub const fn with_weather_accuracy(mut self, modifier: WeatherAccuracyModifier) -> Self {
        self.weather_accuracy = Some(modifier);
        self
    }

    pub const fn weather_accuracy(&self) -> Option<WeatherAccuracyModifier> {
        self.weather_accuracy
    }

    /// 返回新的招式值，并为其设置威力天气修正。
    pub const fn with_weather_move(mut self, modifier: WeatherMoveModifier) -> Self {
        self.weather_move = Some(modifier);
        self
    }

    pub const fn weather_move(&self) -> Option<WeatherMoveModifier> {
        self.weather_move
    }
}
