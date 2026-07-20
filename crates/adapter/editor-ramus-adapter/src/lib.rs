//! Ramus authorization and routing for logical editor resources.

#![forbid(unsafe_code)]

use std::{collections::BTreeMap, error::Error, fmt, sync::Arc};

use editor_application::{EditorCall, EditorDocumentId, EditorKind, EditorOperation};
use ramus_core::{
    AuthorizationService, Capability, Catalog, CompileLimits, Compiler, Effect, EffectPermit,
    ExecutionError, ExecutionFailure, MethodName, MethodRegistration, MethodSchema, NodePath,
    ParameterName, ParameterSchema, ParseDiagnosticKind, ParseFailure, ParseLimits, PlanDraft,
    Principal, Provider, ProviderError, ProviderId, ProviderRequest, Runtime, SchemaVersion, Value,
    ValueType, parse_with_limits,
};

const EDITOR_PRINCIPAL: &str = "editor-client";
const PROVIDER_ID: &str = "editor-resources";
const PARSE_LIMITS: ParseLimits = ParseLimits {
    max_source_bytes: 8 * 1024,
    max_calls: 32,
    max_arguments_per_call: 8,
};
const COMPILE_LIMITS: CompileLimits = CompileLimits {
    max_calls: 32,
    max_arguments_per_call: 8,
    max_total_bytes: 8 * 1024,
    max_value_bytes: 4 * 1024,
    max_value_nodes: 512,
    max_value_depth: 16,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoutedEditorIntent {
    Open {
        kind: EditorKind,
        document: EditorDocumentId,
    },
    Call(EditorCall),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditorRouterDiagnostic {
    pub stage: EditorRouterDiagnosticStage,
    pub code: String,
    pub message: String,
}

impl fmt::Display for EditorRouterDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl Error for EditorRouterDiagnostic {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditorRouterDiagnosticStage {
    Parse,
    Seal,
    Provider,
    Runtime,
    Protocol,
}

pub struct EditorRamusRouter {
    authorization: AuthorizationService,
    principal: Principal,
    compiler: Compiler,
    runtime: Runtime,
}

impl EditorRamusRouter {
    pub fn new() -> Result<Self, EditorRouterDiagnostic> {
        let provider_id = ProviderId::new(PROVIDER_ID).map_err(configuration_diagnostic)?;
        let catalog = build_catalog(&provider_id)?;
        let authorization = AuthorizationService::new();
        let principal = authorization
            .create_principal(EDITOR_PRINCIPAL)
            .map_err(configuration_diagnostic)?;
        grant_editor_capabilities(&authorization, &principal)?;
        let compiler = Compiler::new(Arc::clone(&catalog));
        let mut runtime = Runtime::new(catalog, authorization.checker());
        runtime
            .bind_provider(provider_id, Arc::new(EditorProvider))
            .map_err(configuration_diagnostic)?;
        Ok(Self {
            authorization,
            principal,
            compiler,
            runtime,
        })
    }

    /// Routes human-authored Ramus source for resource operations.
    pub fn route(&self, source: &str) -> Result<Vec<RoutedEditorIntent>, EditorRouterDiagnostic> {
        let document = parse_with_limits(source, PARSE_LIMITS).map_err(parse_diagnostic)?;
        self.route_draft(PlanDraft::from(document))
    }

    /// Routes a versioned structured call from a GUI, CLI, or model client.
    pub fn route_call(
        &self,
        call: EditorCall,
    ) -> Result<RoutedEditorIntent, EditorRouterDiagnostic> {
        call.validate().map_err(protocol_diagnostic)?;
        let (path, method, arguments) = draft_for_call(&call)?;
        let mut routed = self.route_draft(PlanDraft {
            calls: vec![ramus_core::DraftCall {
                path,
                method,
                arguments,
            }],
        })?;
        routed
            .pop()
            .ok_or_else(|| protocol_diagnostic("editor routing did not produce an intent"))
    }

    fn route_draft(
        &self,
        draft: PlanDraft,
    ) -> Result<Vec<RoutedEditorIntent>, EditorRouterDiagnostic> {
        let plan = {
            let session = self
                .authorization
                .session(&self.principal)
                .map_err(configuration_diagnostic)?;
            self.compiler
                .seal_with_limits(&session.view(), draft, COMPILE_LIMITS)
                .map_err(seal_diagnostic)?
        };
        let report = self.runtime.execute(plan).map_err(execution_diagnostic)?;
        report.outputs.iter().map(intent_from_value).collect()
    }
}

struct EditorProvider;

impl Provider for EditorProvider {
    fn execute(
        &self,
        permit: EffectPermit,
        request: &ProviderRequest,
    ) -> Result<Value, ProviderError> {
        if permit.principal().as_str() != EDITOR_PRINCIPAL
            || permit.path() != &request.path
            || permit.method() != &request.method
            || permit.capability() != Capability::from(effect_for(request)?)
        {
            return Err(rejected(
                "invalid-permit",
                "the invocation permit does not match the editor resource request",
            ));
        }
        intent_to_value(&intent_from_request(request)?)
    }
}

fn build_catalog(provider_id: &ProviderId) -> Result<Arc<Catalog>, EditorRouterDiagnostic> {
    let mut catalog = Catalog::new();
    for (path, schema, effect) in schemas()? {
        let schema_version = SchemaVersion::new(1)
            .ok_or_else(|| configuration_diagnostic("schema version must be non-zero"))?;
        catalog
            .register(MethodRegistration {
                provider_id: provider_id.clone(),
                path,
                schema,
                schema_version,
                effect,
            })
            .map_err(configuration_diagnostic)?;
    }
    Ok(Arc::new(catalog))
}

fn schemas() -> Result<Vec<(NodePath, MethodSchema, Effect)>, EditorRouterDiagnostic> {
    let resource_parameters = || {
        Ok(vec![
            parameter("kind", ValueType::Enum(editor_kind_values()))?,
            parameter("document", ValueType::String)?,
        ])
    };
    Ok(vec![
        schema(
            "/editor/resource",
            "open",
            resource_parameters()?,
            Effect::Read,
        )?,
        schema(
            "/editor/resource",
            "inspect",
            resource_parameters()?,
            Effect::Read,
        )?,
        schema(
            "/editor/resource",
            "validate",
            resource_parameters()?,
            Effect::Read,
        )?,
        schema(
            "/editor/resource",
            "save",
            resource_parameters()?,
            Effect::Write,
        )?,
        schema(
            "/editor/command",
            "execute",
            vec![
                parameter("kind", ValueType::Enum(editor_kind_values()))?,
                parameter("document", ValueType::String)?,
                parameter("payload", ValueType::Record)?,
            ],
            Effect::Write,
        )?,
    ])
}

fn schema(
    path: &str,
    method: &str,
    parameters: Vec<ParameterSchema>,
    effect: Effect,
) -> Result<(NodePath, MethodSchema, Effect), EditorRouterDiagnostic> {
    let method = MethodName::new(method).map_err(configuration_diagnostic)?;
    let schema = MethodSchema::new(method, parameters).map_err(configuration_diagnostic)?;
    Ok((
        NodePath::parse(path).map_err(configuration_diagnostic)?,
        schema,
        effect,
    ))
}

fn parameter(name: &str, value_type: ValueType) -> Result<ParameterSchema, EditorRouterDiagnostic> {
    Ok(ParameterSchema {
        name: ParameterName::new(name).map_err(configuration_diagnostic)?,
        value_type,
        required: true,
        positional: false,
    })
}

fn editor_kind_values() -> Vec<String> {
    ["map", "trainer", "pokemon"]
        .into_iter()
        .map(String::from)
        .collect()
}

fn grant_editor_capabilities(
    authorization: &AuthorizationService,
    principal: &Principal,
) -> Result<(), EditorRouterDiagnostic> {
    for (path, schema, effect) in schemas()? {
        for capability in [
            Capability::Discover,
            Capability::Complete,
            Capability::from(effect),
        ] {
            authorization
                .grant(
                    principal,
                    path.clone(),
                    Some(schema.name().clone()),
                    capability,
                )
                .map_err(configuration_diagnostic)?;
        }
    }
    Ok(())
}

fn draft_for_call(
    call: &EditorCall,
) -> Result<(String, String, Vec<ramus_core::DraftArgument>), EditorRouterDiagnostic> {
    let mut arguments = vec![
        named("kind", Value::String(kind_name(call.kind()).to_owned())),
        named(
            "document",
            Value::String(call.document().as_str().to_owned()),
        ),
    ];
    let (path, method) = match call.operation() {
        EditorOperation::Inspect => ("/editor/resource", "inspect"),
        EditorOperation::Validate => ("/editor/resource", "validate"),
        EditorOperation::Save => ("/editor/resource", "save"),
        EditorOperation::Command => {
            let payload = json_to_value(call.payload()).map_err(protocol_diagnostic)?;
            if !matches!(payload, Value::Record(_)) {
                return Err(protocol_diagnostic(
                    "editor command payload must be a JSON object",
                ));
            }
            arguments.push(named("payload", payload));
            ("/editor/command", "execute")
        }
    };
    Ok((path.to_owned(), method.to_owned(), arguments))
}

fn intent_from_request(request: &ProviderRequest) -> Result<RoutedEditorIntent, ProviderError> {
    let kind = kind_argument(request)?;
    let document = EditorDocumentId::new(string_argument(request, "document")?)
        .map_err(|error| rejected("invalid-document", error.to_string()))?;
    match (request.path.as_str(), request.method.as_str()) {
        ("/editor/resource", "open") => Ok(RoutedEditorIntent::Open { kind, document }),
        ("/editor/resource", "inspect") => {
            call(kind, document, EditorOperation::Inspect, Value::Unit)
        }
        ("/editor/resource", "validate") => {
            call(kind, document, EditorOperation::Validate, Value::Unit)
        }
        ("/editor/resource", "save") => call(kind, document, EditorOperation::Save, Value::Unit),
        ("/editor/command", "execute") => call(
            kind,
            document,
            EditorOperation::Command,
            argument(request, "payload")?,
        ),
        _ => Err(rejected(
            "unknown-resource-operation",
            "unknown editor resource operation",
        )),
    }
}

fn call(
    kind: EditorKind,
    document: EditorDocumentId,
    operation: EditorOperation,
    payload: Value,
) -> Result<RoutedEditorIntent, ProviderError> {
    let payload = value_to_json(&payload)?;
    EditorCall::new(kind, document, operation, payload)
        .map(RoutedEditorIntent::Call)
        .map_err(|error| rejected("invalid-editor-call", error.to_string()))
}

fn intent_to_value(intent: &RoutedEditorIntent) -> Result<Value, ProviderError> {
    let mut record = BTreeMap::new();
    match intent {
        RoutedEditorIntent::Open { kind, document } => {
            record.insert(
                String::from("operation"),
                Value::String(String::from("open")),
            );
            record.insert(
                String::from("kind"),
                Value::String(kind_name(*kind).to_owned()),
            );
            record.insert(
                String::from("document"),
                Value::String(document.as_str().to_owned()),
            );
        }
        RoutedEditorIntent::Call(call) => {
            record.insert(
                String::from("operation"),
                Value::String(operation_name(call.operation()).to_owned()),
            );
            record.insert(
                String::from("kind"),
                Value::String(kind_name(call.kind()).to_owned()),
            );
            record.insert(
                String::from("document"),
                Value::String(call.document().as_str().to_owned()),
            );
            record.insert(
                String::from("payload"),
                json_to_value(call.payload())
                    .map_err(|error| rejected("invalid-payload", error))?,
            );
        }
    }
    Ok(Value::Record(record))
}

fn intent_from_value(value: &Value) -> Result<RoutedEditorIntent, EditorRouterDiagnostic> {
    let Value::Record(record) = value else {
        return Err(protocol_diagnostic(
            "editor provider did not return a record",
        ));
    };
    let kind = kind_from_value(record.get("kind")).map_err(protocol_diagnostic)?;
    let document = document_from_value(record.get("document")).map_err(protocol_diagnostic)?;
    let operation = string_from_value(record.get("operation")).map_err(protocol_diagnostic)?;
    if operation == "open" {
        return Ok(RoutedEditorIntent::Open { kind, document });
    }
    let operation = operation_from_name(operation).map_err(protocol_diagnostic)?;
    let payload = record
        .get("payload")
        .map(value_to_json)
        .transpose()
        .map_err(|error| protocol_diagnostic(format!("{error:?}")))?
        .unwrap_or(serde_json::Value::Null);
    EditorCall::new(kind, document, operation, payload)
        .map(RoutedEditorIntent::Call)
        .map_err(protocol_diagnostic)
}

fn effect_for(request: &ProviderRequest) -> Result<Effect, ProviderError> {
    match (request.path.as_str(), request.method.as_str()) {
        ("/editor/resource", "open" | "inspect" | "validate") => Ok(Effect::Read),
        ("/editor/resource", "save") | ("/editor/command", "execute") => Ok(Effect::Write),
        _ => Err(rejected(
            "unknown-resource-operation",
            "unknown editor resource operation",
        )),
    }
}

fn argument(request: &ProviderRequest, name: &str) -> Result<Value, ProviderError> {
    request
        .arguments
        .get(name)
        .cloned()
        .ok_or_else(|| rejected("missing-argument", format!("missing argument: {name}")))
}

fn string_argument(request: &ProviderRequest, name: &str) -> Result<String, ProviderError> {
    match argument(request, name)? {
        Value::String(ref value) => Ok(value.clone()),
        _ => Err(rejected(
            "invalid-argument",
            format!("{name} must be a string"),
        )),
    }
}

fn kind_argument(request: &ProviderRequest) -> Result<EditorKind, ProviderError> {
    kind_from_name(&string_argument(request, "kind")?)
        .map_err(|error| rejected("invalid-kind", error))
}

fn named(name: &str, value: Value) -> ramus_core::DraftArgument {
    ramus_core::DraftArgument {
        name: Some(name.to_owned()),
        value,
    }
}

fn kind_name(kind: EditorKind) -> &'static str {
    match kind {
        EditorKind::Map => "map",
        EditorKind::Trainer => "trainer",
        EditorKind::Pokemon => "pokemon",
    }
}

fn kind_from_name(value: &str) -> Result<EditorKind, String> {
    match value {
        "map" => Ok(EditorKind::Map),
        "trainer" => Ok(EditorKind::Trainer),
        "pokemon" => Ok(EditorKind::Pokemon),
        _ => Err(format!("unknown editor kind: {value}")),
    }
}

fn kind_from_value(value: Option<&Value>) -> Result<EditorKind, String> {
    kind_from_name(string_from_value(value)?)
}

fn document_from_value(value: Option<&Value>) -> Result<EditorDocumentId, String> {
    EditorDocumentId::new(string_from_value(value)?)
        .map_err(|error| format!("invalid editor document: {error}"))
}

fn string_from_value(value: Option<&Value>) -> Result<&str, String> {
    match value {
        Some(Value::String(value)) => Ok(value),
        _ => Err(String::from(
            "editor provider record has an invalid string field",
        )),
    }
}

fn operation_name(operation: EditorOperation) -> &'static str {
    match operation {
        EditorOperation::Inspect => "inspect",
        EditorOperation::Validate => "validate",
        EditorOperation::Command => "command",
        EditorOperation::Save => "save",
    }
}

fn operation_from_name(value: &str) -> Result<EditorOperation, String> {
    match value {
        "inspect" => Ok(EditorOperation::Inspect),
        "validate" => Ok(EditorOperation::Validate),
        "command" => Ok(EditorOperation::Command),
        "save" => Ok(EditorOperation::Save),
        _ => Err(format!("unknown editor operation: {value}")),
    }
}

fn json_to_value(value: &serde_json::Value) -> Result<Value, String> {
    match value {
        serde_json::Value::Null => Ok(Value::Unit),
        serde_json::Value::Bool(value) => Ok(Value::Boolean(*value)),
        serde_json::Value::Number(value) => value
            .as_i64()
            .map(Value::Integer)
            .ok_or_else(|| String::from("editor protocol does not support non-integer numbers")),
        serde_json::Value::String(value) => Ok(Value::String(value.clone())),
        serde_json::Value::Array(values) => values
            .iter()
            .map(json_to_value)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::List),
        serde_json::Value::Object(values) => values
            .iter()
            .map(|(name, value)| Ok((name.clone(), json_to_value(value)?)))
            .collect::<Result<BTreeMap<_, _>, String>>()
            .map(Value::Record),
    }
}

