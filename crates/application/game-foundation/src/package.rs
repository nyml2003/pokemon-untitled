use serde::{Deserialize, Serialize};

use crate::{CONTENT_VERSION, ContentError, ContentPackageId, GameIdError, ThinSliceContent};

pub const STARTER_REGION_PACKAGE_ID: &str = "starter-region";
pub const STARTER_REGION_PACKAGE_REVISION: u16 = 1;
pub const EMBEDDED_GEN3_DATA_REFERENCE: &str = "embedded-pokeapi-gen3-v1";
pub const LEGACY_GEN3_RULESET_REFERENCE: &str = "legacy-gen3@1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContentPackageManifest {
    package: ContentPackageId,
    revision: u16,
    content_version: String,
    data_reference: String,
    ruleset_reference: String,
}

impl ContentPackageManifest {
    pub fn new(
        package: ContentPackageId,
        revision: u16,
        content_version: impl Into<String>,
        data_reference: impl Into<String>,
        ruleset_reference: impl Into<String>,
    ) -> Result<Self, ContentPackageError> {
        if revision == 0 {
            return Err(ContentPackageError::InvalidRevision(revision));
        }
        let content_version = required_reference(content_version.into(), "content version")?;
        let data_reference = required_reference(data_reference.into(), "data reference")?;
        let ruleset_reference = required_reference(ruleset_reference.into(), "ruleset reference")?;
        Ok(Self {
            package,
            revision,
            content_version,
            data_reference,
            ruleset_reference,
        })
    }

    pub fn starter_region() -> Result<Self, ContentPackageError> {
        Self::new(
            ContentPackageId::new(STARTER_REGION_PACKAGE_ID)?,
            STARTER_REGION_PACKAGE_REVISION,
            CONTENT_VERSION,
            EMBEDDED_GEN3_DATA_REFERENCE,
            LEGACY_GEN3_RULESET_REFERENCE,
        )
    }

    pub fn package(&self) -> &ContentPackageId {
        &self.package
    }

    pub const fn revision(&self) -> u16 {
        self.revision
    }

    pub fn content_version(&self) -> &str {
        &self.content_version
    }

    pub fn data_reference(&self) -> &str {
        &self.data_reference
    }

    pub fn ruleset_reference(&self) -> &str {
        &self.ruleset_reference
    }

    pub fn storage_key(&self) -> String {
        format!("{}@{}", self.package.as_str(), self.revision)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentPackage {
    manifest: ContentPackageManifest,
    content: ThinSliceContent,
}

impl ContentPackage {
    pub fn new(
        manifest: ContentPackageManifest,
        content: ThinSliceContent,
    ) -> Result<Self, ContentPackageError> {
        if manifest.content_version() != content.content_version() {
            return Err(ContentPackageError::ContentVersionMismatch {
                expected: content.content_version().to_owned(),
                actual: manifest.content_version().to_owned(),
            });
        }
        Ok(Self { manifest, content })
    }

    pub fn standard() -> Result<Self, ContentPackageError> {
        Self::new(
            ContentPackageManifest::starter_region()?,
            ThinSliceContent::standard()?,
        )
    }

    pub fn manifest(&self) -> &ContentPackageManifest {
        &self.manifest
    }

    pub fn content(&self) -> &ThinSliceContent {
        &self.content
    }

    pub fn into_parts(self) -> (ContentPackageManifest, ThinSliceContent) {
        (self.manifest, self.content)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContentPackageError {
    Id(GameIdError),
    Content(ContentError),
    Json(String),
    InvalidBattleKind(crate::BattleId),
    InvalidRevision(u16),
    EmptyReference(&'static str),
    ContentVersionMismatch { expected: String, actual: String },
}

impl From<GameIdError> for ContentPackageError {
    fn from(value: GameIdError) -> Self {
        Self::Id(value)
    }
}

impl From<ContentError> for ContentPackageError {
    fn from(value: ContentError) -> Self {
        Self::Content(value)
    }
}

fn required_reference(value: String, field: &'static str) -> Result<String, ContentPackageError> {
    if value.trim().is_empty() {
        return Err(ContentPackageError::EmptyReference(field));
    }
    Ok(value)
}
