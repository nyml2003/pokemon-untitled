use std::collections::BTreeMap;
use std::env;
use std::sync::Arc;

use ramus_core::{
    AuthorizationService, Capability, Catalog, CompileLimits, Compiler, Effect, EffectPermit,
    MethodName, MethodRegistration, MethodSchema, NodePath, ParameterName, ParameterSchema,
    ParseLimits, PlanDraft, Provider, ProviderError, ProviderId, ProviderRequest, Runtime,
    SchemaVersion, Value, ValueType, parse_with_limits,
};

struct BattleProvider;

impl Provider for BattleProvider {
    fn execute(
        &self,
        permit: EffectPermit,
        request: &ProviderRequest,
    ) -> Result<Value, ProviderError> {
        debug_assert_eq!(permit.path(), &request.path);
        debug_assert_eq!(permit.method(), &request.method);
        Ok(Value::Record(BTreeMap::from([
            ("accepted".into(), Value::Boolean(true)),
            (
                "principal".into(),
                Value::String(permit.principal().as_str().into()),
            ),
        ])))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = env::args().skip(1).collect::<Vec<_>>().join(" ");
    let source = if source.is_empty() {
        "/battle/turn submit move=thunderbolt target=opponent:1".to_owned()
    } else {
        source
    };

    let path = NodePath::parse("/battle/turn")?;
    let method = MethodName::new("submit")?;
    let provider_id = ProviderId::new("battle")?;
    let schema = MethodSchema::new(
        method.clone(),
        vec![
            ParameterSchema {
                name: ParameterName::new("move")?,
                value_type: ValueType::Enum(vec!["thunderbolt".into(), "surf".into()]),
                required: true,
                positional: true,
            },
            ParameterSchema {
                name: ParameterName::new("target")?,
                value_type: ValueType::String,
                required: true,
                positional: true,
            },
        ],
    )
    .map_err(|error| format!("invalid example schema: {error:?}"))?;

    let mut catalog = Catalog::new();
    catalog
        .register(MethodRegistration {
            provider_id: provider_id.clone(),
            path: path.clone(),
            schema,
            schema_version: SchemaVersion::new(1).expect("version is non-zero"),
            effect: Effect::Invoke,
        })
        .map_err(|error| format!("catalog registration failed: {error:?}"))?;
    let catalog = Arc::new(catalog);

    let authorization = AuthorizationService::new();
    let principal = authorization
        .create_principal("local-player")
        .map_err(|error| format!("principal creation failed: {error:?}"))?;
    for capability in [
        Capability::Discover,
        Capability::Complete,
        Capability::Invoke,
    ] {
        authorization
            .grant(&principal, path.clone(), Some(method.clone()), capability)
            .map_err(|error| format!("grant failed: {error:?}"))?;
    }

    let compiler = Compiler::new(Arc::clone(&catalog));
    let document = parse_with_limits(
        &source,
        ParseLimits {
            max_source_bytes: 4096,
            max_calls: 8,
            max_arguments_per_call: 16,
        },
    )?;
    let plan = {
        let session = authorization
            .session(&principal)
            .map_err(|error| format!("authorization failed: {error:?}"))?;
        compiler
            .seal_with_limits(
                &session.view(),
                PlanDraft::from(document),
                CompileLimits {
                    max_calls: 8,
                    max_arguments_per_call: 16,
                    ..CompileLimits::default()
                },
            )
            .map_err(|error| format!("{}: {}", error.code.as_str(), error.message))?
    };

    let mut runtime = Runtime::new(catalog, authorization.checker());
    runtime
        .bind_provider(provider_id, Arc::new(BattleProvider))
        .map_err(|error| format!("provider binding failed: {error:?}"))?;
    let report = runtime.execute(plan).map_err(|failure| {
        format!(
            "execution failed at call {}: {:?}",
            failure.call_index, failure.error
        )
    })?;

    println!("{:#?}", report.outputs);
    Ok(())
}
