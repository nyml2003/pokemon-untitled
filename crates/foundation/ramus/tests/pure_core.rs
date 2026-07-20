use std::collections::BTreeMap;
use std::sync::Arc;

use ramus_core::{
    AuthorizationService, Capability, Catalog, CatalogError, CompileLimits, Compiler,
    DiagnosticCode, DraftArgument, DraftCall, Effect, MethodName, MethodRegistration, MethodSchema,
    ModelError, NodePath, ParameterName, ParameterSchema, PlanDraft, ProviderId, SchemaError,
    SchemaVersion, Span, Value, ValueType,
};

fn path(value: &str) -> NodePath {
    NodePath::parse(value).unwrap()
}

fn method(value: &str) -> MethodName {
    MethodName::new(value).unwrap()
}

fn parameter(
    name: &str,
    value_type: ValueType,
    required: bool,
    positional: bool,
) -> ParameterSchema {
    ParameterSchema {
        name: ParameterName::new(name).unwrap(),
        value_type,
        required,
        positional,
    }
}

fn registration(
    node_path: &str,
    method_name: &str,
    effect: Effect,
    parameters: Vec<ParameterSchema>,
) -> MethodRegistration {
    MethodRegistration {
        provider_id: ProviderId::new("battle").unwrap(),
        path: path(node_path),
        schema: MethodSchema::new(method(method_name), parameters).unwrap(),
        schema_version: SchemaVersion::new(1).unwrap(),
        effect,
    }
}

fn compiler_with(
    registration: MethodRegistration,
    capabilities: &[Capability],
) -> (Compiler, AuthorizationService, ramus_core::Principal) {
    let mut catalog = Catalog::new();
    let node_path = registration.path.clone();
    let method_name = registration.schema.name().clone();
    catalog.register(registration).unwrap();
    let authorization = AuthorizationService::new();
    let principal = authorization.create_principal("agent").unwrap();
    for capability in capabilities {
        authorization
            .grant(
                &principal,
                node_path.clone(),
                Some(method_name.clone()),
                *capability,
            )
            .unwrap();
    }
    (Compiler::new(Arc::new(catalog)), authorization, principal)
}

fn draft(arguments: Vec<DraftArgument>) -> PlanDraft {
    PlanDraft {
        calls: vec![DraftCall {
            path: "/battle/turn".into(),
            method: "submit".into(),
            arguments,
        }],
    }
}

fn named(name: &str, value: Value) -> DraftArgument {
    DraftArgument {
        name: Some(name.into()),
        value,
    }
}

fn positional(value: Value) -> DraftArgument {
    DraftArgument { name: None, value }
}

#[test]
fn value_objects_reject_invalid_identifiers_and_paths() {
    for invalid in ["", "1name", "has space", "bad/slash"] {
        assert!(MethodName::new(invalid).is_err(), "{invalid}");
        assert!(ParameterName::new(invalid).is_err(), "{invalid}");
        assert!(ProviderId::new(invalid).is_err(), "{invalid}");
    }

    for invalid in ["relative", "/trailing/", "/double//slash", "/bad segment"] {
        assert!(NodePath::parse(invalid).is_err(), "{invalid}");
    }
    assert_eq!(NodePath::parse("/").unwrap().as_str(), "/");
    assert_eq!(method("valid-name.v1").as_str(), "valid-name.v1");
    assert_eq!(method("_valid_1").as_str(), "_valid_1");
    assert_eq!(ProviderId::new("battle").unwrap().as_str(), "battle");
    assert_eq!(SchemaVersion::new(0), None);
}

#[test]
fn model_errors_have_stable_messages() {
    assert_eq!(
        ModelError::InvalidIdentifier("1bad".into()).to_string(),
        "invalid identifier: 1bad"
    );
    assert_eq!(
        ModelError::InvalidPath("relative".into()).to_string(),
        "invalid node path: relative"
    );
}

#[test]
fn effect_capabilities_map_one_to_one() {
    assert_eq!(Capability::from(Effect::Read), Capability::Read);
    assert_eq!(Capability::from(Effect::Write), Capability::Write);
    assert_eq!(Capability::from(Effect::Invoke), Capability::Invoke);
}

#[test]
fn schema_rejects_duplicate_parameters() {
    let duplicate = parameter("value", ValueType::String, true, true);
    assert_eq!(
        MethodSchema::new(method("set"), vec![duplicate.clone(), duplicate]),
        Err(SchemaError::DuplicateParameter("value".into()))
    );
}

