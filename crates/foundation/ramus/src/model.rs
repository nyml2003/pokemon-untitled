use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrincipalId(String);

impl PrincipalId {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProviderId(String);

impl ProviderId {
    pub fn new(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodePath(String);

impl NodePath {
    pub fn parse(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        if value == "/" {
            return Ok(Self(value));
        }
        if !value.starts_with('/') || value.ends_with('/') || value.contains("//") {
            return Err(ModelError::InvalidPath(value));
        }
        if value
            .split('/')
            .skip(1)
            .any(|segment| validate_identifier(segment).is_err())
        {
            return Err(ModelError::InvalidPath(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn is_prefix_of(&self, other: &Self) -> bool {
        self.0 == "/"
            || self.0 == other.0
            || other
                .0
                .strip_prefix(&self.0)
                .is_some_and(|suffix| suffix.starts_with('/'))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MethodName(String);

impl MethodName {
    pub fn new(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterName(String);

impl ParameterName {
    pub fn new(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Effect {
    Read,
    Write,
    Invoke,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    Discover,
    Complete,
    Read,
    Write,
    Invoke,
}

impl From<Effect> for Capability {
    fn from(value: Effect) -> Self {
        match value {
            Effect::Read => Self::Read,
            Effect::Write => Self::Write,
            Effect::Invoke => Self::Invoke,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelError {
    InvalidIdentifier(String),
    InvalidPath(String),
}

impl fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(value) => write!(formatter, "invalid identifier: {value}"),
            Self::InvalidPath(value) => write!(formatter, "invalid node path: {value}"),
        }
    }
}

impl std::error::Error for ModelError {}

fn validate_identifier(value: &str) -> Result<(), ModelError> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(ModelError::InvalidIdentifier(value.to_owned()));
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(ModelError::InvalidIdentifier(value.to_owned()));
    }
    if chars.any(|character| {
        !character.is_ascii_alphanumeric() && !matches!(character, '_' | '-' | '.')
    }) {
        return Err(ModelError::InvalidIdentifier(value.to_owned()));
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/model.rs"]
mod tests;