fn value_to_json(value: &Value) -> Result<serde_json::Value, ProviderError> {
    match value {
        Value::Unit => Ok(serde_json::Value::Null),
        Value::Boolean(value) => Ok(serde_json::Value::Bool(*value)),
        Value::Integer(value) => Ok(serde_json::Value::Number((*value).into())),
        Value::String(value) => Ok(serde_json::Value::String(value.clone())),
        Value::List(values) => values
            .iter()
            .map(value_to_json)
            .collect::<Result<Vec<_>, _>>()
            .map(serde_json::Value::Array),
        Value::Record(values) => values
            .iter()
            .map(|(name, value)| Ok((name.clone(), value_to_json(value)?)))
            .collect::<Result<serde_json::Map<_, _>, ProviderError>>()
            .map(serde_json::Value::Object),
    }
}

fn parse_diagnostic(failure: ParseFailure) -> EditorRouterDiagnostic {
    let Some(diagnostic) = failure.diagnostics().first() else {
        return EditorRouterDiagnostic {
            stage: EditorRouterDiagnosticStage::Parse,
            code: String::from("parse-failure"),
            message: String::from("editor resource text could not be parsed"),
        };
    };
    EditorRouterDiagnostic {
        stage: EditorRouterDiagnosticStage::Parse,
        code: parse_diagnostic_code(&diagnostic.kind).to_owned(),
        message: diagnostic.to_string(),
    }
}