#[test]
fn schema_rejects_required_positional_after_optional() {
    assert_eq!(
        MethodSchema::new(
            method("set"),
            vec![
                parameter("optional", ValueType::String, false, true),
                parameter("required", ValueType::String, true, true),
            ],
        ),
        Err(SchemaError::RequiredAfterOptional("required".into()))
    );
}

#[test]
fn schema_allows_required_named_after_optional_positional() {
    assert!(
        MethodSchema::new(
            method("set"),
            vec![
                parameter("optional", ValueType::String, false, true),
                parameter("required_named", ValueType::String, true, false),
            ],
        )
        .is_ok()
    );
}

#[test]
fn schema_allows_multiple_optional_positionals() {
    assert!(
        MethodSchema::new(
            method("set"),
            vec![
                parameter("first", ValueType::String, false, true),
                parameter("second", ValueType::String, false, true),
            ],
        )
        .is_ok()
    );
}

#[test]
fn catalog_default_empty_and_duplicate_behaviors_are_explicit() {
    let mut catalog = Catalog::default();
    assert!(catalog.is_empty());
    let item = registration("/battle/turn", "submit", Effect::Invoke, vec![]);
    catalog.register(item.clone()).unwrap();
    assert!(!catalog.is_empty());
    assert_eq!(catalog.register(item), Err(CatalogError::DuplicateMethod));
}

#[test]
fn compiler_completion_requires_complete_and_honors_prefix() {
    let item = registration(
        "/battle/turn",
        "submit",
        Effect::Invoke,
        vec![parameter("move", ValueType::String, true, true)],
    );
    let (compiler, authorization, principal) = compiler_with(
        item,
        &[
            Capability::Discover,
            Capability::Complete,
            Capability::Invoke,
        ],
    );
    let session = authorization.session(&principal).unwrap();
    let view = session.view();
    assert_eq!(compiler.complete(&view, "/missing"), vec![]);
    let completions = compiler.complete(&view, "/battle");
    assert_eq!(completions[0].invocation, "/battle/turn submit");
    assert_eq!(completions[0].parameters, vec!["move"]);
}

#[test]
fn compiler_validates_named_and_positional_arguments() {
    let item = registration(
        "/battle/turn",
        "submit",
        Effect::Invoke,
        vec![
            parameter("move", ValueType::Enum(vec!["surf".into()]), true, true),
            parameter("target", ValueType::String, true, true),
            parameter("priority", ValueType::Integer, false, false),
            parameter("force", ValueType::Boolean, false, false),
        ],
    );
    let (compiler, authorization, principal) =
        compiler_with(item, &[Capability::Discover, Capability::Invoke]);
    let session = authorization.session(&principal).unwrap();
    let plan = compiler
        .seal(
            &session.view(),
            draft(vec![
                positional(Value::String("surf".into())),
                positional(Value::String("opponent:1".into())),
                named("priority", Value::Integer(1)),
                named("force", Value::Boolean(true)),
            ]),
        )
        .unwrap();
    assert_eq!(plan.principal().as_str(), "agent");
    assert_eq!(plan.len(), 1);
    assert!(!plan.is_empty());
}

#[test]
fn compiler_accepts_structured_values_when_the_schema_requires_them() {
    let item = registration(
        "/battle/turn",
        "submit",
        Effect::Invoke,
        vec![
            parameter("payload", ValueType::Record, true, false),
            parameter("selection", ValueType::List, true, false),
        ],
    );
    let (compiler, authorization, principal) =
        compiler_with(item, &[Capability::Discover, Capability::Invoke]);
    let session = authorization.session(&principal).unwrap();
    let plan = compiler.seal(
        &session.view(),
        draft(vec![
            named(
                "payload",
                Value::Record(BTreeMap::from([(
                    String::from("name"),
                    Value::String(String::from("route-rival")),
                )])),
            ),
            named(
                "selection",
                Value::List(vec![Value::Integer(1), Value::Integer(2)]),
            ),
        ]),
    );
    assert!(plan.is_ok());
}

#[test]
fn compiler_reports_each_argument_contract_failure() {
    let item = registration(
        "/battle/turn",
        "submit",
        Effect::Invoke,
        vec![parameter("move", ValueType::String, true, true)],
    );
    let (compiler, authorization, principal) =
        compiler_with(item, &[Capability::Discover, Capability::Invoke]);
    let cases = [
        (
            vec![named("unknown", Value::String("x".into()))],
            DiagnosticCode::UnknownParameter,
        ),
        (
            vec![
                positional(Value::String("x".into())),
                positional(Value::String("y".into())),
            ],
            DiagnosticCode::TooManyPositionals,
        ),
        (
            vec![
                named("move", Value::String("x".into())),
                named("move", Value::String("y".into())),
            ],
            DiagnosticCode::DuplicateParameter,
        ),
        (vec![], DiagnosticCode::MissingParameter),
        (
            vec![named("move", Value::Boolean(true))],
            DiagnosticCode::InvalidValue,
        ),
    ];

    let session = authorization.session(&principal).unwrap();
    for (arguments, expected) in cases {
        let error = match compiler.seal(&session.view(), draft(arguments)) {
            Ok(_) => panic!("invalid arguments must not seal"),
            Err(error) => error,
        };
        assert_eq!(error.code, expected);
        assert_eq!(error.code.as_str(), expected.as_str());
    }
}

