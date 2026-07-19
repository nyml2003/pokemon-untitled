use std::{error::Error, fmt};

use crate::{UiId, UiKey};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiBuildError {
    EmptyContentId,
    EmptyKey,
    DuplicateId(UiId),
    DuplicateKey(UiKey),
    IdExhausted,
    ZeroRatioBase(UiId),
    ZeroLogicalCanvas(UiId),
    ZeroTextSizeBase(UiId),
}
impl fmt::Display for UiBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyContentId => f.write_str("UI content id must not be empty"),
            Self::EmptyKey => f.write_str("UI key must not be empty"),
            Self::DuplicateId(id) => write!(f, "UI node id {:?} is duplicated", id),
            Self::DuplicateKey(key) => write!(f, "UI key {:?} is duplicated", key),
            Self::IdExhausted => f.write_str("UI tree exhausted all structural IDs"),
            Self::ZeroRatioBase(id) => write!(f, "UI node {:?} has a zero ratio base", id),
            Self::ZeroLogicalCanvas(id) => {
                write!(f, "UI node {:?} has a zero logical canvas", id)
            }
            Self::ZeroTextSizeBase(id) => {
                write!(f, "UI node {:?} has a zero text-size base", id)
            }
        }
    }
}
impl Error for UiBuildError {}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiLayoutError {
    InsufficientSpace { id: UiId },
}
impl fmt::Display for UiLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientSpace { id } => {
                write!(f, "UI node {:?} does not fit its container", id)
            }
        }
    }
}
impl Error for UiLayoutError {}
