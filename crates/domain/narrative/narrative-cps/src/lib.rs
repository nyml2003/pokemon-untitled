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
#[path = "../tests/unit/lib.rs"]
mod tests;
