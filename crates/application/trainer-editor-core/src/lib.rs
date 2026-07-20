//! Pure training-catalog editor state and typed edit commands.

#![forbid(unsafe_code)]

use std::{error::Error, fmt};

use editor_application::{EditorCore, EditorDiagnostic};
use game_foundation::{TrainerCatalog, TrainerEditCommand, TrainerError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TrainerEditorCommand {
    Inspect,
    Validate,
    Edit(TrainerEditCommand),
    Save,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TrainerEditorState {
    catalog: TrainerCatalog,
    dirty: bool,
}

impl TrainerEditorState {
    pub fn catalog(&self) -> &TrainerCatalog {
        &self.catalog
    }

    pub const fn dirty(&self) -> bool {
        self.dirty
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum TrainerEditorResult {
    State(TrainerEditorState),
    Diagnostics(Vec<EditorDiagnostic>),
    SaveRequested,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrainerEditorError {
    Trainer(TrainerError),
}

impl fmt::Display for TrainerEditorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Trainer(error) => error.fmt(formatter),
        }
    }
}

impl Error for TrainerEditorError {}

impl From<TrainerError> for TrainerEditorError {
    fn from(value: TrainerError) -> Self {
        Self::Trainer(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrainerEditorModel {
    catalog: TrainerCatalog,
    dirty: bool,
}

impl TrainerEditorModel {
    pub fn new(catalog: TrainerCatalog) -> Result<Self, TrainerEditorError> {
        catalog.validate()?;
        Ok(Self {
            catalog,
            dirty: false,
        })
    }

    pub fn execute(
        &self,
        command: TrainerEditorCommand,
    ) -> Result<(Self, TrainerEditorResult), TrainerEditorError> {
        match command {
            TrainerEditorCommand::Inspect => {
                Ok((self.clone(), TrainerEditorResult::State(self.inspect())))
            }
            TrainerEditorCommand::Validate => Ok((
                self.clone(),
                TrainerEditorResult::Diagnostics(self.validate()),
            )),
            TrainerEditorCommand::Edit(command) => {
                let model = self.clone().transition(command)?;
                Ok((model.clone(), TrainerEditorResult::State(model.inspect())))
            }
            TrainerEditorCommand::Save => Ok((self.clone(), TrainerEditorResult::SaveRequested)),
        }
    }

    pub fn saved(mut self) -> Self {
        self.dirty = false;
        self
    }

    pub fn catalog(&self) -> &TrainerCatalog {
        &self.catalog
    }
}

impl EditorCore for TrainerEditorModel {
    type Command = TrainerEditCommand;
    type Snapshot = TrainerEditorState;
    type Error = TrainerEditorError;

    fn inspect(&self) -> Self::Snapshot {
        TrainerEditorState {
            catalog: self.catalog.clone(),
            dirty: self.dirty,
        }
    }

    fn validate(&self) -> Vec<EditorDiagnostic> {
        match self.catalog.validate() {
            Ok(()) => Vec::new(),
            Err(error) => vec![EditorDiagnostic::new("trainer.invalid", error.to_string())],
        }
    }

    fn transition(mut self, command: Self::Command) -> Result<Self, Self::Error> {
        self.catalog = self.catalog.transition(command)?;
        self.dirty = true;
        Ok(self)
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_foundation::{TrainerId, TrainerPokemon};

    #[test]
    fn typed_commands_are_transactional_and_mark_the_model_dirty() -> Result<(), TrainerEditorError>
    {
        let model = TrainerEditorModel::new(TrainerCatalog::standard()?)?;
        let trainer = TrainerId::new("route-rival").map_err(TrainerError::from)?;
        let (model, _) =
            model.execute(TrainerEditorCommand::Edit(TrainerEditCommand::AddPokemon {
                trainer,
                pokemon: TrainerPokemon::new("Poochyena", 6)?,
            }))?;
        assert!(model.is_dirty());
        assert_eq!(model.catalog().trainers()[0].pokemon().len(), 2);
        Ok(())
    }
}
