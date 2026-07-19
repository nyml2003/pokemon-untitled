use narrative_cps::CpsNode;
use narrative_token::SliceByteStream;

use super::*;

fn compile(source: &str, chunk_size: usize) -> CompileOutcome {
    Compiler::compile(SliceByteStream::with_chunk_size(
        source.as_bytes(),
        chunk_size,
    ))
}

#[test]
fn compiles_a_streamed_script_into_a_continuation_graph() {
    let outcome = compile(
        "script opening() { say(text: text:welcome); wait(event: event:road_clear); end(); }",
        1,
    );
    assert!(outcome.is_success());
    assert!(outcome.diagnostics().is_empty());
    let program = outcome.program().unwrap();
    assert_eq!(program.id().as_str(), "script:opening");
    assert!(matches!(
        program.continuation(ContinuationId::new(0)),
        Some(CpsNode::Say { text, next }) if text.as_str() == "text:welcome" && next.value() == 1
    ));
    assert!(matches!(
        program.continuation(ContinuationId::new(1)),
        Some(CpsNode::Wait { event, resume }) if event.as_str() == "event:road_clear" && resume.value() == 2
    ));
    assert!(matches!(
        program.continuation(ContinuationId::new(2)),
        Some(CpsNode::End)
    ));
}

#[test]
fn compiles_actor_bound_movement_and_facing_commands() {
    let outcome = compile(
        "script guide(actor: actor:forest-guide) { move(direction: right); face(direction: up); say(text: text:hello); end(); }",
        1,
    );
    assert!(outcome.is_success());
    let program = outcome.program().unwrap();
    assert_eq!(program.actor().unwrap().as_str(), "actor:forest-guide");
    assert!(matches!(
        program.continuation(ContinuationId::new(0)),
        Some(CpsNode::Move {
            direction: narrative_cps::ScriptDirection::Right,
            next,
        }) if next.value() == 1
    ));
    assert!(matches!(
        program.continuation(ContinuationId::new(1)),
        Some(CpsNode::Face {
            direction: narrative_cps::ScriptDirection::Up,
            next,
        }) if next.value() == 2
    ));
}

#[test]
fn compiler_returns_structured_diagnostics_for_every_baseline_failure() {
    let cases = [
        (
            "actor guard {}",
            DiagnosticCode::Expected,
            "expected script",
        ),
        (
            "script 9() { end(); }",
            DiagnosticCode::Expected,
            "expected identifier",
        ),
        (
            "script x(thing: actor:x) { end(); }",
            DiagnosticCode::Expected,
            "expected actor",
        ),
        (
            "script x() { say(text: event:no); end(); }",
            DiagnosticCode::InvalidResource,
            "expected text: resource, got event:",
        ),
        (
            "script x() { say(text: true); end(); }",
            DiagnosticCode::Expected,
            "expected resource ID",
        ),
        (
            "script x() { move(direction: north); end(); }",
            DiagnosticCode::InvalidDirection,
            "invalid direction 'north'",
        ),
        (
            "script x() { move(direction: right) end(); }",
            DiagnosticCode::Expected,
            "expected ';'",
        ),
        (
            "script x() { choose(); end(); }",
            DiagnosticCode::UnsupportedStatement,
            "unsupported statement 'choose'",
        ),
        (
            "script x() { end(); wait(event: event:x); }",
            DiagnosticCode::StatementAfterEnd,
            "statement follows end",
        ),
        (
            "script x() { say(text: text:x); }",
            DiagnosticCode::MissingEnd,
            "script must end with end();",
        ),
        (
            "script x() { end();",
            DiagnosticCode::Expected,
            "expected '}'",
        ),
        (
            "script x() { end(); } extra",
            DiagnosticCode::Expected,
            "expected end of file",
        ),
        (
            "script x() { say(text: text:x); @ }",
            DiagnosticCode::Lexical,
            "unexpected byte 0x40",
        ),
    ];
    for (source, code, message) in cases {
        let outcome = compile(source, 2);
        assert!(!outcome.is_success());
        assert!(outcome.program().is_none());
        assert_eq!(outcome.diagnostics().len(), 1);
        assert_eq!(outcome.diagnostics()[0].code(), code);
        assert_eq!(outcome.diagnostics()[0].message(), message);
        assert!(outcome.diagnostics()[0].span().end() >= outcome.diagnostics()[0].span().start());
    }
}
