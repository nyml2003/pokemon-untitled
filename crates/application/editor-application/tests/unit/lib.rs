use crate::{
    EDITOR_PROTOCOL_VERSION, EditorCall, EditorDocumentId, EditorKind, EditorOperation,
    EditorProtocolError,
};
use serde_json::json;

#[test]
fn structured_calls_keep_resource_identity_out_of_payloads() -> Result<(), EditorProtocolError> {
    let document = EditorDocumentId::new("route-trainers")
        .map_err(|error| EditorProtocolError::Json(error.to_string()))?;
    let call = EditorCall::new(
        EditorKind::Trainer,
        document,
        EditorOperation::Command,
        json!({ "set_name": { "trainer": "route-rival", "name": "小遥" } }),
    )?;
    let json = call.to_json()?;
    assert!(json.contains(EDITOR_PROTOCOL_VERSION));
    assert!(json.contains("route-trainers"));
    Ok(())
}

#[test]
fn read_operations_reject_payloads() -> Result<(), EditorProtocolError> {
    let document = EditorDocumentId::new("route-trainers")
        .map_err(|error| EditorProtocolError::Json(error.to_string()))?;
    let result = EditorCall::new(
        EditorKind::Trainer,
        document,
        EditorOperation::Inspect,
        json!({}),
    );
    assert!(matches!(
        result,
        Err(EditorProtocolError::UnexpectedPayload(
            EditorOperation::Inspect
        ))
    ));
    Ok(())
}

#[test]
fn json_calls_reject_invalid_document_ids() {
    let result = EditorCall::from_json(
        r#"{
            "protocol_version": "editor-v1",
            "kind": "trainer",
            "document": "../route-trainers",
            "operation": "inspect",
            "payload": null
        }"#,
    );
    assert!(matches!(result, Err(EditorProtocolError::Json(_))));
}
