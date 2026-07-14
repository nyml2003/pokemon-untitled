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
    pub fn new() -> Self {
        let provider_id = ProviderId::new(PROVIDER_ID).expect("the fixed provider id is valid");
        let catalog = build_catalog(&provider_id);
        let authorization = AuthorizationService::new();
        let principal = authorization
            .create_principal(PLAYER_ID)
            .expect("the fixed local player principal is valid and unique");
        grant_player_actions(&authorization, &principal);

        let compiler = Compiler::new(Arc::clone(&catalog));
        let mut runtime = Runtime::new(catalog, authorization.checker());
        runtime
            .bind_provider(provider_id, Arc::new(BattleProvider))
            .expect("the battle provider is bound exactly once");

        Self {
            authorization,
            principal,
            compiler,
            runtime,
        }
    }

    pub fn action_invocations(&self, legal_actions: &[Action]) -> Vec<ActionInvocation> {
        let session = self
            .authorization
            .session(&self.principal)
            .expect("the local player belongs to this authority");
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
        invocations
    }

    pub fn execute_invocation(&self, invocation: &str) -> Result<Action, AdapterDiagnostic> {
        let document = parse_with_limits(invocation, PARSE_LIMITS).map_err(parse_diagnostic)?;
        let plan = {
            let session = self
                .authorization
                .session(&self.principal)
                .expect("the local player belongs to this authority");
            self.compiler
                .seal_with_limits(&session.view(), PlanDraft::from(document), COMPILE_LIMITS)
                .map_err(seal_diagnostic)?
        };
        let report = self.runtime.execute(plan).map_err(execution_diagnostic)?;
        action_from_provider_output(&report.outputs)
    }
}

