use std::sync::Arc;

use ramus_core::{
    AuthorizationService, Catalog, Compiler, Effect, MethodName, MethodRegistration, MethodSchema,
    NodePath, ParameterName, ParameterSchema, PlanDraft, ProviderId, SchemaVersion, ValueType,
};

#[test]
fn pure_catalog_registration_does_not_require_a_provider_handler() {
    let mut catalog = Catalog::new();

    catalog
        .register(MethodRegistration {
            provider_id: ProviderId::new("battle").unwrap(),
            path: NodePath::parse("/battle/state").unwrap(),
            schema: MethodSchema::new(
                MethodName::new("get").unwrap(),
                vec![ParameterSchema {
                    name: ParameterName::new("side").unwrap(),
                    value_type: ValueType::String,
                    required: true,
                    positional: true,
                }],
            )
            .unwrap(),
            schema_version: SchemaVersion::new(1).unwrap(),
            effect: Effect::Read,
        })
        .unwrap();

    assert_eq!(catalog.len(), 1);
}

#[test]
fn pure_compiler_accepts_an_explicit_capability_view() {
    let catalog = Arc::new(Catalog::new());
    let compiler = Compiler::new(catalog);
    let authorization = AuthorizationService::new();
    let principal = authorization.create_principal("agent").unwrap();
    let session = authorization.session(&principal).unwrap();
    let error = match compiler.seal(&session.view(), PlanDraft { calls: vec![] }) {
        Ok(_) => panic!("empty plan must not seal"),
        Err(error) => error,
    };

    assert_eq!(error.code.as_str(), "empty-plan");
}