#[test]
fn compiler_limits_reject_untrusted_agent_drafts() {
    let item = registration("/battle/turn", "submit", Effect::Invoke, vec![]);
    let (compiler, authorization, principal) =
        compiler_with(item, &[Capability::Discover, Capability::Invoke]);
    let call = DraftCall {
        path: "/battle/turn".into(),
        method: "submit".into(),
        arguments: vec![],
    };
    let session = authorization.session(&principal).unwrap();
    let too_many_calls = compiler
        .seal_with_limits(
            &session.view(),
            PlanDraft {
                calls: vec![call.clone(), call.clone()],
            },
            CompileLimits {
                max_calls: 1,
                max_arguments_per_call: 1,
                ..CompileLimits::default()
            },
        )
        .err()
        .unwrap();
    assert_eq!(too_many_calls.code, DiagnosticCode::TooManyCalls);
    assert_eq!(too_many_calls.code.as_str(), "too-many-calls");

    let too_many_arguments = compiler
        .seal_with_limits(
            &session.view(),
            PlanDraft {
                calls: vec![DraftCall {
                    arguments: vec![
                        positional(Value::String("one".into())),
                        positional(Value::String("two".into())),
                    ],
                    ..call
                }],
            },
            CompileLimits {
                max_calls: 1,
                max_arguments_per_call: 1,
                ..CompileLimits::default()
            },
        )
        .err()
        .unwrap();
    assert_eq!(too_many_arguments.code, DiagnosticCode::TooManyArguments);
    assert_eq!(too_many_arguments.code.as_str(), "too-many-arguments");

    let string_parameter = parameter("value", ValueType::String, true, true);
    let item = registration(
        "/battle/turn",
        "submit",
        Effect::Invoke,
        vec![string_parameter],
    );
    let (compiler, authorization, principal) =
        compiler_with(item, &[Capability::Discover, Capability::Invoke]);
    let session = authorization.session(&principal).unwrap();
    let too_large = compiler
        .seal_with_limits(
            &session.view(),
            draft(vec![positional(Value::String("oversized".into()))]),
            CompileLimits {
                max_total_bytes: 8,
                ..CompileLimits::default()
            },
        )
        .err()
        .unwrap();
    assert_eq!(too_large.code, DiagnosticCode::InputTooLarge);

    let too_deep = compiler
        .seal_with_limits(
            &session.view(),
            draft(vec![positional(Value::List(vec![Value::List(vec![
                Value::String("leaf".into()),
            ])]))]),
            CompileLimits {
                max_value_depth: 2,
                ..CompileLimits::default()
            },
        )
        .err()
        .unwrap();
    assert_eq!(too_deep.code, DiagnosticCode::ValueTooDeep);

    let value_too_large = compiler
        .seal_with_limits(
            &session.view(),
            draft(vec![positional(Value::String("oversized".into()))]),
            CompileLimits {
                max_value_bytes: 4,
                ..CompileLimits::default()
            },
        )
        .err()
        .unwrap();
    assert_eq!(value_too_large.code, DiagnosticCode::ValueTooLarge);

    let too_many_nodes = compiler
        .seal_with_limits(
            &session.view(),
            draft(vec![positional(Value::List(vec![
                Value::Unit,
                Value::Unit,
            ]))]),
            CompileLimits {
                max_value_nodes: 2,
                ..CompileLimits::default()
            },
        )
        .err()
        .unwrap();
    assert_eq!(too_many_nodes.code, DiagnosticCode::TooManyValueNodes);

    let record_is_measured_before_schema_validation = compiler
        .seal(
            &session.view(),
            draft(vec![positional(Value::Record(BTreeMap::from([(
                "field".into(),
                Value::Unit,
            )])))]),
        )
        .err()
        .unwrap();
    assert_eq!(
        record_is_measured_before_schema_validation.code,
        DiagnosticCode::InvalidValue
    );

    let nested_record_is_bounded = compiler
        .seal_with_limits(
            &session.view(),
            draft(vec![positional(Value::Record(BTreeMap::from([(
                "field".into(),
                Value::Unit,
            )])))]),
            CompileLimits {
                max_value_depth: 1,
                ..CompileLimits::default()
            },
        )
        .err()
        .unwrap();
    assert_eq!(nested_record_is_bounded.code, DiagnosticCode::ValueTooDeep);

    for (code, label) in [
        (DiagnosticCode::InputTooLarge, "input-too-large"),
        (DiagnosticCode::ValueTooLarge, "value-too-large"),
        (DiagnosticCode::TooManyValueNodes, "too-many-value-nodes"),
        (DiagnosticCode::ValueTooDeep, "value-too-deep"),
    ] {
        assert_eq!(code.as_str(), label);
    }
}

