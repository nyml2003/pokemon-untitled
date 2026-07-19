use std::sync::Arc;

use ramus_core::{
    AuthorizationService, Capability, Catalog, Compiler, DiagnosticCode, Effect, ExecutionError,
    MethodName, MethodRegistration, MethodSchema, NodePath, PlanDraft, PrincipalError, Provider,
    ProviderError, ProviderId, ProviderRequest, Runtime, RuntimeConfigurationError, SchemaVersion,
    Value,
};

struct RejectingProvider;

struct AcceptingProvider;

impl Provider for AcceptingProvider {
    fn execute(
        &self,
        permit: ramus_core::EffectPermit,
        request: &ProviderRequest,
    ) -> Result<Value, ProviderError> {
        assert_eq!(permit.principal().as_str(), "agent");
        assert_eq!(permit.path(), &request.path);
        assert_eq!(permit.method(), &request.method);
        assert_eq!(permit.capability(), Capability::Invoke);
        Ok(Value::String("accepted".into()))
    }
}

impl Provider for RejectingProvider {
    fn execute(
        &self,
        _permit: ramus_core::EffectPermit,
        _request: &ProviderRequest,
    ) -> Result<Value, ProviderError> {
        Err(ProviderError::Rejected {
            code: "denied-by-application".into(),
            message: "application rejected the request".into(),
        })
    }
}

fn path(value: &str) -> NodePath {
    NodePath::parse(value).unwrap()
}

fn method(value: &str) -> MethodName {
    MethodName::new(value).unwrap()
}

fn registration(method_name: &str, effect: Effect) -> MethodRegistration {
    MethodRegistration {
        provider_id: ProviderId::new("test").unwrap(),
        path: path("/matrix"),
        schema: MethodSchema::new(method(method_name), vec![]).unwrap(),
        schema_version: SchemaVersion::new(1).unwrap(),
        effect,
    }
}

fn matrix_catalog() -> Arc<Catalog> {
    let mut catalog = Catalog::new();
    for (method_name, effect) in [
        ("read", Effect::Read),
        ("write", Effect::Write),
        ("invoke", Effect::Invoke),
    ] {
        catalog.register(registration(method_name, effect)).unwrap();
    }
    Arc::new(catalog)
}

fn draft(method_name: &str) -> PlanDraft {
    PlanDraft {
        calls: vec![ramus_core::DraftCall {
            path: "/matrix".into(),
            method: method_name.into(),
            arguments: vec![],
        }],
    }
}

#[test]
fn capability_matrix_is_default_deny_and_operation_specific() {
    let effects = [
        ("read", Capability::Read),
        ("write", Capability::Write),
        ("invoke", Capability::Invoke),
    ];
    for granted in [
        Capability::Discover,
        Capability::Complete,
        Capability::Read,
        Capability::Write,
        Capability::Invoke,
    ] {
        let authorization = AuthorizationService::new();
        let principal = authorization
            .create_principal(format!("principal-{granted:?}"))
            .unwrap();
        authorization
            .grant(&principal, path("/matrix"), None, granted)
            .unwrap();
        let compiler = Compiler::new(matrix_catalog());
        let session = authorization.session(&principal).unwrap();
        let view = session.view();
        let (discovered, completed, sealed) = (
            compiler.discover(&view).len(),
            compiler.complete(&view, "/matrix").len(),
            effects.map(|(method_name, _)| compiler.seal(&view, draft(method_name))),
        );

        assert_eq!(
            discovered,
            if granted == Capability::Discover {
                3
            } else {
                0
            },
            "discover with {granted:?}"
        );
        assert_eq!(
            completed,
            if granted == Capability::Complete {
                3
            } else {
                0
            },
            "complete with {granted:?}"
        );
        for ((_, required), sealed) in effects.into_iter().zip(sealed) {
            assert!(sealed.is_err(), "one grant must never be enough to seal");
            assert_eq!(
                sealed.err().unwrap().code,
                DiagnosticCode::OperationUnavailable,
                "{granted:?} must not leak {required:?}"
            );
        }
    }

    for (method_name, required) in effects {
        let authorization = AuthorizationService::new();
        let principal = authorization
            .create_principal(format!("principal-{method_name}"))
            .unwrap();
        for capability in [Capability::Discover, required] {
            authorization
                .grant(
                    &principal,
                    path("/matrix"),
                    Some(method(method_name)),
                    capability,
                )
                .unwrap();
        }
        let compiler = Compiler::new(matrix_catalog());
        let session = authorization.session(&principal).unwrap();
        let view = session.view();
        assert!(compiler.seal(&view, draft(method_name)).is_ok());
        for (other, _) in effects {
            if other != method_name {
                assert_eq!(
                    compiler.seal(&view, draft(other)).err().unwrap().code,
                    DiagnosticCode::OperationUnavailable
                );
            }
        }
    }
}

