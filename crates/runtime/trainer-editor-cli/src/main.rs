//! JSON Lines adapter for the trainer editor application protocol.

#![forbid(unsafe_code)]

use std::{
    error::Error,
    io::{self, BufRead, Write},
    path::PathBuf,
};

use editor_application::{EditorCall, EditorDocumentId, EditorOperation};
use editor_ramus_adapter::{EditorRamusRouter, RoutedEditorIntent};
use editor_resource_adapter::EditorResourceRegistry;
use game_foundation::TrainerCatalog;
use trainer_editor_core::{TrainerEditorCommand, TrainerEditorModel};

const DOCUMENT: &str = "route-trainers";

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
    model: &mut TrainerEditorModel,
    source: &str,
) -> Result<serde_json::Value, String> {
    let call = EditorCall::from_json(source).map_err(|error| error.to_string())?;
    let intent = router.route_call(call).map_err(|error| error.message)?;
    match intent {
        RoutedEditorIntent::Open {
            kind,
            document: requested,
        } => {
            if kind != editor_application::EditorKind::Trainer || requested != *document {
                return Err(String::from(
                    "trainer CLI cannot open the requested document",
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
    model: &mut TrainerEditorModel,
    call: EditorCall,
) -> Result<serde_json::Value, String> {
    if call.kind() != editor_application::EditorKind::Trainer || call.document() != document {
        return Err(String::from(
            "trainer CLI cannot handle the requested document",
        ));
    }
    let command = match call.operation() {
        EditorOperation::Inspect => TrainerEditorCommand::Inspect,
        EditorOperation::Validate => TrainerEditorCommand::Validate,
        EditorOperation::Command => serde_json::from_value(call.payload().clone())
            .map(TrainerEditorCommand::Edit)
            .map_err(|error| format!("invalid trainer edit command: {error}"))?,
        EditorOperation::Save => TrainerEditorCommand::Save,
    };
    let (next, result) = model.execute(command).map_err(|error| error.to_string())?;
    *model = next;
    if matches!(call.operation(), EditorOperation::Save) {
        registry
            .save_text(
                editor_application::EditorKind::Trainer,
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
) -> Result<TrainerEditorModel, Box<dyn Error>> {
    let source = registry.load_text(editor_application::EditorKind::Trainer, document)?;
    Ok(TrainerEditorModel::new(TrainerCatalog::from_json(
        &source,
    )?)?)
}

fn response_to_json(response: Result<serde_json::Value, String>) -> serde_json::Value {
    match response {
        Ok(value) => serde_json::json!({"ok": value}),
        Err(message) => serde_json::json!({"error": message}),
    }
}