#[test]
fn rejecting_an_extremely_deep_value_does_not_overflow_during_drop() {
    let item = registration(
        "/battle/turn",
        "submit",
        Effect::Invoke,
        vec![parameter("value", ValueType::String, true, true)],
    );
    let (compiler, authorization, principal) =
        compiler_with(item, &[Capability::Discover, Capability::Invoke]);
    let mut value = Value::Unit;
    for depth in 0..50_000 {
        value = if depth % 2 == 0 {
            Value::List(vec![value])
        } else {
            Value::Record(BTreeMap::from([("child".into(), value)]))
        };
    }

    let error = compiler
        .seal(
            &authorization.session(&principal).unwrap().view(),
            draft(vec![positional(value)]),
        )
        .err()
        .unwrap();

    assert_eq!(error.code, DiagnosticCode::ValueTooDeep);
}

#[test]
fn hidden_path_method_and_effect_fail_with_one_diagnostic() {
    let item = registration("/battle/turn", "submit", Effect::Invoke, vec![]);
    let (compiler, authorization, principal) = compiler_with(item, &[Capability::Discover]);
    for (node_path, method_name) in [
        ("relative", "submit"),
        ("/battle/turn", "1bad"),
        ("/missing", "submit"),
        ("/battle/turn", "submit"),
    ] {
        let session = authorization.session(&principal).unwrap();
        let error = match compiler.seal(
            &session.view(),
            PlanDraft {
                calls: vec![DraftCall {
                    path: node_path.into(),
                    method: method_name.into(),
                    arguments: vec![],
                }],
            },
        ) {
            Ok(_) => panic!("unavailable operation must not seal"),
            Err(error) => error,
        };
        assert_eq!(error.code, DiagnosticCode::OperationUnavailable);
        assert_eq!(error.code.as_str(), "operation-unavailable");
        assert_eq!(error.message, "operation is unavailable");
    }
}

#[test]
fn spans_report_empty_and_non_empty_ranges() {
    assert!(Span::new(1, 1).is_empty());
    assert!(!Span::new(1, 2).is_empty());
}

#[test]
fn root_and_method_scoped_capabilities_match_only_their_scope() {
    let mut catalog = Catalog::new();
    catalog
        .register(registration(
            "/battle/turn",
            "submit",
            Effect::Invoke,
            vec![],
        ))
        .unwrap();
    catalog
        .register(registration("/battle/state", "get", Effect::Read, vec![]))
        .unwrap();
    let authorization = AuthorizationService::new();
    let principal = authorization.create_principal("agent").unwrap();
    authorization
        .grant(&principal, path("/"), None, Capability::Discover)
        .unwrap();
    authorization
        .grant(
            &principal,
            path("/battle/turn"),
            Some(method("other")),
            Capability::Invoke,
        )
        .unwrap();
    let compiler = Compiler::new(Arc::new(catalog));
    let session = authorization.session(&principal).unwrap();
    let view = session.view();
    let discovered = compiler.discover(&view).len();
    let error = match compiler.seal(&view, draft(vec![])) {
        Ok(_) => panic!("wrong method grant must not authorize invocation"),
        Err(error) => error,
    };
    assert_eq!(discovered, 2);
    assert_eq!(error.code, DiagnosticCode::OperationUnavailable);
}

#[test]
fn path_mismatch_does_not_consume_a_matching_capability() {
    let mut catalog = Catalog::new();
    catalog
        .register(registration(
            "/battle/turn",
            "submit",
            Effect::Invoke,
            vec![],
        ))
        .unwrap();
    let authorization = AuthorizationService::new();
    let principal = authorization.create_principal("agent").unwrap();
    authorization
        .grant(
            &principal,
            path("/other"),
            Some(method("submit")),
            Capability::Discover,
        )
        .unwrap();
    let compiler = Compiler::new(Arc::new(catalog));
    assert!(
        compiler
            .discover(&authorization.session(&principal).unwrap().view())
            .is_empty()
    );
}
