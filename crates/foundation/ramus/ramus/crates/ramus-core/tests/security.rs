use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use ramus_core::{
    AuthorizationRevoker, AuthorizationService, Capability, Catalog, Compiler, DiagnosticCode,
    DraftArgument, DraftCall, Effect, ExecutionError, MethodName, MethodRegistration, MethodSchema,
    NodePath, ParameterName, ParameterSchema, PlanDraft, Principal, Provider, ProviderError,
    ProviderId, ProviderRequest, Runtime, SchemaVersion, Value, ValueType,
};

struct RecordingProvider {
    calls: AtomicUsize,
    outputs: Mutex<Vec<Value>>,
    revoke_after_call: Option<(AuthorizationRevoker, Principal)>,
}

impl RecordingProvider {
    fn fixed(output: Value) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            outputs: Mutex::new(vec![output]),
            revoke_after_call: None,
        }
    }

    fn sequence(
        outputs: Vec<Value>,
        revoke_after_call: Option<(AuthorizationRevoker, Principal)>,
    ) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            outputs: Mutex::new(outputs),
            revoke_after_call,
        }
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Provider for RecordingProvider {
    fn execute(
        &self,
        _permit: ramus_core::EffectPermit,
        _request: &ProviderRequest,
    ) -> Result<Value, ProviderError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        let output = self
            .outputs
            .lock()
            .unwrap()
            .get(call)
            .cloned()
            .unwrap_or(Value::Unit);
        if call == 0
            && let Some((authorization, principal)) = &self.revoke_after_call
        {
            authorization.revoke_all(principal).unwrap();
        }
        Ok(output)
    }
}

fn path(value: &str) -> NodePath {
    NodePath::parse(value).unwrap()
}

fn method(value: &str) -> MethodName {
    MethodName::new(value).unwrap()
}

fn principal(authorization: &AuthorizationService, value: &str) -> Principal {
    authorization.create_principal(value).unwrap()
}

fn schema(name: &str) -> MethodSchema {
    MethodSchema::new(
        method(name),
        vec![
            ParameterSchema {
                name: ParameterName::new("move").unwrap(),
                value_type: ValueType::Enum(vec!["thunderbolt".into(), "surf".into()]),
                required: true,
                positional: true,
            },
            ParameterSchema {
                name: ParameterName::new("target").unwrap(),
                value_type: ValueType::String,
                required: true,
                positional: true,
            },
        ],
    )
    .unwrap()
}

fn register(catalog: &mut Catalog, node_path: &str, method_name: &str, effect: Effect) {
    catalog
        .register(MethodRegistration {
            provider_id: ProviderId::new("battle").unwrap(),
            path: path(node_path),
            schema: schema(method_name),
            schema_version: SchemaVersion::new(1).unwrap(),
            effect,
        })
        .unwrap();
}

fn grant_method(
    authorization: &AuthorizationService,
    principal: &Principal,
    node_path: &str,
    method_name: &str,
    capabilities: &[Capability],
) {
    for capability in capabilities {
        authorization
            .grant(
                principal,
                path(node_path),
                Some(method(method_name)),
                *capability,
            )
            .unwrap();
    }
}

fn call(node_path: &str, method_name: &str) -> DraftCall {
    DraftCall {
        path: node_path.into(),
        method: method_name.into(),
        arguments: vec![
            DraftArgument {
                name: Some("move".into()),
                value: Value::String("thunderbolt".into()),
            },
            DraftArgument {
                name: Some("target".into()),
                value: Value::String("opponent:1".into()),
            },
        ],
    }
}

#[test]
fn typed_command_executes_through_a_single_use_permit() {
    let authorization = AuthorizationService::new();
    let agent = principal(&authorization, "agent");
    grant_method(
        &authorization,
        &agent,
        "/battle/turn",
        "submit",
        &[
            Capability::Discover,
            Capability::Complete,
            Capability::Invoke,
        ],
    );
    let provider = Arc::new(RecordingProvider::fixed(Value::Record(BTreeMap::from([(
        "accepted".into(),
        Value::Boolean(true),
    )]))));
    let mut catalog = Catalog::new();
    register(&mut catalog, "/battle/turn", "submit", Effect::Invoke);
    let catalog = Arc::new(catalog);
    let compiler = Compiler::new(Arc::clone(&catalog));
    let plan = {
        let session = authorization.session(&agent).unwrap();
        compiler
            .seal(
                &session.view(),
                PlanDraft {
                    calls: vec![call("/battle/turn", "submit")],
                },
            )
            .unwrap()
    };
    let mut runtime = Runtime::new(catalog, authorization.checker());
    runtime
        .bind_provider(ProviderId::new("battle").unwrap(), provider.clone())
        .unwrap();

    let report = runtime.execute(plan).unwrap();

    assert_eq!(provider.call_count(), 1);
    assert_eq!(
        report.outputs,
        vec![Value::Record(BTreeMap::from([(
            "accepted".into(),
            Value::Boolean(true)
        )]))]
    );
}

