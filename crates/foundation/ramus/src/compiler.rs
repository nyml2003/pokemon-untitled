use std::collections::BTreeMap;
use std::sync::Arc;

use crate::catalog::Catalog;
use crate::model::{Capability, MethodName, NodePath};
use crate::plan::{DraftArgument, PlanDraft, TypedCall, TypedPlan};
use crate::policy::CapabilityView;
use crate::schema::{MethodSchema, ParameterSchema, Value, ValueType};

const HIDDEN_OPERATION: &str = "operation is unavailable";

pub struct Compiler {
    catalog: Arc<Catalog>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompileLimits {
    pub max_calls: usize,
    pub max_arguments_per_call: usize,
    pub max_total_bytes: usize,
    pub max_value_bytes: usize,
    pub max_value_nodes: usize,
    pub max_value_depth: usize,
}

impl Default for CompileLimits {
    fn default() -> Self {
        Self {
            max_calls: 64,
            max_arguments_per_call: 64,
            max_total_bytes: 64 * 1024,
            max_value_bytes: 16 * 1024,
            max_value_nodes: 4 * 1024,
            max_value_depth: 32,
        }
    }
}

impl Compiler {
    pub fn new(catalog: Arc<Catalog>) -> Self {
        Self { catalog }
    }

    pub fn discover(&self, view: &CapabilityView<'_>) -> Vec<DiscoveryEntry> {
        self.catalog
            .methods()
            .filter(|registered| {
                view.allows(
                    &registered.path,
                    registered.schema.name(),
                    Capability::Discover,
                )
            })
            .map(|registered| DiscoveryEntry {
                path: registered.path.clone(),
                method: registered.schema.name().clone(),
                schema: registered.schema.clone(),
            })
            .collect()
    }

    pub fn complete(&self, view: &CapabilityView<'_>, prefix: &str) -> Vec<Completion> {
        self.catalog
            .methods()
            .filter(|registered| {
                view.allows(
                    &registered.path,
                    registered.schema.name(),
                    Capability::Complete,
                )
            })
            .filter_map(|registered| {
                let invocation = format!(
                    "{} {}",
                    registered.path.as_str(),
                    registered.schema.name().as_str()
                );
                invocation.starts_with(prefix).then(|| Completion {
                    invocation,
                    parameters: registered
                        .schema
                        .parameters()
                        .iter()
                        .map(|parameter| parameter.name.as_str().to_owned())
                        .collect(),
                })
            })
            .collect()
    }

    pub fn seal(
        &self,
        view: &CapabilityView<'_>,
        draft: PlanDraft,
    ) -> Result<TypedPlan, Diagnostic> {
        self.seal_with_limits(view, draft, CompileLimits::default())
    }

