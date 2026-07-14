//! Ramus projects typed application capabilities into a path-addressed command tree.

#![forbid(unsafe_code)]

mod ast;
mod boundary;
mod catalog;
mod compiler;
mod execution;
mod model;
mod parser;
mod plan;
mod policy;
mod schema;
mod value;

pub use ast::{Argument, Call, Document, Span, Spanned};
pub use boundary::{
    AuthorizationChecker, AuthorizationRevoker, AuthorizationService, AuthorizationSession,
    CapabilityGeneration, EffectPermit, Principal, PrincipalError, Provider, Runtime,
    RuntimeConfigurationError,
};
pub use catalog::{Catalog, CatalogError, CatalogGeneration, MethodRegistration, SchemaVersion};
pub use compiler::{
    CompileLimits, Compiler, Completion, Diagnostic, DiagnosticCode, DiscoveryEntry,
};
pub use execution::{
    ExecutionError, ExecutionFailure, ExecutionReport, ProviderError, ProviderRequest,
};
pub use model::{
    Capability, Effect, MethodName, ModelError, NodePath, ParameterName, PrincipalId, ProviderId,
};
pub use parser::{
    ForbiddenSyntax, ParseDiagnostic, ParseDiagnosticKind, ParseFailure, ParseLimits,
    SourceLocation, parse, parse_with_limits,
};
pub use plan::{DraftArgument, DraftCall, PlanDraft, TypedPlan};
pub use policy::CapabilityView;
pub use schema::{MethodSchema, ParameterSchema, SchemaError, Value, ValueType};
