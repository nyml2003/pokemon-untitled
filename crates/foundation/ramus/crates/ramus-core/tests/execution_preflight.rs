use std::sync::Arc;

use ramus_core::{
    AuthorizationService, Capability, Catalog, Compiler, Effect, ExecutionError, MethodName,
    MethodRegistration, MethodSchema, NodePath, ParameterName, ParameterSchema, PlanDraft,
    ProviderId, SchemaVersion, ValueType,
};

fn registration(path: &str, method: &str) -> MethodRegistration {
    MethodRegistration {
        provider_id: ProviderId::new("battle").unwrap(),
        path: NodePath::parse(path).unwrap(),
        schema: MethodSchema::new(MethodName::new(method).unwrap(), vec![]).unwrap(),
        schema_version: SchemaVersion::new(1).unwrap(),
        effect: Effect::Read,
    }
}

#[test]
fn preflight_accepts_the_catalog_that_sealed_the_plan() {
    let catalog = Arc::new(Catalog::new());
    let authorization = AuthorizationService::new();
    let principal = authorization.create_principal("agent").unwrap();
    let compiler = Compiler::new(Arc::clone(&catalog));
    let empty = compiler.seal(
        &authorization.session(&principal).unwrap().view(),
        PlanDraft { calls: vec![] },
    );
    assert!(empty.is_err());
}

#[test]
fn runtime_reports_catalog_generation_changes_before_provider_lookup() {
    let mut sealing_catalog = Catalog::new();
    let item = registration("/battle/state", "get");
    sealing_catalog.register(item.clone()).unwrap();
    let authorization = AuthorizationService::new();
    let principal = authorization.create_principal("agent").unwrap();
    for capability in [Capability::Discover, Capability::Read] {
        authorization
            .grant(
                &principal,
                item.path.clone(),
                Some(item.schema.name().clone()),
                capability,
            )
            .unwrap();
    }
    let sealing_catalog = Arc::new(sealing_catalog);
    let compiler = Compiler::new(Arc::clone(&sealing_catalog));
    let plan = {
        let session = authorization.session(&principal).unwrap();
        compiler
            .seal(
                &session.view(),
                PlanDraft {
                    calls: vec![ramus_core::DraftCall {
                        path: "/battle/state".into(),
                        method: "get".into(),
                        arguments: vec![],
                    }],
                },
            )
            .unwrap()
    };

    let mut changed = (*sealing_catalog).clone();
    changed
        .register(registration("/battle/turn", "legal"))
        .unwrap();
    let runtime = ramus_core::Runtime::new(Arc::new(changed), authorization.checker());
    assert_eq!(
        runtime.execute(plan).unwrap_err().error,
        ExecutionError::CatalogChanged
    );
}

fn sealed_plan_for(item: &MethodRegistration) -> (ramus_core::TypedPlan, AuthorizationService) {
    let mut catalog = Catalog::new();
    catalog.register(item.clone()).unwrap();
    let authorization = AuthorizationService::new();
    let principal = authorization.create_principal("agent").unwrap();
    for capability in [Capability::Discover, Capability::Read] {
        authorization
            .grant(
                &principal,
                item.path.clone(),
                Some(item.schema.name().clone()),
                capability,
            )
            .unwrap();
    }
    let compiler = Compiler::new(Arc::new(catalog));
    let plan = {
        let session = authorization.session(&principal).unwrap();
        compiler
            .seal(
                &session.view(),
                PlanDraft {
                    calls: vec![ramus_core::DraftCall {
                        path: item.path.as_str().into(),
                        method: item.schema.name().as_str().into(),
                        arguments: vec![],
                    }],
                },
            )
            .unwrap()
    };
    (plan, authorization)
}

#[test]
fn preflight_rejects_a_same_generation_catalog_without_the_sealed_operation() {
    let sealed = registration("/battle/state", "get");
    let (plan, authorization) = sealed_plan_for(&sealed);
    let mut different = Catalog::new();
    different
        .register(registration("/battle/turn", "legal"))
        .unwrap();

    let runtime = ramus_core::Runtime::new(Arc::new(different), authorization.checker());
    assert_eq!(
        runtime.execute(plan).unwrap_err().error,
        ExecutionError::CatalogChanged
    );
}

#[test]
fn preflight_rejects_same_identity_with_a_different_schema_version() {
    let sealed = registration("/battle/state", "get");
    let (plan, authorization) = sealed_plan_for(&sealed);
    let mut changed = sealed.clone();
    changed.schema_version = SchemaVersion::new(2).unwrap();
    let mut catalog = Catalog::new();
    catalog.register(changed).unwrap();

    let runtime = ramus_core::Runtime::new(Arc::new(catalog), authorization.checker());
    assert_eq!(
        runtime.execute(plan).unwrap_err().error,
        ExecutionError::SchemaChanged
    );
}

#[test]
fn preflight_rejects_same_version_with_a_different_schema() {
    let sealed = registration("/battle/state", "get");
    let (plan, authorization) = sealed_plan_for(&sealed);
    let mut changed = sealed.clone();
    changed.schema = MethodSchema::new(
        MethodName::new("get").unwrap(),
        vec![ParameterSchema {
            name: ParameterName::new("format").unwrap(),
            value_type: ValueType::String,
            required: false,
            positional: false,
        }],
    )
    .unwrap();
    let mut catalog = Catalog::new();
    catalog.register(changed).unwrap();

    let runtime = ramus_core::Runtime::new(Arc::new(catalog), authorization.checker());
    assert_eq!(
        runtime.execute(plan).unwrap_err().error,
        ExecutionError::SchemaChanged
    );
}

#[test]
fn preflight_rejects_same_path_with_a_different_provider_or_effect() {
    for mutate in ["provider", "effect"] {
        let sealed = registration("/battle/state", "get");
        let (plan, authorization) = sealed_plan_for(&sealed);
        let mut changed = sealed.clone();
        if mutate == "provider" {
            changed.provider_id = ProviderId::new("other").unwrap();
        } else {
            changed.effect = Effect::Write;
        }
        let mut catalog = Catalog::new();
        catalog.register(changed).unwrap();
        let runtime = ramus_core::Runtime::new(Arc::new(catalog), authorization.checker());
        assert_eq!(
            runtime.execute(plan).unwrap_err().error,
            ExecutionError::SchemaChanged
        );
    }
}
