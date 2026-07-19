//! Capability-filtered Ramus boundary for player battle actions.

#![forbid(unsafe_code)]

use std::sync::Arc;

use battle_application::{Action, MoveSlot, TeamSlot};
use ramus_core::{
    AuthorizationService, Capability, Catalog, CompileLimits, Compiler, Diagnostic, Effect,
    EffectPermit, ExecutionError, ExecutionFailure, MethodName, MethodRegistration, MethodSchema,
    NodePath, ParseDiagnosticKind, ParseFailure, ParseLimits, PlanDraft, Principal, Provider,
    ProviderError, ProviderId, ProviderRequest, Runtime, SchemaVersion, Value, parse_with_limits,
};

const PLAYER_ID: &str = "local-player";
const PROVIDER_ID: &str = "gen3-battle";
const STRUGGLE_INVOCATION: (&str, &str) = ("/battle/action", "struggle");
const MOVE_INVOCATIONS: [(&str, &str); 4] = [
    ("/battle/move/one", "use"),
    ("/battle/move/two", "use"),
    ("/battle/move/three", "use"),
    ("/battle/move/four", "use"),
];
const SWITCH_INVOCATIONS: [(&str, &str); 6] = [
    ("/battle/team/one", "switch"),
    ("/battle/team/two", "switch"),
    ("/battle/team/three", "switch"),
    ("/battle/team/four", "switch"),
    ("/battle/team/five", "switch"),
    ("/battle/team/six", "switch"),
];

const PARSE_LIMITS: ParseLimits = ParseLimits {
    max_source_bytes: 256,
    max_calls: 1,
    max_arguments_per_call: 0,
};

