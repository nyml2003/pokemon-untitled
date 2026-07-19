use std::collections::BTreeMap;

use super::*;
use battle_application::{Action, MoveSlot, TeamSlot};
use ramus_core::{DiagnosticCode, ForbiddenSyntax};

#[test]
fn discovery_intersects_authorized_commands_with_current_legal_actions() {
    let adapter = BattleRamusAdapter::new().unwrap();
    let legal = [
        Action::UseMove(MoveSlot::new(1).unwrap()),
        Action::Switch(TeamSlot::new(4).unwrap()),
    ];

    let invocations = adapter.action_invocations(&legal).unwrap();

    assert_eq!(invocations.len(), 2);
    assert_eq!(invocations[0].action, legal[0]);
    assert_eq!(invocations[0].invocation, "/battle/move/two use");
    assert_eq!(invocations[1].action, legal[1]);
    assert_eq!(invocations[1].invocation, "/battle/team/five switch");
}

#[test]
fn executing_an_authorized_invocation_returns_exactly_one_action() {
    let adapter = BattleRamusAdapter::new().unwrap();
    assert_eq!(
        adapter.execute_invocation("/battle/move/three use"),
        Ok(Action::UseMove(MoveSlot::new(2).unwrap()))
    );
}

#[test]
fn malformed_and_unknown_invocations_are_diagnostics() {
    let adapter = BattleRamusAdapter::new().unwrap();

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
