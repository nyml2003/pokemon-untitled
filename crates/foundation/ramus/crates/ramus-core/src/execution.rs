use std::collections::BTreeMap;

use crate::catalog::Catalog;
use crate::model::{MethodName, NodePath};
use crate::plan::{TypedCall, TypedPlan};
use crate::schema::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderRequest {
    pub path: NodePath,
    pub method: MethodName,
    pub arguments: BTreeMap<String, Value>,
}

impl From<&TypedCall> for ProviderRequest {
    fn from(call: &TypedCall) -> Self {
        Self {
            path: call.path.clone(),
            method: call.method.clone(),
            arguments: call.arguments.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderError {
    Rejected { code: String, message: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionReport {
    pub outputs: Vec<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionFailure {
    pub call_index: usize,
    pub completed_outputs: Vec<Value>,
    pub error: ExecutionError,
}

impl ExecutionFailure {
    pub(crate) fn before_any(error: ExecutionError) -> Self {
        Self::new(0, Vec::new(), error)
    }

    pub(crate) fn new(
        call_index: usize,
        completed_outputs: Vec<Value>,
        error: ExecutionError,
    ) -> Self {
        Self {
            call_index,
            completed_outputs,
            error,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionError {
    CatalogChanged,
    SchemaChanged,
    AuthorizationRevoked,
    ProviderUnavailable,
    Provider(ProviderError),
}

pub(crate) fn preflight_plan(catalog: &Catalog, plan: &TypedPlan) -> Result<(), ExecutionFailure> {
    if plan.catalog_generation() != catalog.generation() {
        return Err(ExecutionFailure::before_any(ExecutionError::CatalogChanged));
    }
    for (call_index, call) in plan.calls().iter().enumerate() {
        let Some(registered) = catalog.resolve(&call.path, &call.method) else {
            return Err(ExecutionFailure::new(
                call_index,
                Vec::new(),
                ExecutionError::CatalogChanged,
            ));
        };
        if registered.provider_id != call.provider_id
            || registered.schema_version != call.schema_version
            || registered.schema != call.schema
            || registered.effect != call.effect
        {
            return Err(ExecutionFailure::new(
                call_index,
                Vec::new(),
                ExecutionError::SchemaChanged,
            ));
        }
    }
    Ok(())
}