impl Default for BattleRamusAdapter {
    fn default() -> Self {
        Self::new()
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

fn build_catalog(provider_id: &ProviderId) -> Arc<Catalog> {
    let mut catalog = Catalog::new();
    for (path, method) in all_invocations() {
        catalog
            .register(MethodRegistration {
                provider_id: provider_id.clone(),
                path: NodePath::parse(path).expect("fixed battle paths are valid"),
                schema: MethodSchema::new(
                    MethodName::new(method).expect("fixed battle methods are valid"),
                    vec![],
                )
                .expect("parameter-free battle schemas are valid"),
                schema_version: SchemaVersion::new(1).expect("schema version is non-zero"),
                effect: Effect::Invoke,
            })
            .expect("fixed battle catalog entries are unique");
    }
    Arc::new(catalog)
}

fn grant_player_actions(authorization: &AuthorizationService, principal: &Principal) {
    for (path, method) in all_invocations() {
        let path = NodePath::parse(path).expect("fixed battle paths are valid");
        let method = MethodName::new(method).expect("fixed battle methods are valid");
        for capability in [
            Capability::Discover,
            Capability::Complete,
            Capability::Invoke,
        ] {
            authorization
                .grant(principal, path.clone(), Some(method.clone()), capability)
                .expect("the local player belongs to this authority");
        }
    }
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
        .map(|index| Action::UseMove(MoveSlot::new(index).expect("fixed move slots are valid")))
        .or_else(|| {
            SWITCH_INVOCATIONS
                .iter()
                .position(|candidate| candidate.0 == path && candidate.1 == method)
                .map(|index| {
                    Action::Switch(TeamSlot::new(index).expect("fixed team slots are valid"))
                })
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
    let diagnostic = failure
        .diagnostics()
        .first()
        .expect("every parse failure contains at least one diagnostic");
    AdapterDiagnostic {
        stage: DiagnosticStage::Parse,
        code: parse_diagnostic_code(&diagnostic.kind).into(),
        message: diagnostic.to_string(),
    }
}

fn parse_diagnostic_code(kind: &ParseDiagnosticKind) -> &'static str {
    match kind {
        ParseDiagnosticKind::SourceTooLarge => "source-too-large",
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
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use battle_application::{Action, MoveSlot, TeamSlot};
    use ramus_core::{DiagnosticCode, ForbiddenSyntax};

    #[test]
    fn discovery_intersects_authorized_commands_with_current_legal_actions() {
        let adapter = BattleRamusAdapter::new();
        let legal = [
            Action::UseMove(MoveSlot::new(1).unwrap()),
            Action::Switch(TeamSlot::new(4).unwrap()),
        ];

        let invocations = adapter.action_invocations(&legal);

        assert_eq!(invocations.len(), 2);
        assert_eq!(invocations[0].action, legal[0]);
        assert_eq!(invocations[0].invocation, "/battle/move/two use");
        assert_eq!(invocations[1].action, legal[1]);
        assert_eq!(invocations[1].invocation, "/battle/team/five switch");
    }

    #[test]
    fn executing_an_authorized_invocation_returns_exactly_one_action() {
        let adapter = BattleRamusAdapter::default();
        assert_eq!(
            adapter.execute_invocation("/battle/move/three use"),
            Ok(Action::UseMove(MoveSlot::new(2).unwrap()))
        );
    }

    #[test]
    fn malformed_and_unknown_invocations_are_diagnostics() {
        let adapter = BattleRamusAdapter::new();

        let malformed = adapter.execute_invocation("").unwrap_err();
        let unknown = adapter
            .execute_invocation("/battle/debug inspect")
            .unwrap_err();

        assert_eq!(malformed.stage, DiagnosticStage::Parse);
        assert_eq!(unknown.stage, DiagnosticStage::Seal);
    }

    fn request(path: &str, method: &str) -> ProviderRequest {
        ProviderRequest {
            path: NodePath::parse(path).unwrap(),
            method: MethodName::new(method).unwrap(),
            arguments: BTreeMap::new(),
        }
    }

    fn rejection_code(result: Result<(), ProviderError>) -> String {
        let ProviderError::Rejected { code, .. } = result.unwrap_err();
        code
    }

    #[test]
    fn provider_validation_rejects_every_mismatched_request_field() {
        let valid = request("/battle/move/one", "use");
        assert!(
            validate_provider_request(
                PLAYER_ID,
                Capability::Invoke,
                &valid.path,
                &valid.method,
                &valid,
            )
            .is_ok()
        );

        let other_path = NodePath::parse("/battle/move/two").unwrap();
        let other_method = MethodName::new("switch").unwrap();
        for result in [
            validate_provider_request(
                "other",
                Capability::Invoke,
                &valid.path,
                &valid.method,
                &valid,
            ),
            validate_provider_request(
                PLAYER_ID,
                Capability::Discover,
                &valid.path,
                &valid.method,
                &valid,
            ),
            validate_provider_request(
                PLAYER_ID,
                Capability::Invoke,
                &other_path,
                &valid.method,
                &valid,
            ),
            validate_provider_request(
                PLAYER_ID,
                Capability::Invoke,
                &valid.path,
                &other_method,
                &valid,
            ),
        ] {
            assert_eq!(rejection_code(result), "invalid-permit");
        }

        let mut with_arguments = valid.clone();
        with_arguments
            .arguments
            .insert("unexpected".into(), Value::String("value".into()));
        assert_eq!(
            rejection_code(validate_provider_request(
                PLAYER_ID,
                Capability::Invoke,
                &with_arguments.path,
                &with_arguments.method,
                &with_arguments,
            )),
            "unexpected-arguments"
        );

        let unknown = request("/battle/debug", "inspect");
        assert_eq!(
            rejection_code(validate_provider_request(
                PLAYER_ID,
                Capability::Invoke,
                &unknown.path,
                &unknown.method,
                &unknown,
            )),
            "unknown-action"
        );
    }

    #[test]
    fn provider_outputs_and_diagnostic_mappings_are_total() {
        assert_eq!(
            action_from_provider_output(&[Value::String("/battle/action struggle".into())]),
            Ok(Action::Struggle)
        );
        for outputs in [
            vec![],
            vec![Value::String("no-space".into())],
            vec![Value::String("/battle/unknown invoke".into())],
            vec![Value::String("one".into()), Value::String("two".into())],
        ] {
            assert_eq!(
                action_from_provider_output(&outputs).unwrap_err().code,
                "invalid-provider-output"
            );
        }

        let parse_kinds = [
            ParseDiagnosticKind::SourceTooLarge,
            ParseDiagnosticKind::TooManyCalls,
            ParseDiagnosticKind::TooManyArguments,
            ParseDiagnosticKind::EmptyInput,
            ParseDiagnosticKind::EmptyStatement,
            ParseDiagnosticKind::ExpectedNodePath,
            ParseDiagnosticKind::InvalidNodePath { value: "x".into() },
            ParseDiagnosticKind::ExpectedMethod,
            ParseDiagnosticKind::InvalidMethodName { value: "x".into() },
            ParseDiagnosticKind::ExpectedArgument,
            ParseDiagnosticKind::InvalidParameterName { value: "x".into() },
            ParseDiagnosticKind::MissingArgumentValue,
            ParseDiagnosticKind::WhitespaceAroundEquals,
            ParseDiagnosticKind::MissingWhitespace,
            ParseDiagnosticKind::UnterminatedString,
            ParseDiagnosticKind::InvalidEscape { escape: 'x' },
            ParseDiagnosticKind::IntegerOutOfRange { value: "x".into() },
            ParseDiagnosticKind::ForbiddenSyntax(ForbiddenSyntax::Pipe),
            ParseDiagnosticKind::UnexpectedCharacter { character: 'x' },
        ];
        for kind in parse_kinds {
            assert!(!parse_diagnostic_code(&kind).is_empty());
        }

        let sealed = seal_diagnostic(Diagnostic {
            code: DiagnosticCode::EmptyPlan,
            message: "empty".into(),
            parameter: None,
        });
        assert_eq!(sealed.stage, DiagnosticStage::Seal);
        assert_eq!(sealed.code, "empty-plan");
    }

    #[test]
    fn every_runtime_failure_has_a_stable_adapter_stage_and_code() {
        let failures = [
            (
                ExecutionError::Provider(ProviderError::Rejected {
                    code: "denied".into(),
                    message: "no".into(),
                }),
                DiagnosticStage::Provider,
                "denied",
            ),
            (
                ExecutionError::CatalogChanged,
                DiagnosticStage::Runtime,
                "catalog-changed",
            ),
            (
                ExecutionError::SchemaChanged,
                DiagnosticStage::Runtime,
                "schema-changed",
            ),
            (
                ExecutionError::AuthorizationRevoked,
                DiagnosticStage::Runtime,
                "authorization-revoked",
            ),
            (
                ExecutionError::ProviderUnavailable,
                DiagnosticStage::Runtime,
                "provider-unavailable",
            ),
        ];
        for (error, stage, code) in failures {
            let diagnostic = execution_diagnostic(ExecutionFailure {
                call_index: 2,
                completed_outputs: Vec::new(),
                error,
            });
            assert_eq!(diagnostic.stage, stage);
            assert_eq!(diagnostic.code, code);
            assert!(!diagnostic.message.is_empty());
        }
    }
}
