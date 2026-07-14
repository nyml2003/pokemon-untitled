use std::collections::BTreeMap;

use crate::ast::{Argument, Document};
use crate::boundary::{CapabilityGeneration, Principal};
use crate::catalog::{CatalogGeneration, SchemaVersion};
use crate::model::{Effect, MethodName, NodePath, PrincipalId, ProviderId};
use crate::schema::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftArgument {
    pub name: Option<String>,
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftCall {
    pub path: String,
    pub method: String,
    pub arguments: Vec<DraftArgument>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanDraft {
    pub calls: Vec<DraftCall>,
}

impl From<Document> for PlanDraft {
    fn from(document: Document) -> Self {
        Self {
            calls: document
                .calls
                .into_iter()
                .map(|call| DraftCall {
                    path: call.path.value.as_str().to_owned(),
                    method: call.method.value.as_str().to_owned(),
                    arguments: call
                        .arguments
                        .into_iter()
                        .map(|argument| match argument {
                            Argument::Positional(value) => DraftArgument {
                                name: None,
                                value: value.value,
                            },
                            Argument::Named { name, value, .. } => DraftArgument {
                                name: Some(name.value.as_str().to_owned()),
                                value: value.value,
                            },
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

pub struct TypedPlan {
    principal: Principal,
    catalog_generation: CatalogGeneration,
    capability_generation: CapabilityGeneration,
    calls: Vec<TypedCall>,
}

pub(crate) struct TypedCall {
    pub provider_id: ProviderId,
    pub path: NodePath,
    pub method: MethodName,
    pub schema_version: SchemaVersion,
    pub schema: crate::schema::MethodSchema,
    pub arguments: BTreeMap<String, Value>,
    pub effect: Effect,
}

impl TypedPlan {
    pub(crate) fn seal(
        principal: Principal,
        catalog_generation: CatalogGeneration,
        capability_generation: CapabilityGeneration,
        calls: Vec<TypedCall>,
    ) -> Self {
        Self {
            principal,
            catalog_generation,
            capability_generation,
            calls,
        }
    }

    pub fn principal(&self) -> &PrincipalId {
        self.principal.id()
    }

    pub const fn catalog_generation(&self) -> CatalogGeneration {
        self.catalog_generation
    }

    pub const fn capability_generation(&self) -> CapabilityGeneration {
        self.capability_generation
    }

    pub fn len(&self) -> usize {
        self.calls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    pub(crate) fn calls(&self) -> &[TypedCall] {
        &self.calls
    }

    pub(crate) fn principal_handle(&self) -> &Principal {
        &self.principal
    }
}
