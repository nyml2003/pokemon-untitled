use std::collections::BTreeMap;

use crate::model::{Effect, MethodName, NodePath, ProviderId};
use crate::schema::MethodSchema;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CatalogGeneration(u64);

impl CatalogGeneration {
    pub const fn initial() -> Self {
        Self(1)
    }

    fn next(self) -> Self {
        Self(self.0.checked_add(1).expect("catalog generation exhausted"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaVersion(u64);

impl SchemaVersion {
    pub const fn new(value: u64) -> Option<Self> {
        if value > 0 { Some(Self(value)) } else { None }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodRegistration {
    pub provider_id: ProviderId,
    pub path: NodePath,
    pub schema: MethodSchema,
    pub schema_version: SchemaVersion,
    pub effect: Effect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RegisteredMethod {
    pub provider_id: ProviderId,
    pub path: NodePath,
    pub schema: MethodSchema,
    pub schema_version: SchemaVersion,
    pub effect: Effect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalog {
    generation: CatalogGeneration,
    methods: BTreeMap<(NodePath, MethodName), RegisteredMethod>,
}

impl Catalog {
    pub fn new() -> Self {
        Self {
            generation: CatalogGeneration::initial(),
            methods: BTreeMap::new(),
        }
    }

    pub const fn generation(&self) -> CatalogGeneration {
        self.generation
    }

    pub fn len(&self) -> usize {
        self.methods.len()
    }

    pub fn is_empty(&self) -> bool {
        self.methods.is_empty()
    }

    pub fn register(&mut self, registration: MethodRegistration) -> Result<(), CatalogError> {
        let key = (
            registration.path.clone(),
            registration.schema.name().clone(),
        );
        if self.methods.contains_key(&key) {
            return Err(CatalogError::DuplicateMethod);
        }
        self.methods.insert(
            key,
            RegisteredMethod {
                provider_id: registration.provider_id,
                path: registration.path,
                schema: registration.schema,
                schema_version: registration.schema_version,
                effect: registration.effect,
            },
        );
        self.generation = self.generation.next();
        Ok(())
    }

    pub(crate) fn resolve(
        &self,
        path: &NodePath,
        method: &MethodName,
    ) -> Option<&RegisteredMethod> {
        self.methods.get(&(path.clone(), method.clone()))
    }

    pub(crate) fn methods(&self) -> impl Iterator<Item = &RegisteredMethod> {
        self.methods.values()
    }
}

impl Default for Catalog {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogError {
    DuplicateMethod,
}