fn parse_diagnostic_code(kind: &ParseDiagnosticKind) -> &'static str {
    match kind {
        ParseDiagnosticKind::SourceTooLarge => "source-too-large",
        ParseDiagnosticKind::InvalidSourceBoundary => "invalid-source-boundary",
        ParseDiagnosticKind::TooManyCalls => "too-many-calls",
        ParseDiagnosticKind::TooManyArguments => "too-many-arguments",
        ParseDiagnosticKind::EmptyInput => "empty-input",
        ParseDiagnosticKind::EmptyStatement => "empty-statement",
        ParseDiagnosticKind::ExpectedNodePath => "expected-node-path",
        ParseDiagnosticKind::InvalidNodePath { .. } => "invalid-node-path",
        ParseDiagnosticKind::ExpectedMethod => "expected-method",
        ParseDiagnosticKind::InvalidMethodName { .. } => "invalid-method-name",
        ParseDiagnosticKind::ExpectedArgument => "expected-argument",
        ParseDiagnosticKind::InvalidParameterName { .. } => "invalid-parameter-name",
        ParseDiagnosticKind::MissingArgumentValue => "missing-argument-value",
        ParseDiagnosticKind::WhitespaceAroundEquals => "whitespace-around-equals",
        ParseDiagnosticKind::MissingWhitespace => "missing-whitespace",
        ParseDiagnosticKind::UnterminatedString => "unterminated-string",
        ParseDiagnosticKind::InvalidEscape { .. } => "invalid-escape",
        ParseDiagnosticKind::IntegerOutOfRange { .. } => "integer-out-of-range",
        ParseDiagnosticKind::ForbiddenSyntax(_) => "forbidden-syntax",
        ParseDiagnosticKind::UnexpectedCharacter { .. } => "unexpected-character",
    }
}

