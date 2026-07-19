use ramus_core::{
    Argument, DraftArgument, ParseDiagnosticKind, ParseLimits, PlanDraft, Value, parse,
    parse_with_limits,
};

#[test]
fn lowering_preserves_quoted_and_bare_literal_types() {
    let document = parse("/flags set first bare=true quoted=\"true\"").unwrap();
    let draft = PlanDraft::from(document);

    assert_eq!(
        draft.calls[0].arguments,
        vec![
            DraftArgument {
                name: None,
                value: Value::String("first".into()),
            },
            DraftArgument {
                name: Some("bare".into()),
                value: Value::Boolean(true),
            },
            DraftArgument {
                name: Some("quoted".into()),
                value: Value::String("true".into()),
            },
        ]
    );
}

#[test]
fn parsed_argument_spans_cover_the_original_argument() {
    let document = parse("/battle/turn submit move=thunderbolt").unwrap();

    assert!(matches!(
        &document.calls[0].arguments[0],
        Argument::Named { span, .. } if &"/battle/turn submit move=thunderbolt"[span.start..span.end] == "move=thunderbolt"
    ));
}

#[test]
fn parser_limits_reject_oversized_source_calls_and_arguments() {
    let source_error = parse_with_limits(
        "/x run",
        ParseLimits {
            max_source_bytes: 3,
            max_calls: 1,
            max_arguments_per_call: 1,
        },
    )
    .unwrap_err();
    assert_eq!(
        source_error.diagnostics()[0].kind,
        ParseDiagnosticKind::SourceTooLarge
    );
    assert_eq!(
        source_error.diagnostics()[0].to_string(),
        "source exceeds its limit at 1:1"
    );

    let call_error = parse_with_limits(
        "/x run\n/y run",
        ParseLimits {
            max_source_bytes: 64,
            max_calls: 1,
            max_arguments_per_call: 1,
        },
    )
    .unwrap_err();
    assert_eq!(
        call_error.diagnostics()[0].kind,
        ParseDiagnosticKind::TooManyCalls
    );
    assert_eq!(
        call_error.diagnostics()[0].to_string(),
        "call count exceeds its limit at 1:1"
    );

    let argument_error = parse_with_limits(
        "/x run one two",
        ParseLimits {
            max_source_bytes: 64,
            max_calls: 1,
            max_arguments_per_call: 1,
        },
    )
    .unwrap_err();
    assert_eq!(
        argument_error.diagnostics()[0].kind,
        ParseDiagnosticKind::TooManyArguments
    );
    assert_eq!(
        argument_error.diagnostics()[0].to_string(),
        "argument count exceeds its limit at 1:12"
    );

    assert!(
        parse_with_limits(
            "/x run one",
            ParseLimits {
                max_source_bytes: 10,
                max_calls: 1,
                max_arguments_per_call: 1,
            },
        )
        .is_ok()
    );
}
