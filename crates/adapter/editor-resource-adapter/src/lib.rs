//! Filesystem adapter for logical editor documents.

#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

use editor_application::{EditorDocumentId, EditorDocumentStore, EditorKind};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ResourceKey {
    kind: EditorKind,
    document: EditorDocumentId,
}

impl ResourceKey {
    fn new(kind: EditorKind, document: EditorDocumentId) -> Self {
        Self { kind, document }
    }
}

#[derive(Clone, Debug)]
pub struct EditorResourceRegistry {
    root: PathBuf,
    entries: BTreeMap<ResourceKey, PathBuf>,
}

impl EditorResourceRegistry {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            entries: BTreeMap::new(),
        }
    }

    pub fn standard(root: impl Into<PathBuf>) -> Result<Self, EditorResourceError> {
        let mut registry = Self::new(root);
        registry.register(
            EditorKind::Map,
            "verdant-route",
            "maps/verdant-route/world.json",
        )?;
        registry.register(
            EditorKind::Trainer,
            "route-trainers",
            "assets/source/trainer/trainers-v1.json",
        )?;
        registry.register(
            EditorKind::Pokemon,
            "kanto-hoenn-pokedex",
            "assets/source/pokemon/pokemon-v1.json",
        )?;
        Ok(registry)
    }

    pub fn register(
        &mut self,
        kind: EditorKind,
        document: impl Into<String>,
        relative_path: impl Into<PathBuf>,
    ) -> Result<(), EditorResourceError> {
        let document = EditorDocumentId::new(document.into())
            .map_err(|error| EditorResourceError::InvalidDocument(error.to_string()))?;
        let relative_path = relative_path.into();
        if relative_path.is_absolute()
            || relative_path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(EditorResourceError::UnsafePath(relative_path));
        }
        self.entries
            .insert(ResourceKey::new(kind, document), relative_path);
        Ok(())
    }

    pub fn resolve(
        &self,
        kind: EditorKind,
        document: &EditorDocumentId,
    ) -> Result<PathBuf, EditorResourceError> {
        let key = ResourceKey::new(kind, document.clone());
        let relative_path =
            self.entries
                .get(&key)
                .ok_or_else(|| EditorResourceError::UnknownDocument {
                    kind,
                    document: document.clone(),
                })?;
        Ok(self.root.join(relative_path))
    }

    pub fn load_text(
        &self,
        kind: EditorKind,
        document: &EditorDocumentId,
    ) -> Result<String, EditorResourceError> {
        let path = self.resolve(kind, document)?;
        fs::read_to_string(&path).map_err(|source| EditorResourceError::Read { path, source })
    }

    pub fn save_text(
        &self,
        kind: EditorKind,
        document: &EditorDocumentId,
        text: &str,
    ) -> Result<(), EditorResourceError> {
        let path = self.resolve(kind, document)?;
        let parent = path
            .parent()
            .ok_or_else(|| EditorResourceError::UnsafePath(path.clone()))?;
        fs::create_dir_all(parent).map_err(|source| EditorResourceError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
        let temporary = temporary_path(&path)?;
        fs::write(&temporary, text).map_err(|source| EditorResourceError::Write {
            path: temporary.clone(),
            source,
        })?;
        fs::rename(&temporary, &path).map_err(|source| EditorResourceError::Write { path, source })
    }
}

impl EditorDocumentStore<String> for EditorResourceRegistry {
    type Error = EditorResourceError;

    fn load(&self, kind: EditorKind, document: &EditorDocumentId) -> Result<String, Self::Error> {
        self.load_text(kind, document)
    }

    fn save(
        &self,
        kind: EditorKind,
        document: &EditorDocumentId,
        value: &String,
    ) -> Result<(), Self::Error> {
        self.save_text(kind, document, value)
    }
}

fn temporary_path(path: &Path) -> Result<PathBuf, EditorResourceError> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| EditorResourceError::UnsafePath(path.to_path_buf()))?;
    Ok(path.with_file_name(format!(".{file_name}.editor-tmp")))
}

#[derive(Debug)]
pub enum EditorResourceError {
    InvalidDocument(String),
    UnsafePath(PathBuf),
    UnknownDocument {
        kind: EditorKind,
        document: EditorDocumentId,
    },
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for EditorResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDocument(error) => write!(formatter, "invalid editor document: {error}"),
            Self::UnsafePath(path) => {
                write!(formatter, "unsafe editor resource path: {}", path.display())
            }
            Self::UnknownDocument { kind, document } => {
                write!(
                    formatter,
                    "unregistered {kind:?} document: {}",
                    document.as_str()
                )
            }
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "cannot read editor resource {}: {source}",
                    path.display()
                )
            }
            Self::Write { path, source } => {
                write!(
                    formatter,
                    "cannot write editor resource {}: {source}",
                    path.display()
                )
            }
        }
    }
}

impl Error for EditorResourceError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_rejects_parent_traversal() {
        let mut registry = EditorResourceRegistry::new(".");
        let result = registry.register(EditorKind::Trainer, "route-trainers", "../secret.json");
        assert!(matches!(result, Err(EditorResourceError::UnsafePath(_))));
    }
}