const COMPILE_LIMITS: CompileLimits = CompileLimits {
    max_calls: 1,
    max_arguments_per_call: 0,
    max_total_bytes: 256,
    max_value_bytes: 0,
    max_value_nodes: 0,
    max_value_depth: 0,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionInvocation {
    pub action: Action,
    pub invocation: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticStage {
    Parse,
    Seal,
    Provider,
    Runtime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterDiagnostic {
    pub stage: DiagnosticStage,
    pub code: String,
    pub message: String,
}

pub struct BattleRamusAdapter {
    authorization: AuthorizationService,
    principal: Principal,
    compiler: Compiler,
    runtime: Runtime,
}

impl BattleRamusAdapter {
    pub fn new() -> Result<Self, AdapterDiagnostic> {
        let provider_id = ProviderId::new(PROVIDER_ID).map_err(configuration_diagnostic)?;
        let catalog = build_catalog(&provider_id)?;
        let authorization = AuthorizationService::new();
        let principal = authorization
            .create_principal(PLAYER_ID)
            .map_err(configuration_diagnostic)?;
        grant_player_actions(&authorization, &principal)?;

        let compiler = Compiler::new(Arc::clone(&catalog));
        let mut runtime = Runtime::new(catalog, authorization.checker());
        runtime
            .bind_provider(provider_id, Arc::new(BattleProvider))
            .map_err(configuration_diagnostic)?;

        Ok(Self {
            authorization,
            principal,
            compiler,
            runtime,
        })
    }

    pub fn action_invocations(
        &self,
        legal_actions: &[Action],
    ) -> Result<Vec<ActionInvocation>, AdapterDiagnostic> {
        let session = self
            .authorization
            .session(&self.principal)
            .map_err(configuration_diagnostic)?;
        let mut invocations = self
            .compiler
            .discover(&session.view())
            .into_iter()
            .filter_map(|entry| {
                let invocation = format!("{} {}", entry.path.as_str(), entry.method.as_str());
                let action = action_for_parts(entry.path.as_str(), entry.method.as_str())?;
                legal_actions
                    .contains(&action)
                    .then_some(ActionInvocation { action, invocation })
            })
            .collect::<Vec<_>>();
        invocations.sort_by(|left, right| left.invocation.cmp(&right.invocation));
        Ok(invocations)
    }

    pub fn execute_invocation(&self, invocation: &str) -> Result<Action, AdapterDiagnostic> {
        let document = parse_with_limits(invocation, PARSE_LIMITS).map_err(parse_diagnostic)?;
        let plan = {
            let session = self
                .authorization
                .session(&self.principal)
                .map_err(configuration_diagnostic)?;
            self.compiler
                .seal_with_limits(&session.view(), PlanDraft::from(document), COMPILE_LIMITS)
                .map_err(seal_diagnostic)?
        };
        let report = self.runtime.execute(plan).map_err(execution_diagnostic)?;
        action_from_provider_output(&report.outputs)
    }
}

struct BattleProvider;

impl Provider for BattleProvider {
    fn execute(
        &self,
        permit: EffectPermit,
        request: &ProviderRequest,
    ) -> Result<Value, ProviderError> {
        validate_provider_request(
            permit.principal().as_str(),
            permit.capability(),
            permit.path(),
            permit.method(),
            request,
        )?;
        Ok(Value::String(format!(
            "{} {}",
            request.path.as_str(),
            request.method.as_str()
        )))
    }
}

fn validate_provider_request(
    principal: &str,
    capability: Capability,
    path: &NodePath,
    method: &MethodName,
    request: &ProviderRequest,
) -> Result<(), ProviderError> {
    if principal != PLAYER_ID
        || capability != Capability::Invoke
        || path != &request.path
        || method != &request.method
    {
        return Err(rejected(
            "invalid-permit",
            "the invocation permit does not match the provider request",
        ));
    }
    if !request.arguments.is_empty() {
        return Err(rejected(
            "unexpected-arguments",
            "battle palette commands do not accept arguments",
        ));
    }
    action_for_parts(request.path.as_str(), request.method.as_str())
        .ok_or_else(|| {
            rejected(
                "unknown-action",
                "the battle provider does not implement this invocation",
            )
        })
        .map(drop)
}

fn build_catalog(provider_id: &ProviderId) -> Result<Arc<Catalog>, AdapterDiagnostic> {
    let mut catalog = Catalog::new();
    for (path, method) in all_invocations() {
        let path = NodePath::parse(path).map_err(configuration_diagnostic)?;
        let method = MethodName::new(method).map_err(configuration_diagnostic)?;
        let schema = MethodSchema::new(method, vec![]).map_err(configuration_diagnostic)?;
        let schema_version = SchemaVersion::new(1)
            .ok_or_else(|| configuration_diagnostic("schema version must be non-zero"))?;
        catalog
            .register(MethodRegistration {
                provider_id: provider_id.clone(),
                path,
                schema,
                schema_version,
                effect: Effect::Invoke,
            })
            .map_err(configuration_diagnostic)?;
    }
    Ok(Arc::new(catalog))
}

fn grant_player_actions(
    authorization: &AuthorizationService,
    principal: &Principal,
) -> Result<(), AdapterDiagnostic> {
    for (path, method) in all_invocations() {
        let path = NodePath::parse(path).map_err(configuration_diagnostic)?;
        let method = MethodName::new(method).map_err(configuration_diagnostic)?;
        for capability in [
            Capability::Discover,
            Capability::Complete,
            Capability::Invoke,
        ] {
            authorization
                .grant(principal, path.clone(), Some(method.clone()), capability)
                .map_err(configuration_diagnostic)?;
        }
    }
    Ok(())
}

fn all_invocations() -> impl Iterator<Item = (&'static str, &'static str)> {
    MOVE_INVOCATIONS
        .into_iter()
        .chain(SWITCH_INVOCATIONS)
        .chain([STRUGGLE_INVOCATION])
}

fn action_for_parts(path: &str, method: &str) -> Option<Action> {
    MOVE_INVOCATIONS
        .iter()
        .position(|candidate| candidate.0 == path && candidate.1 == method)
        .and_then(|index| MoveSlot::new(index).ok().map(Action::UseMove))
        .or_else(|| {
            SWITCH_INVOCATIONS
                .iter()
                .position(|candidate| candidate.0 == path && candidate.1 == method)
                .and_then(|index| TeamSlot::new(index).ok().map(Action::Switch))
        })
        .or_else(|| {
            (path == STRUGGLE_INVOCATION.0 && method == STRUGGLE_INVOCATION.1)
                .then_some(Action::Struggle)
        })
}

fn rejected(code: &str, message: &str) -> ProviderError {
    ProviderError::Rejected {
        code: code.into(),
        message: message.into(),
    }
}

fn invalid_provider_output() -> AdapterDiagnostic {
    AdapterDiagnostic {
        stage: DiagnosticStage::Runtime,
        code: "invalid-provider-output".into(),
        message: "battle provider returned an invalid action token".into(),
    }
}

fn configuration_diagnostic(error: impl std::fmt::Debug) -> AdapterDiagnostic {
    AdapterDiagnostic {
        stage: DiagnosticStage::Runtime,
        code: "invalid-adapter-configuration".into(),
        message: format!("{error:?}"),
    }
}

fn action_from_provider_output(outputs: &[Value]) -> Result<Action, AdapterDiagnostic> {
    let [Value::String(output)] = outputs else {
        return Err(invalid_provider_output());
    };
    let Some((path, method)) = output.split_once(' ') else {
        return Err(invalid_provider_output());
    };
    action_for_parts(path, method).ok_or_else(invalid_provider_output)
}

fn parse_diagnostic(failure: ParseFailure) -> AdapterDiagnostic {
    let Some(diagnostic) = failure.diagnostics().first() else {
        return AdapterDiagnostic {
            stage: DiagnosticStage::Parse,
            code: "parse-failure".into(),
            message: "shell text could not be parsed".into(),
        };
    };
    AdapterDiagnostic {
        stage: DiagnosticStage::Parse,
        code: parse_diagnostic_code(&diagnostic.kind).into(),
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

fn seal_diagnostic(diagnostic: Diagnostic) -> AdapterDiagnostic {
    AdapterDiagnostic {
        stage: DiagnosticStage::Seal,
        code: diagnostic.code.as_str().into(),
        message: diagnostic.message,
    }
}

fn execution_diagnostic(failure: ExecutionFailure) -> AdapterDiagnostic {
    match failure.error {
        ExecutionError::Provider(ProviderError::Rejected { code, message }) => AdapterDiagnostic {
            stage: DiagnosticStage::Provider,
            code,
            message,
        },
        error @ ExecutionError::CatalogChanged => AdapterDiagnostic {
            stage: DiagnosticStage::Runtime,
            code: "catalog-changed".into(),
            message: format!(
                "runtime execution failed at call {}: {error:?}",
                failure.call_index
            ),
        },
        error @ ExecutionError::SchemaChanged => AdapterDiagnostic {
            stage: DiagnosticStage::Runtime,
            code: "schema-changed".into(),
            message: format!(
                "runtime execution failed at call {}: {error:?}",
                failure.call_index
            ),
        },
        error @ ExecutionError::AuthorizationRevoked => AdapterDiagnostic {
            stage: DiagnosticStage::Runtime,
            code: "authorization-revoked".into(),
            message: format!(
                "runtime execution failed at call {}: {error:?}",
                failure.call_index
            ),
        },
        error @ ExecutionError::ProviderUnavailable => AdapterDiagnostic {
            stage: DiagnosticStage::Runtime,
            code: "provider-unavailable".into(),
            message: format!(
                "runtime execution failed at call {}: {error:?}",
                failure.call_index
            ),
        },
    }
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
