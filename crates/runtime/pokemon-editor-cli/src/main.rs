//! JSON Lines adapter for the Pokemon editor application protocol.

#![forbid(unsafe_code)]

use std::{
    error::Error,
    io::{self, BufRead, Write},
    path::PathBuf,
};

use editor_application::{EditorCall, EditorDocumentId, EditorOperation};
use editor_ramus_adapter::{EditorRamusRouter, RoutedEditorIntent};
use editor_resource_adapter::EditorResourceRegistry;
use pokemon_editor_core::{PokemonCatalog, PokemonEditorCommand, PokemonEditorModel};

const DOCUMENT: &str = "kanto-hoenn-pokedex";

fn main() -> Result<(), Box<dyn Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../");
    let registry = EditorResourceRegistry::standard(root)?;
    let document = EditorDocumentId::new(DOCUMENT)?;
    let mut model = load(&registry, &document)?;
    let router = EditorRamusRouter::new()?;
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();

    for line in stdin.lock().lines() {
        let response = match line {
            Ok(line) if line.trim().is_empty() => continue,
            Ok(line) => handle(&router, &registry, &document, &mut model, &line),
            Err(error) => Err(error.to_string()),
        };
        serde_json::to_writer(&mut stdout, &response_to_json(response))?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
    }
    Ok(())
}

fn handle(
    router: &EditorRamusRouter,
    registry: &EditorResourceRegistry,
    document: &EditorDocumentId,
    model: &mut PokemonEditorModel,
    source: &str,
) -> Result<serde_json::Value, String> {
    let call = EditorCall::from_json(source).map_err(|error| error.to_string())?;
    let intent = router.route_call(call).map_err(|error| error.message)?;
    match intent {
        RoutedEditorIntent::Open {
            kind,
            document: requested,
        } => {
            if kind != editor_application::EditorKind::Pokemon || requested != *document {
                return Err(String::from(
                    "Pokemon CLI cannot open the requested document",
                ));
            }
            *model = load(registry, document).map_err(|error| error.to_string())?;
            Ok(serde_json::json!({"opened": document.as_str()}))
        }
        RoutedEditorIntent::Call(call) => execute_call(registry, document, model, call),
    }
}

fn execute_call(
    registry: &EditorResourceRegistry,
    document: &EditorDocumentId,
    model: &mut PokemonEditorModel,
    call: EditorCall,
) -> Result<serde_json::Value, String> {
    if call.kind() != editor_application::EditorKind::Pokemon || call.document() != document {
        return Err(String::from(
            "Pokemon CLI cannot handle the requested document",
        ));
    }
    let command = match call.operation() {
        EditorOperation::Inspect => PokemonEditorCommand::Inspect,
        EditorOperation::Validate => PokemonEditorCommand::Validate,
        EditorOperation::Command => serde_json::from_value(call.payload().clone())
            .map(PokemonEditorCommand::Edit)
            .map_err(|error| format!("invalid Pokemon edit command: {error}"))?,
        EditorOperation::Save => PokemonEditorCommand::Save,
    };
    let (next, result) = model.execute(command).map_err(|error| error.to_string())?;
    *model = next;
    if matches!(call.operation(), EditorOperation::Save) {
        registry
            .save_text(
                editor_application::EditorKind::Pokemon,
                document,
                &model
                    .catalog()
                    .to_json_pretty()
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
        *model = model.clone().saved();
    }
    serde_json::to_value(result).map_err(|error| error.to_string())
}

fn load(
    registry: &EditorResourceRegistry,
    document: &EditorDocumentId,
) -> Result<PokemonEditorModel, Box<dyn Error>> {
    let source = registry.load_text(editor_application::EditorKind::Pokemon, document)?;
    Ok(PokemonEditorModel::new(PokemonCatalog::from_json(
        &source,
    )?)?)
}

fn response_to_json(response: Result<serde_json::Value, String>) -> serde_json::Value {
    match response {
        Ok(value) => serde_json::json!({"ok": value}),
        Err(message) => serde_json::json!({"error": message}),
    }
}