#[test]
fn principals_are_rejected_by_foreign_authorities() {
    let first = AuthorizationService::new();
    let second = AuthorizationService::new();
    let principal = first.create_principal("agent").unwrap();

    assert_eq!(
        second.grant(&principal, path("/"), None, Capability::Discover),
        Err(PrincipalError::ForeignAuthority)
    );
    assert_eq!(
        second.revoke_all(&principal),
        Err(PrincipalError::ForeignAuthority)
    );
    assert!(matches!(
        second.session(&principal),
        Err(PrincipalError::ForeignAuthority)
    ));
}

#[test]
fn principal_ids_cannot_be_reissued_by_the_same_authority() {
    let authorization = AuthorizationService::new();
    authorization.create_principal("developer").unwrap();

    assert!(authorization.create_principal("developer").is_err());
}

#[test]
fn duplicate_grants_and_noop_revokes_do_not_advance_generation() {
    let authorization = AuthorizationService::new();
    let principal = authorization.create_principal("agent").unwrap();
    let initial = authorization.generation(&principal).unwrap();
    authorization
        .grant(&principal, path("/"), None, Capability::Discover)
        .unwrap();
    let after_grant = authorization.generation(&principal).unwrap();
    assert_ne!(after_grant, initial);

    authorization
        .grant(&principal, path("/"), None, Capability::Discover)
        .unwrap();
    assert_eq!(authorization.generation(&principal).unwrap(), after_grant);
    authorization.revoke_all(&principal).unwrap();
    let after_revoke = authorization.generation(&principal).unwrap();
    assert_ne!(after_revoke, after_grant);
    authorization.revoke_all(&principal).unwrap();
    assert_eq!(authorization.generation(&principal).unwrap(), after_revoke);
}

fn seal_invoke(
    authorization: &AuthorizationService,
    catalog: Arc<Catalog>,
) -> ramus_core::TypedPlan {
    let principal = authorization.create_principal("agent").unwrap();
    for capability in [Capability::Discover, Capability::Invoke] {
        authorization
            .grant(
                &principal,
                path("/matrix"),
                Some(method("invoke")),
                capability,
            )
            .unwrap();
    }
    let session = authorization.session(&principal).unwrap();
    Compiler::new(catalog)
        .seal(&session.view(), draft("invoke"))
        .unwrap()
}

#[test]
fn runtime_rejects_duplicate_provider_bindings() {
    let catalog = matrix_catalog();
    let authorization = AuthorizationService::new();
    let plan = seal_invoke(&authorization, Arc::clone(&catalog));
    let mut runtime = Runtime::new(catalog, authorization.checker());
    let provider: Arc<dyn Provider> = Arc::new(AcceptingProvider);
    runtime
        .bind_provider(ProviderId::new("test").unwrap(), Arc::clone(&provider))
        .unwrap();
    assert_eq!(
        runtime.bind_provider(
            ProviderId::new("test").unwrap(),
            Arc::new(RejectingProvider)
        ),
        Err(RuntimeConfigurationError::DuplicateProvider)
    );
    assert_eq!(
        runtime.execute(plan).unwrap().outputs,
        vec![Value::String("accepted".into())]
    );
}

#[test]
fn runtime_reports_missing_and_rejected_providers() {
    let catalog = matrix_catalog();
    let missing_auth = AuthorizationService::new();
    let missing_plan = seal_invoke(&missing_auth, Arc::clone(&catalog));
    let missing_runtime = Runtime::new(Arc::clone(&catalog), missing_auth.checker());
    assert_eq!(
        missing_runtime.execute(missing_plan).unwrap_err().error,
        ExecutionError::ProviderUnavailable
    );

    let rejected_auth = AuthorizationService::new();
    let rejected_plan = seal_invoke(&rejected_auth, Arc::clone(&catalog));
    let mut rejected_runtime = Runtime::new(catalog, rejected_auth.checker());
    rejected_runtime
        .bind_provider(
            ProviderId::new("test").unwrap(),
            Arc::new(RejectingProvider),
        )
        .unwrap();
    assert_eq!(
        rejected_runtime.execute(rejected_plan).unwrap_err().error,
        ExecutionError::Provider(ProviderError::Rejected {
            code: "denied-by-application".into(),
            message: "application rejected the request".into(),
        })
    );
}

#[test]
fn unrelated_principal_changes_do_not_invalidate_a_sealed_plan() {
    let catalog = matrix_catalog();
    let authorization = AuthorizationService::new();
    let plan = seal_invoke(&authorization, Arc::clone(&catalog));
    let other = authorization.create_principal("other").unwrap();
    authorization
        .grant(&other, path("/other"), None, Capability::Discover)
        .unwrap();
    let mut runtime = Runtime::new(catalog, authorization.checker());
    runtime
        .bind_provider(
            ProviderId::new("test").unwrap(),
            Arc::new(AcceptingProvider),
        )
        .unwrap();

    assert_eq!(
        runtime.execute(plan).unwrap().outputs,
        vec![Value::String("accepted".into())]
    );
}
