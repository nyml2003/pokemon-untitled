//! Ramus registration and routing for every player and AI game intent.

#![forbid(unsafe_code)]

use std::sync::Arc;

use game_foundation::{BattleOutcome, Direction, GameCommand, ItemId, NpcId, WarpId};
use ramus_core::{
    AuthorizationService, Capability, Catalog, CompileLimits, Compiler, Effect, EffectPermit,
    ExecutionError, ExecutionFailure, MethodName, MethodRegistration, MethodSchema, NodePath,
    ParameterName, ParameterSchema, ParseDiagnosticKind, ParseFailure, ParseLimits, PlanDraft,
    Principal, Provider, ProviderError, ProviderId, ProviderRequest, Runtime, SchemaVersion, Value,
    ValueType, parse_with_limits,
};

const PLAYER_ID: &str = "local-player";
const PROVIDER_ID: &str = "game-intents";
const PARSE_LIMITS: ParseLimits = ParseLimits {
    max_source_bytes: 4096,
    max_calls: 32,
    max_arguments_per_call: 8,
};
const COMPILE_LIMITS: CompileLimits = CompileLimits {
    max_calls: 32,
    max_arguments_per_call: 8,
    max_total_bytes: 4096,
    max_value_bytes: 256,
    max_value_nodes: 64,
    max_value_depth: 4,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoutedIntent {
    Command(GameCommand),
    Save,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticStage {
    Parse,
    Seal,
    Provider,
    Runtime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouterDiagnostic {
    pub stage: DiagnosticStage,
    pub code: String,
    pub message: String,
}

pub struct GameRamusRouter {
    authorization: AuthorizationService,
    principal: Principal,
    compiler: Compiler,
    runtime: Runtime,
}

impl GameRamusRouter {
    pub fn new() -> Result<Self, RouterDiagnostic> {
        let provider_id = ProviderId::new(PROVIDER_ID).map_err(configuration_diagnostic)?;
        let catalog = build_catalog(&provider_id)?;
        let authorization = AuthorizationService::new();
        let principal = authorization
            .create_principal(PLAYER_ID)
            .map_err(configuration_diagnostic)?;
        grant_intents(&authorization, &principal)?;
        let compiler = Compiler::new(Arc::clone(&catalog));
        let mut runtime = Runtime::new(catalog, authorization.checker());
        runtime
            .bind_provider(provider_id, Arc::new(IntentProvider))
            .map_err(configuration_diagnostic)?;
        Ok(Self {
            authorization,
            principal,
            compiler,
            runtime,
        })
    }

    /// Parses, authorizes and schema-validates one or more newline-delimited intents.
    pub fn route(&self, source: &str) -> Result<Vec<RoutedIntent>, RouterDiagnostic> {
        let document = parse_with_limits(source, PARSE_LIMITS).map_err(parse_diagnostic)?;
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
        report.outputs.iter().map(intent_from_value).collect()
    }
}

struct IntentProvider;

impl Provider for IntentProvider {
    fn execute(
        &self,
        permit: EffectPermit,
        request: &ProviderRequest,
    ) -> Result<Value, ProviderError> {
        if permit.principal().as_str() != PLAYER_ID
            || permit.capability() != Capability::Invoke
            || permit.path() != &request.path
            || permit.method() != &request.method
        {
            return Err(rejected(
                "invalid-permit",
                "the invocation permit does not match the game intent request",
            ));
        }
        let intent = intent_from_request(request)?;
        Ok(intent_to_value(&intent))
    }
}

fn build_catalog(provider_id: &ProviderId) -> Result<Arc<Catalog>, RouterDiagnostic> {
    let mut catalog = Catalog::new();
    for (path, schema) in schemas()? {
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

fn schemas() -> Result<Vec<(NodePath, MethodSchema)>, RouterDiagnostic> {
    Ok(vec![
        schema("/game/session", "new", vec![])?,
        schema(
            "/game/npc",
            "interact",
            vec![parameter("npc", ValueType::String)?],
        )?,
        schema(
            "/game/world",
            "move",
            vec![parameter(
                "direction",
                enum_type(&["up", "down", "left", "right"]),
            )?],
        )?,
        schema(
            "/game/world",
            "warp",
            vec![parameter("warp", ValueType::String)?],
        )?,
        schema(
            "/game/world",
            "encounter",
            vec![parameter("roll", ValueType::Integer)?],
        )?,
        schema(
            "/game/battle",
            "resolve",
            vec![
                parameter("outcome", enum_type(&["victory", "defeat"]))?,
                parameter("hp", ValueType::Integer)?,
                parameter("pp", ValueType::Integer)?,
            ],
        )?,
        schema(
            "/game/shop",
            "buy",
            vec![
                parameter("npc", ValueType::String)?,
                parameter("item", ValueType::String)?,
                parameter("quantity", ValueType::Integer)?,
            ],
        )?,
        schema("/game/save", "save", vec![])?,
    ])
}

fn schema(
    path: &str,
    method: &str,
    parameters: Vec<ParameterSchema>,
) -> Result<(NodePath, MethodSchema), RouterDiagnostic> {
    let method = MethodName::new(method).map_err(configuration_diagnostic)?;
    let schema = MethodSchema::new(method, parameters).map_err(configuration_diagnostic)?;
    Ok((
        NodePath::parse(path).map_err(configuration_diagnostic)?,
        schema,
    ))
}

fn parameter(name: &str, value_type: ValueType) -> Result<ParameterSchema, RouterDiagnostic> {
    Ok(ParameterSchema {
        name: ParameterName::new(name).map_err(configuration_diagnostic)?,
        value_type,
        required: true,
        positional: false,
    })
}

fn enum_type(values: &[&str]) -> ValueType {
    ValueType::Enum(values.iter().map(|value| (*value).into()).collect())
}

fn grant_intents(
    authorization: &AuthorizationService,
    principal: &Principal,
) -> Result<(), RouterDiagnostic> {
    for (path, schema) in schemas()? {
        for capability in [
            Capability::Discover,
            Capability::Complete,
            Capability::Invoke,
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

fn intent_from_request(request: &ProviderRequest) -> Result<RoutedIntent, ProviderError> {
    let path = request.path.as_str();
    let method = request.method.as_str();
    match (path, method) {
        ("/game/session", "new") => {
            no_arguments(request).map(|()| RoutedIntent::Command(GameCommand::NewGame))
        }
        ("/game/npc", "interact") => Ok(RoutedIntent::Command(GameCommand::Interact {
            npc: NpcId::new(string_argument(request, "npc")?).map_err(game_error)?,
        })),
        ("/game/world", "move") => Ok(RoutedIntent::Command(GameCommand::Move {
            direction: direction_argument(request)?,
        })),
        ("/game/world", "warp") => Ok(RoutedIntent::Command(GameCommand::Warp {
            warp: WarpId::new(string_argument(request, "warp")?).map_err(game_error)?,
        })),
        ("/game/world", "encounter") => Ok(RoutedIntent::Command(GameCommand::Encounter {
            roll: u8_argument(request, "roll")?,
        })),
        ("/game/battle", "resolve") => Ok(RoutedIntent::Command(GameCommand::ResolveBattle {
            outcome: outcome_argument(request)?,
            hp: u16_argument(request, "hp")?,
            pp: u8_argument(request, "pp")?,
        })),
        ("/game/shop", "buy") => Ok(RoutedIntent::Command(GameCommand::Buy {
            npc: NpcId::new(string_argument(request, "npc")?).map_err(game_error)?,
            item: ItemId::new(string_argument(request, "item")?).map_err(game_error)?,
            quantity: u16_argument(request, "quantity")?,
        })),
        ("/game/save", "save") => no_arguments(request).map(|()| RoutedIntent::Save),
        _ => Err(rejected(
            "unknown-intent",
            "the game intent provider does not implement this invocation",
        )),
    }
}

fn no_arguments(request: &ProviderRequest) -> Result<(), ProviderError> {
    if request.arguments.is_empty() {
        Ok(())
    } else {
        Err(rejected(
            "unexpected-arguments",
            "this game intent does not accept arguments",
        ))
    }
}
fn string_argument<'a>(request: &'a ProviderRequest, name: &str) -> Result<&'a str, ProviderError> {
    match request.arguments.get(name) {
        Some(Value::String(value)) => Ok(value),
        _ => Err(rejected(
            "invalid-argument",
            "a required string argument is missing",
        )),
    }
}
fn integer_argument(request: &ProviderRequest, name: &str) -> Result<i64, ProviderError> {
    match request.arguments.get(name) {
        Some(Value::Integer(value)) => Ok(*value),
        _ => Err(rejected(
            "invalid-argument",
            "a required integer argument is missing",
        )),
    }
}
fn u8_argument(request: &ProviderRequest, name: &str) -> Result<u8, ProviderError> {
    u8::try_from(integer_argument(request, name)?).map_err(|_| {
        rejected(
            "invalid-argument",
            "integer argument is outside the u8 range",
        )
    })
}
fn u16_argument(request: &ProviderRequest, name: &str) -> Result<u16, ProviderError> {
    u16::try_from(integer_argument(request, name)?).map_err(|_| {
        rejected(
            "invalid-argument",
            "integer argument is outside the u16 range",
        )
    })
}
fn direction_argument(request: &ProviderRequest) -> Result<Direction, ProviderError> {
    match string_argument(request, "direction")? {
        "up" => Ok(Direction::Up),
        "down" => Ok(Direction::Down),
        "left" => Ok(Direction::Left),
        "right" => Ok(Direction::Right),
        _ => Err(rejected("invalid-argument", "direction is not supported")),
    }
}
fn outcome_argument(request: &ProviderRequest) -> Result<BattleOutcome, ProviderError> {
    match string_argument(request, "outcome")? {
        "victory" => Ok(BattleOutcome::Victory),
        "defeat" => Ok(BattleOutcome::Defeat),
        _ => Err(rejected(
            "invalid-argument",
            "battle outcome is not supported",
        )),
    }
}

fn intent_to_value(intent: &RoutedIntent) -> Value {
    Value::String(match intent {
        RoutedIntent::Command(GameCommand::NewGame) => "new".into(),
        RoutedIntent::Command(GameCommand::Interact { npc }) => {
            format!("interact:{}", npc.as_str())
        }
        RoutedIntent::Command(GameCommand::Move { direction }) => {
            format!("move:{}", direction_name(*direction))
        }
        RoutedIntent::Command(GameCommand::Warp { warp }) => format!("warp:{}", warp.as_str()),
        RoutedIntent::Command(GameCommand::Encounter { roll }) => format!("encounter:{roll}"),
        RoutedIntent::Command(GameCommand::ResolveBattle { outcome, hp, pp }) => {
            format!("resolve:{}:{hp}:{pp}", outcome_name(*outcome))
        }
        RoutedIntent::Command(GameCommand::Buy {
            npc,
            item,
            quantity,
        }) => format!("buy:{}:{}:{quantity}", npc.as_str(), item.as_str()),
        RoutedIntent::Save => "save".into(),
    })
}

fn intent_from_value(value: &Value) -> Result<RoutedIntent, RouterDiagnostic> {
    let Value::String(value) = value else {
        return Err(invalid_provider_output());
    };
    let parts = value.split(':').collect::<Vec<_>>();
    match parts.as_slice() {
        ["new"] => Ok(RoutedIntent::Command(GameCommand::NewGame)),
        ["save"] => Ok(RoutedIntent::Save),
        ["interact", npc] => Ok(RoutedIntent::Command(GameCommand::Interact {
            npc: NpcId::new(*npc).map_err(output_game_error)?,
        })),
        ["move", direction] => direction_from_name(direction)
            .map(|direction| RoutedIntent::Command(GameCommand::Move { direction })),
        ["warp", warp] => Ok(RoutedIntent::Command(GameCommand::Warp {
            warp: WarpId::new(*warp).map_err(output_game_error)?,
        })),
        ["encounter", roll] => Ok(RoutedIntent::Command(GameCommand::Encounter {
            roll: parse_output_integer(roll)?,
        })),
        ["resolve", outcome, hp, pp] => Ok(RoutedIntent::Command(GameCommand::ResolveBattle {
            outcome: outcome_from_name(outcome)?,
            hp: parse_output_integer(hp)?,
            pp: parse_output_integer(pp)?,
        })),
        ["buy", npc, item, quantity] => Ok(RoutedIntent::Command(GameCommand::Buy {
            npc: NpcId::new(*npc).map_err(output_game_error)?,
            item: ItemId::new(*item).map_err(output_game_error)?,
            quantity: parse_output_integer(quantity)?,
        })),
        _ => Err(invalid_provider_output()),
    }
}

fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Up => "up",
        Direction::Down => "down",
        Direction::Left => "left",
        Direction::Right => "right",
    }
}
fn outcome_name(outcome: BattleOutcome) -> &'static str {
    match outcome {
        BattleOutcome::Victory => "victory",
        BattleOutcome::Defeat => "defeat",
    }
}
fn direction_from_name(value: &str) -> Result<Direction, RouterDiagnostic> {
    match value {
        "up" => Ok(Direction::Up),
        "down" => Ok(Direction::Down),
        "left" => Ok(Direction::Left),
        "right" => Ok(Direction::Right),
        _ => Err(invalid_provider_output()),
    }
}
fn outcome_from_name(value: &str) -> Result<BattleOutcome, RouterDiagnostic> {
    match value {
        "victory" => Ok(BattleOutcome::Victory),
        "defeat" => Ok(BattleOutcome::Defeat),
        _ => Err(invalid_provider_output()),
    }
}
fn parse_output_integer<T>(value: &str) -> Result<T, RouterDiagnostic>
where
    T: TryFrom<u64>,
{
    value
        .parse::<u64>()
        .ok()
        .and_then(|value| T::try_from(value).ok())
        .ok_or_else(invalid_provider_output)
}

fn rejected(code: &str, message: &str) -> ProviderError {
    ProviderError::Rejected {
        code: code.into(),
        message: message.into(),
    }
}
fn game_error(error: impl std::fmt::Debug) -> ProviderError {
    rejected(
        "invalid-argument",
        &format!("invalid game identifier: {error:?}"),
    )
}
fn output_game_error(error: impl std::fmt::Debug) -> RouterDiagnostic {
    RouterDiagnostic {
        stage: DiagnosticStage::Runtime,
        code: "invalid-provider-output".into(),
        message: format!("provider returned invalid game identifier: {error:?}"),
    }
}
fn invalid_provider_output() -> RouterDiagnostic {
    RouterDiagnostic {
        stage: DiagnosticStage::Runtime,
        code: "invalid-provider-output".into(),
        message: "game intent provider returned an invalid intent token".into(),
    }
}
fn configuration_diagnostic(error: impl std::fmt::Debug) -> RouterDiagnostic {
    RouterDiagnostic {
        stage: DiagnosticStage::Runtime,
        code: "invalid-router-configuration".into(),
        message: format!("{error:?}"),
    }
}
fn parse_diagnostic(failure: ParseFailure) -> RouterDiagnostic {
    let Some(diagnostic) = failure.diagnostics().first() else {
        return RouterDiagnostic {
            stage: DiagnosticStage::Parse,
            code: "parse-failure".into(),
            message: "game intent text could not be parsed".into(),
        };
    };
    RouterDiagnostic {
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
fn seal_diagnostic(diagnostic: ramus_core::Diagnostic) -> RouterDiagnostic {
    RouterDiagnostic {
        stage: DiagnosticStage::Seal,
        code: diagnostic.code.as_str().into(),
        message: diagnostic.message,
    }
}
fn execution_diagnostic(failure: ExecutionFailure) -> RouterDiagnostic {
    match failure.error {
        ExecutionError::Provider(ProviderError::Rejected { code, message }) => RouterDiagnostic {
            stage: DiagnosticStage::Provider,
            code,
            message,
        },
        ExecutionError::CatalogChanged => RouterDiagnostic {
            stage: DiagnosticStage::Runtime,
            code: "catalog-changed".into(),
            message: "catalog changed before execution".into(),
        },
        ExecutionError::SchemaChanged => RouterDiagnostic {
            stage: DiagnosticStage::Runtime,
            code: "schema-changed".into(),
            message: "schema changed before execution".into(),
        },
        ExecutionError::AuthorizationRevoked => RouterDiagnostic {
            stage: DiagnosticStage::Runtime,
            code: "authorization-revoked".into(),
            message: "authorization was revoked before execution".into(),
        },
        ExecutionError::ProviderUnavailable => RouterDiagnostic {
            stage: DiagnosticStage::Runtime,
            code: "provider-unavailable".into(),
            message: "game intent provider is unavailable".into(),
        },
    }
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