#[test]
fn unauthorized_and_missing_operations_are_indistinguishable() {
    let authorization = AuthorizationService::new();
    let agent = principal(&authorization, "agent");
    let developer = principal(&authorization, "developer");
    let mut catalog = Catalog::new();
    register(&mut catalog, "/dev/battle", "seed", Effect::Write);
    grant_method(
        &authorization,
        &developer,
        "/dev/battle",
        "seed",
        &[Capability::Discover, Capability::Write],
    );
    let compiler = Compiler::new(Arc::new(catalog));
    let (hidden, missing) = {
        let session = authorization.session(&agent).unwrap();
        let view = session.view();
        assert!(compiler.discover(&view).is_empty());
        assert!(compiler.complete(&view, "/dev").is_empty());
        let hidden = compiler
            .seal(
                &view,
                PlanDraft {
                    calls: vec![call("/dev/battle", "seed")],
                },
            )
            .err()
            .unwrap();
        let missing = compiler
            .seal(
                &view,
                PlanDraft {
                    calls: vec![call("/missing", "seed")],
                },
            )
            .err()
            .unwrap();
        (hidden, missing)
    };
    assert_eq!(
        compiler
            .discover(&authorization.session(&developer).unwrap().view())
            .len(),
        1
    );
    assert_eq!(hidden, missing);
    assert_eq!(hidden.code, DiagnosticCode::OperationUnavailable);
}

#[test]
fn revocation_removes_operations_from_each_new_authorization_view() {
    let authorization = AuthorizationService::new();
    let agent = principal(&authorization, "agent");
    grant_method(
        &authorization,
        &agent,
        "/battle/turn",
        "submit",
        &[
            Capability::Discover,
            Capability::Complete,
            Capability::Invoke,
        ],
    );
    let mut catalog = Catalog::new();
    register(&mut catalog, "/battle/turn", "submit", Effect::Invoke);
    let compiler = Compiler::new(Arc::new(catalog));
    authorization.revoke_all(&agent).unwrap();
    let error = {
        let session = authorization.session(&agent).unwrap();
        let view = session.view();
        assert!(compiler.discover(&view).is_empty());
        assert!(compiler.complete(&view, "/battle").is_empty());
        compiler
            .seal(
                &view,
                PlanDraft {
                    calls: vec![call("/battle/turn", "submit")],
                },
            )
            .err()
            .unwrap()
    };
    assert_eq!(error.code, DiagnosticCode::OperationUnavailable);
}

#[test]
fn malformed_arguments_never_reach_the_provider() {
    let authorization = AuthorizationService::new();
    let agent = principal(&authorization, "agent");
    grant_method(
        &authorization,
        &agent,
        "/battle/turn",
        "submit",
        &[Capability::Discover, Capability::Invoke],
    );
    let provider = Arc::new(RecordingProvider::fixed(Value::Unit));
    let mut catalog = Catalog::new();
    register(&mut catalog, "/battle/turn", "submit", Effect::Invoke);
    let compiler = Compiler::new(Arc::new(catalog));
    let mut malformed = call("/battle/turn", "submit");
    malformed.arguments[0].value = Value::String("not-a-move".into());

    let error = match compiler.seal(
        &authorization.session(&agent).unwrap().view(),
        PlanDraft {
            calls: vec![malformed],
        },
    ) {
        Ok(_) => panic!("malformed arguments must not seal"),
        Err(error) => error,
    };
    assert_eq!(error.code, DiagnosticCode::InvalidValue);
    assert_eq!(provider.call_count(), 0);
}

#[test]
fn revocation_after_seal_prevents_the_first_effect() {
    let authorization = AuthorizationService::new();
    let agent = principal(&authorization, "agent");
    grant_method(
        &authorization,
        &agent,
        "/battle/turn",
        "submit",
        &[Capability::Discover, Capability::Invoke],
    );
    let provider = Arc::new(RecordingProvider::fixed(Value::Unit));
    let mut catalog = Catalog::new();
    register(&mut catalog, "/battle/turn", "submit", Effect::Invoke);
    let catalog = Arc::new(catalog);
    let compiler = Compiler::new(Arc::clone(&catalog));
    let plan = {
        let session = authorization.session(&agent).unwrap();
        compiler
            .seal(
                &session.view(),
                PlanDraft {
                    calls: vec![call("/battle/turn", "submit")],
                },
            )
            .unwrap()
    };
    let mut runtime = Runtime::new(catalog, authorization.checker());
    runtime
        .bind_provider(ProviderId::new("battle").unwrap(), provider.clone())
        .unwrap();
    authorization.revoke_all(&agent).unwrap();
    let failure = runtime.execute(plan).unwrap_err();
    assert_eq!(failure.error, ExecutionError::AuthorizationRevoked);
    assert_eq!(provider.call_count(), 0);
}