fn seal_diagnostic(diagnostic: ramus_core::Diagnostic) -> EditorRouterDiagnostic {
    EditorRouterDiagnostic {
        stage: EditorRouterDiagnosticStage::Seal,
        code: diagnostic.code.as_str().to_owned(),
        message: diagnostic.message,
    }
}

fn execution_diagnostic(failure: ExecutionFailure) -> EditorRouterDiagnostic {
    let (stage, code, message) = match failure.error {
        ExecutionError::Provider(ProviderError::Rejected { code, message }) => {
            (EditorRouterDiagnosticStage::Provider, code, message)
        }
        error => (
            EditorRouterDiagnosticStage::Runtime,
            "runtime".to_owned(),
            format!("{error:?}"),
        ),
    };
    EditorRouterDiagnostic {
        stage,
        code,
        message,
    }
}

fn configuration_diagnostic(error: impl std::fmt::Debug) -> EditorRouterDiagnostic {
    EditorRouterDiagnostic {
        stage: EditorRouterDiagnosticStage::Runtime,
        code: String::from("configuration"),
        message: format!("{error:?}"),
    }
}

fn protocol_diagnostic(error: impl ToString) -> EditorRouterDiagnostic {
    EditorRouterDiagnostic {
        stage: EditorRouterDiagnosticStage::Protocol,
        code: String::from("protocol"),
        message: error.to_string(),
    }
}

fn rejected(code: impl Into<String>, message: impl Into<String>) -> ProviderError {
    ProviderError::Rejected {
        code: code.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn structured_command_routes_through_ramus() -> Result<(), EditorRouterDiagnostic> {
        let router = EditorRamusRouter::new()?;
        let call = EditorCall::new(
            EditorKind::Trainer,
            EditorDocumentId::new("route-trainers").map_err(protocol_diagnostic)?,
            EditorOperation::Command,
            json!({"edit": {"set_name": {"trainer": "route-rival", "name": "小遥"}}}),
        )
        .map_err(protocol_diagnostic)?;
        let result = router.route_call(call)?;
        assert!(matches!(result, RoutedEditorIntent::Call(_)));
        Ok(())
    }

    #[test]
    fn human_resource_open_routes_through_the_same_policy() -> Result<(), EditorRouterDiagnostic> {
        let router = EditorRamusRouter::new()?;
        let result = router.route("/editor/resource open kind=trainer document=route-trainers")?;
        assert!(matches!(
            result.as_slice(),
            [RoutedEditorIntent::Open { .. }]
        ));
        Ok(())
    }
}
