use serde::{Deserialize, Serialize};

use crate::{GameError, GameState, ThinSliceContent};

const SAVE_FORMAT_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SaveEnvelope {
    format_version: u16,
    content_version: String,
    checksum: u64,
    state: GameState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveError {
    Serialization(String),
    Malformed(String),
    UnsupportedFormat(u16),
    ContentMismatch { expected: String, actual: String },
    ChecksumMismatch,
    Game(GameError),
}

impl From<GameError> for SaveError {
    fn from(value: GameError) -> Self {
        Self::Game(value)
    }
}

impl SaveEnvelope {
    pub fn from_state(content: &ThinSliceContent, state: GameState) -> Result<Self, SaveError> {
        state.validate(content)?;
        let checksum = checksum(&state)?;
        Ok(Self {
            format_version: SAVE_FORMAT_VERSION,
            content_version: content.content_version().into(),
            checksum,
            state,
        })
    }
    pub fn to_json(&self) -> Result<Vec<u8>, SaveError> {
        serde_json::to_vec_pretty(self).map_err(|error| SaveError::Serialization(error.to_string()))
    }
    pub fn from_json(content: &ThinSliceContent, bytes: &[u8]) -> Result<Self, SaveError> {
        let save = serde_json::from_slice::<Self>(bytes)
            .map_err(|error| SaveError::Malformed(error.to_string()))?;
        if save.format_version != SAVE_FORMAT_VERSION {
            return Err(SaveError::UnsupportedFormat(save.format_version));
        }
        if save.content_version != content.content_version() {
            return Err(SaveError::ContentMismatch {
                expected: content.content_version().into(),
                actual: save.content_version,
            });
        }
        if save.checksum != checksum(&save.state)? {
            return Err(SaveError::ChecksumMismatch);
        }
        save.state.validate(content)?;
        Ok(save)
    }
    pub fn state(&self) -> &GameState {
        &self.state
    }
}

fn checksum(state: &GameState) -> Result<u64, SaveError> {
    let bytes =
        serde_json::to_vec(state).map_err(|error| SaveError::Serialization(error.to_string()))?;
    Ok(bytes
        .into_iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |value, byte| {
            (value ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        }))
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    #[test]
    fn load_rejects_a_state_with_an_unknown_content_reference() -> Result<(), String> {
        let content =
            ThinSliceContent::standard().map_err(|error| format!("content: {error:?}"))?;
        let state = GameState::new(&content).map_err(|error| format!("state: {error:?}"))?;
        let save = SaveEnvelope::from_state(&content, state)
            .map_err(|error| format!("save: {error:?}"))?;
        let mut value = serde_json::to_value(save).map_err(|error| format!("json: {error}"))?;
        let state_value = value
            .get_mut("state")
            .ok_or_else(|| String::from("missing serialized state"))?;
        let map = state_value
            .get_mut("map")
            .ok_or_else(|| String::from("missing serialized map"))?;
        *map = Value::String(String::from("missing-map"));
        let invalid_state: GameState = serde_json::from_value(state_value.clone())
            .map_err(|error| format!("decode invalid state: {error}"))?;
        let checksum = checksum(&invalid_state).map_err(|error| format!("checksum: {error:?}"))?;
        value["checksum"] = Value::from(checksum);
        let bytes = serde_json::to_vec(&value).map_err(|error| format!("encode: {error}"))?;
        assert!(matches!(
            SaveEnvelope::from_json(&content, &bytes),
            Err(SaveError::Game(GameError::MapMissing(_)))
        ));
        Ok(())
    }
}