#[test]
fn revocation_between_calls_stops_without_rolling_back_completed_output() {
    let authorization = AuthorizationService::new();
    let agent = principal(&authorization, "agent");
    grant_method(
        &authorization,
        &agent,
        "/battle/turn",
        "submit",
        &[Capability::Discover, Capability::Invoke],
    );
    let provider = Arc::new(RecordingProvider::sequence(
        vec![
            Value::String("first".into()),
            Value::String("second".into()),
        ],
        Some((authorization.revoker(), agent.clone())),
    ));
    let mut catalog = Catalog::new();
    register(&mut catalog, "/battle/turn", "submit", Effect::Invoke);
    let catalog = Arc::new(catalog);
    let compiler = Compiler::new(Arc::clone(&catalog));
    let plan = {
        let session = authorization.session(&agent).unwrap();
        compiler
            .seal(
                &session.view(),
                PlanDraft {
                    calls: vec![
                        call("/battle/turn", "submit"),
                        call("/battle/turn", "submit"),
                    ],
                },
            )
            .unwrap()
    };
    let mut runtime = Runtime::new(catalog, authorization.checker());
    runtime
        .bind_provider(ProviderId::new("battle").unwrap(), provider.clone())
        .unwrap();
    let failure = runtime.execute(plan).unwrap_err();
    assert_eq!(failure.call_index, 1);
    assert_eq!(
        failure.completed_outputs,
        vec![Value::String("first".into())]
    );
    assert_eq!(failure.error, ExecutionError::AuthorizationRevoked);
    assert_eq!(provider.call_count(), 1);
}

#[test]
fn a_registry_change_invalidates_an_old_plan() {
    let authorization = AuthorizationService::new();
    let agent = principal(&authorization, "agent");
    grant_method(
        &authorization,
        &agent,
        "/battle/turn",
        "submit",
        &[Capability::Discover, Capability::Invoke],
    );
    let provider = Arc::new(RecordingProvider::fixed(Value::Unit));
    let mut catalog = Catalog::new();
    register(&mut catalog, "/battle/turn", "submit", Effect::Invoke);
    let compile_catalog = Arc::new(catalog.clone());
    let compiler = Compiler::new(compile_catalog);
    let plan = {
        let session = authorization.session(&agent).unwrap();
        compiler
            .seal(
                &session.view(),
                PlanDraft {
                    calls: vec![call("/battle/turn", "submit")],
                },
            )
            .unwrap()
    };

    catalog
        .register(MethodRegistration {
            provider_id: ProviderId::new("battle").unwrap(),
            path: path("/battle/state"),
            schema: schema("inspect"),
            schema_version: SchemaVersion::new(1).unwrap(),
            effect: Effect::Read,
        })
        .unwrap();
    let mut runtime = Runtime::new(Arc::new(catalog), authorization.checker());
    runtime
        .bind_provider(ProviderId::new("battle").unwrap(), provider.clone())
        .unwrap();

    let failure = runtime.execute(plan).unwrap_err();
    assert_eq!(failure.error, ExecutionError::CatalogChanged);
    assert_eq!(provider.call_count(), 0);
}

#[test]
fn revoked_plans_do_not_reveal_catalog_or_provider_state() {
    let authorization = AuthorizationService::new();
    let agent = principal(&authorization, "agent");
    grant_method(
        &authorization,
        &agent,
        "/battle/turn",
        "submit",
        &[Capability::Discover, Capability::Invoke],
    );
    let mut catalog = Catalog::new();
    register(&mut catalog, "/battle/turn", "submit", Effect::Invoke);
    let sealing_catalog = Arc::new(catalog);
    let compiler = Compiler::new(Arc::clone(&sealing_catalog));
    let (provider_plan, catalog_plan) = {
        let session = authorization.session(&agent).unwrap();
        let view = session.view();
        let seal = || {
            compiler
                .seal(
                    &view,
                    PlanDraft {
                        calls: vec![call("/battle/turn", "submit")],
                    },
                )
                .unwrap()
        };
        (seal(), seal())
    };
    authorization.revoke_all(&agent).unwrap();

    let missing_provider = Runtime::new(Arc::clone(&sealing_catalog), authorization.checker());
    assert_eq!(
        missing_provider.execute(provider_plan).unwrap_err().error,
        ExecutionError::AuthorizationRevoked
    );

    let mut changed_catalog = (*sealing_catalog).clone();
    register(
        &mut changed_catalog,
        "/battle/state",
        "inspect",
        Effect::Read,
    );
    let changed_catalog = Runtime::new(Arc::new(changed_catalog), authorization.checker());
    assert_eq!(
        changed_catalog.execute(catalog_plan).unwrap_err().error,
        ExecutionError::AuthorizationRevoked
    );
}
