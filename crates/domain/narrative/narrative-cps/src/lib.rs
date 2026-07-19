//! Pure continuation-based executable form for narrative scripts.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceIdError {
    MissingSeparator,
    WrongPrefix {
        expected: &'static str,
        actual: String,
    },
    InvalidName,
}

impl fmt::Display for ResourceIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSeparator => formatter.write_str("resource ID must contain ':'"),
            Self::WrongPrefix { expected, actual } => {
                write!(formatter, "expected {expected}: resource, got {actual}:")
            }
            Self::InvalidName => formatter.write_str("resource ID name is invalid"),
        }
    }
}

impl std::error::Error for ResourceIdError {}

macro_rules! resource_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ResourceIdError> {
                let value = value.into();
                validate_resource(&$prefix, &value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

resource_id!(ScriptId, "script");
resource_id!(TextId, "text");
resource_id!(EventId, "event");
resource_id!(ActorId, "actor");

fn validate_resource(expected: &'static str, value: &str) -> Result<(), ResourceIdError> {
    let Some((actual, name)) = value.split_once(':') else {
        return Err(ResourceIdError::MissingSeparator);
    };
    if actual != expected {
        return Err(ResourceIdError::WrongPrefix {
            expected,
            actual: actual.into(),
        });
    }
    if !is_resource_name(name) {
        return Err(ResourceIdError::InvalidName);
    }
    Ok(())
}

fn is_resource_name(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContinuationId(u32);

impl ContinuationId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptDirection {
    Up,
    Down,
    Left,
    Right,
}

impl ScriptDirection {
    pub fn from_keyword(value: &str) -> Option<Self> {
        match value {
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CpsNode {
    Move {
        direction: ScriptDirection,
        next: ContinuationId,
    },
    Face {
        direction: ScriptDirection,
        next: ContinuationId,
    },
    Say {
        text: TextId,
        next: ContinuationId,
    },
    Wait {
        event: EventId,
        resume: ContinuationId,
    },
    End,
}

impl CpsNode {
    fn target(&self) -> Option<ContinuationId> {
        match self {
            Self::Move { next, .. } | Self::Face { next, .. } | Self::Say { next, .. } => {
                Some(*next)
            }
            Self::Wait { resume, .. } => Some(*resume),
            Self::End => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptProgramError {
    MissingEntry(ContinuationId),
    MissingTarget {
        source: ContinuationId,
        target: ContinuationId,
    },
}

impl fmt::Display for ScriptProgramError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEntry(entry) => {
                write!(formatter, "missing entry continuation {}", entry.value())
            }
            Self::MissingTarget { source, target } => write!(
                formatter,
                "continuation {} targets missing continuation {}",
                source.value(),
                target.value()
            ),
        }
    }
}

impl std::error::Error for ScriptProgramError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptProgram {
    id: ScriptId,
    actor: Option<ActorId>,
    entry: ContinuationId,
    continuations: BTreeMap<ContinuationId, CpsNode>,
}

impl ScriptProgram {
    pub fn new(
        id: ScriptId,
        entry: ContinuationId,
        continuations: BTreeMap<ContinuationId, CpsNode>,
    ) -> Result<Self, ScriptProgramError> {
        Self::with_actor(id, None, entry, continuations)
    }

    pub fn with_actor(
        id: ScriptId,
        actor: Option<ActorId>,
        entry: ContinuationId,
        continuations: BTreeMap<ContinuationId, CpsNode>,
    ) -> Result<Self, ScriptProgramError> {
        if !continuations.contains_key(&entry) {
            return Err(ScriptProgramError::MissingEntry(entry));
        }
        for (source, node) in &continuations {
            if let Some(target) = node.target()
                && !continuations.contains_key(&target)
            {
                return Err(ScriptProgramError::MissingTarget {
                    source: *source,
                    target,
                });
            }
        }
        Ok(Self {
            id,
            actor,
            entry,
            continuations,
        })
    }

    pub fn id(&self) -> &ScriptId {
        &self.id
    }

    pub fn actor(&self) -> Option<&ActorId> {
        self.actor.as_ref()
    }

    pub const fn entry(&self) -> ContinuationId {
        self.entry
    }

    pub fn continuation(&self, id: ContinuationId) -> Option<&CpsNode> {
        self.continuations.get(&id)
    }

    pub fn continuations(&self) -> &BTreeMap<ContinuationId, CpsNode> {
        &self.continuations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_ids_are_typed_and_validate_every_error() {
        assert_eq!(
            ScriptId::new("script:opening").unwrap().as_str(),
            "script:opening"
        );
        assert_eq!(TextId::new("text:hello").unwrap().as_str(), "text:hello");
        assert_eq!(EventId::new("event:clear").unwrap().as_str(), "event:clear");
        assert_eq!(ActorId::new("actor:guide").unwrap().as_str(), "actor:guide");
        for value in [
            "script",
            "wrong:name",
            "script:",
            "script:9bad",
            "script:bad/name",
        ] {
            assert!(ScriptId::new(value).is_err());
        }
        assert_eq!(
            ResourceIdError::MissingSeparator.to_string(),
            "resource ID must contain ':'"
        );
        assert_eq!(
            ResourceIdError::WrongPrefix {
                expected: "script",
                actual: "event".into(),
            }
            .to_string(),
            "expected script: resource, got event:"
        );
        assert_eq!(
            ResourceIdError::InvalidName.to_string(),
            "resource ID name is invalid"
        );
    }

    #[test]
    fn programs_validate_entries_targets_and_accessors() {
        let id = ScriptId::new("script:opening").unwrap();
        let entry = ContinuationId::new(0);
        let next = ContinuationId::new(1);
        let mut valid = BTreeMap::new();
        valid.insert(
            entry,
            CpsNode::Say {
                text: TextId::new("text:hello").unwrap(),
                next,
            },
        );
        valid.insert(
            next,
            CpsNode::Wait {
                event: EventId::new("event:clear").unwrap(),
                resume: ContinuationId::new(2),
            },
        );
        valid.insert(ContinuationId::new(2), CpsNode::End);
        let program = ScriptProgram::new(id, entry, valid).unwrap();
        assert_eq!(program.id().as_str(), "script:opening");
        assert_eq!(program.actor(), None);
        assert_eq!(program.entry().value(), 0);
        assert!(matches!(
            program.continuation(next),
            Some(CpsNode::Wait { .. })
        ));
        assert_eq!(program.continuations().len(), 3);

        let missing_entry = ScriptProgram::new(
            ScriptId::new("script:missing").unwrap(),
            ContinuationId::new(9),
            BTreeMap::new(),
        )
        .unwrap_err();
        assert_eq!(missing_entry.to_string(), "missing entry continuation 9");

        let mut missing_target = BTreeMap::new();
        missing_target.insert(
            entry,
            CpsNode::Say {
                text: TextId::new("text:hello").unwrap(),
                next,
            },
        );
        let error = ScriptProgram::new(
            ScriptId::new("script:target").unwrap(),
            entry,
            missing_target,
        )
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "continuation 0 targets missing continuation 1"
        );
    }

    #[test]
    fn actor_bound_programs_expose_movement_and_facing_nodes() {
        let entry = ContinuationId::new(0);
        let face = ContinuationId::new(1);
        let mut continuations = BTreeMap::new();
        continuations.insert(
            entry,
            CpsNode::Move {
                direction: ScriptDirection::Right,
                next: face,
            },
        );
        continuations.insert(
            face,
            CpsNode::Face {
                direction: ScriptDirection::Up,
                next: ContinuationId::new(2),
            },
        );
        continuations.insert(ContinuationId::new(2), CpsNode::End);
        let program = ScriptProgram::with_actor(
            ScriptId::new("script:guide").unwrap(),
            Some(ActorId::new("actor:guide").unwrap()),
            entry,
            continuations,
        )
        .unwrap();

        assert_eq!(program.actor().unwrap().as_str(), "actor:guide");
        assert_eq!(
            ScriptDirection::from_keyword("left"),
            Some(ScriptDirection::Left)
        );
        assert_eq!(ScriptDirection::from_keyword("unknown"), None);
        assert!(matches!(
            program.continuation(entry),
            Some(CpsNode::Move {
                direction: ScriptDirection::Right,
                ..
            })
        ));
        assert!(matches!(
            program.continuation(face),
            Some(CpsNode::Face {
                direction: ScriptDirection::Up,
                ..
            })
        ));
    }
}