    pub fn seal_with_limits(
        &self,
        view: &CapabilityView<'_>,
        draft: PlanDraft,
        limits: CompileLimits,
    ) -> Result<TypedPlan, Diagnostic> {
        if draft.calls.is_empty() {
            return Err(Diagnostic::new(DiagnosticCode::EmptyPlan, "plan is empty"));
        }
        if draft.calls.len() > limits.max_calls {
            return Err(Diagnostic::new(
                DiagnosticCode::TooManyCalls,
                "plan call count exceeds its limit",
            ));
        }

        validate_draft_limits(&draft, limits)?;

        let mut calls = Vec::with_capacity(draft.calls.len());
        for call in draft.calls {
            let path = NodePath::parse(call.path).map_err(|_| unavailable())?;
            let method = MethodName::new(call.method).map_err(|_| unavailable())?;
            let Some(registered) = self.catalog.resolve(&path, &method) else {
                return Err(unavailable());
            };
            if !view.allows(&path, &method, Capability::Discover)
                || !view.allows(&path, &method, registered.effect.into())
            {
                return Err(unavailable());
            }
            let arguments = validate_arguments(&registered.schema, call.arguments)?;
            calls.push(TypedCall {
                provider_id: registered.provider_id.clone(),
                path,
                method,
                schema_version: registered.schema_version,
                schema: registered.schema.clone(),
                arguments,
                effect: registered.effect,
            });
        }

        Ok(TypedPlan::seal(
            view.principal().clone(),
            self.catalog.generation(),
            view.generation(),
            calls,
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveryEntry {
    pub path: NodePath,
    pub method: MethodName,
    pub schema: MethodSchema,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
    pub invocation: String,
    pub parameters: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticCode {
    EmptyPlan,
    TooManyCalls,
    TooManyArguments,
    InputTooLarge,
    ValueTooLarge,
    TooManyValueNodes,
    ValueTooDeep,
    OperationUnavailable,
    UnknownParameter,
    DuplicateParameter,
    MissingParameter,
    TooManyPositionals,
    InvalidValue,
}

impl DiagnosticCode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::EmptyPlan => "empty-plan",
            Self::TooManyCalls => "too-many-calls",
            Self::TooManyArguments => "too-many-arguments",
            Self::InputTooLarge => "input-too-large",
            Self::ValueTooLarge => "value-too-large",
            Self::TooManyValueNodes => "too-many-value-nodes",
            Self::ValueTooDeep => "value-too-deep",
            Self::OperationUnavailable => "operation-unavailable",
            Self::UnknownParameter => "unknown-parameter",
            Self::DuplicateParameter => "duplicate-parameter",
            Self::MissingParameter => "missing-parameter",
            Self::TooManyPositionals => "too-many-positionals",
            Self::InvalidValue => "invalid-value",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub parameter: Option<String>,
}

impl Diagnostic {
    fn new(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            parameter: None,
        }
    }

    fn parameter(
        code: DiagnosticCode,
        parameter: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            parameter: Some(parameter.into()),
        }
    }
}

fn unavailable() -> Diagnostic {
    Diagnostic::new(DiagnosticCode::OperationUnavailable, HIDDEN_OPERATION)
}

fn validate_draft_limits(draft: &PlanDraft, limits: CompileLimits) -> Result<(), Diagnostic> {
    let mut total_bytes = 0usize;
    let mut total_nodes = 0usize;

    for call in &draft.calls {
        if call.arguments.len() > limits.max_arguments_per_call {
            return Err(Diagnostic::new(
                DiagnosticCode::TooManyArguments,
                "call argument count exceeds its limit",
            ));
        }
        total_bytes = total_bytes
            .saturating_add(call.path.len())
            .saturating_add(call.method.len());

        for argument in &call.arguments {
            total_bytes = total_bytes.saturating_add(argument.name.as_ref().map_or(0, String::len));
            let remaining_nodes = limits.max_value_nodes.saturating_sub(total_nodes);
            let measurement = measure_value(&argument.value, limits, remaining_nodes)?;
            total_bytes = total_bytes.saturating_add(measurement.bytes);
            total_nodes = total_nodes.saturating_add(measurement.nodes);
        }
    }

    if total_bytes > limits.max_total_bytes {
        return Err(Diagnostic::new(
            DiagnosticCode::InputTooLarge,
            "plan byte size exceeds its limit",
        ));
    }
    Ok(())
}

struct ValueMeasurement {
    bytes: usize,
    nodes: usize,
}

fn measure_value(
    value: &Value,
    limits: CompileLimits,
    max_nodes: usize,
) -> Result<ValueMeasurement, Diagnostic> {
    let mut bytes = 0usize;
    let mut nodes = 0usize;
    measure_value_node(value, limits, max_nodes, 1, &mut bytes, &mut nodes)?;

    Ok(ValueMeasurement { bytes, nodes })
}

fn measure_value_node(
    value: &Value,
    limits: CompileLimits,
    max_nodes: usize,
    depth: usize,
    bytes: &mut usize,
    nodes: &mut usize,
) -> Result<(), Diagnostic> {
    if depth > limits.max_value_depth {
        return Err(Diagnostic::new(
            DiagnosticCode::ValueTooDeep,
            "value nesting depth exceeds its limit",
        ));
    }
    *nodes = nodes.saturating_add(1);
    if *nodes > max_nodes {
        return Err(Diagnostic::new(
            DiagnosticCode::TooManyValueNodes,
            "plan value node count exceeds its limit",
        ));
    }
    match value {
        Value::String(value) => *bytes = bytes.saturating_add(value.len()),
        Value::Integer(_) => *bytes = bytes.saturating_add(size_of::<i64>()),
        Value::Boolean(_) => *bytes = bytes.saturating_add(size_of::<bool>()),
        Value::List(values) => {
            for value in values {
                measure_value_node(
                    value,
                    limits,
                    max_nodes,
                    depth.saturating_add(1),
                    bytes,
                    nodes,
                )?;
            }
        }
        Value::Record(values) => {
            for (name, value) in values {
                *bytes = bytes.saturating_add(name.len());
                measure_value_node(
                    value,
                    limits,
                    max_nodes,
                    depth.saturating_add(1),
                    bytes,
                    nodes,
                )?;
            }
        }
        Value::Unit => {}
    }
    if *bytes > limits.max_value_bytes {
        return Err(Diagnostic::new(
            DiagnosticCode::ValueTooLarge,
            "argument value byte size exceeds its limit",
        ));
    }
    Ok(())
}

fn validate_arguments(
    schema: &MethodSchema,
    draft_arguments: Vec<DraftArgument>,
) -> Result<BTreeMap<String, Value>, Diagnostic> {
    let positional: Vec<&ParameterSchema> = schema
        .parameters()
        .iter()
        .filter(|parameter| parameter.positional)
        .collect();
    let mut next_positional = 0;
    let mut values = BTreeMap::new();

    for argument in draft_arguments {
        let parameter = if let Some(name) = argument.name {
            schema
                .parameters()
                .iter()
                .find(|parameter| parameter.name.as_str() == name)
                .ok_or_else(|| {
                    Diagnostic::parameter(
                        DiagnosticCode::UnknownParameter,
                        name.clone(),
                        format!("unknown parameter: {name}"),
                    )
                })?
        } else {
            let Some(parameter) = positional.get(next_positional) else {
                return Err(Diagnostic::new(
                    DiagnosticCode::TooManyPositionals,
                    "too many positional arguments",
                ));
            };
            next_positional += 1;
            parameter
        };

        let name = parameter.name.as_str().to_owned();
        if values.contains_key(&name) {
            return Err(Diagnostic::parameter(
                DiagnosticCode::DuplicateParameter,
                name.clone(),
                format!("duplicate parameter: {name}"),
            ));
        }
        values.insert(name, validate_value(argument.value, &parameter.value_type)?);
    }

    for parameter in schema.parameters() {
        if parameter.required && !values.contains_key(parameter.name.as_str()) {
            return Err(Diagnostic::parameter(
                DiagnosticCode::MissingParameter,
                parameter.name.as_str(),
                format!("missing parameter: {}", parameter.name.as_str()),
            ));
        }
    }
    Ok(values)
}

fn validate_value(value: Value, expected: &ValueType) -> Result<Value, Diagnostic> {
    let valid = match (expected, &value) {
        (ValueType::String, Value::String(_))
        | (ValueType::Integer, Value::Integer(_))
        | (ValueType::Boolean, Value::Boolean(_)) => true,
        (ValueType::Enum(variants), Value::String(value)) => {
            variants.iter().any(|variant| variant == value)
        }
        (ValueType::List, Value::List(_)) | (ValueType::Record, Value::Record(_)) => true,
        _ => false,
    };
    if valid {
        Ok(value)
    } else {
        Err(Diagnostic::new(
            DiagnosticCode::InvalidValue,
            format!("value does not match {expected:?}"),
        ))
    }
}
