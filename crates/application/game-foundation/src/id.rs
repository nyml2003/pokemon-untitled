use serde::{Deserialize, Serialize};

macro_rules! stable_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, GameIdError> {
                let value = value.into();
                if value.is_empty()
                    || !value.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
                    })
                {
                    return Err(GameIdError::Invalid(value));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

stable_id!(MapId);
stable_id!(NpcId);
stable_id!(WarpId);
stable_id!(ItemId);
stable_id!(CreatureId);
stable_id!(CreatureTemplateId);
stable_id!(MoveId);
stable_id!(BattleId);
stable_id!(ShopId);
stable_id!(EventFlagId);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameIdError {
    Invalid(String),
}
