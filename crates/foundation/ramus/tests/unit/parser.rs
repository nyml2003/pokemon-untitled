use std::fmt;

use super::{ForbiddenSyntax, ParseDiagnosticKind, ParseFailure, diagnostic, parse};
use crate::ast::{Argument, Span};
use crate::schema::Value;

fn argument_value(argument: &Argument) -> &Value {
    match argument {
        Argument::Positional(value) => &value.value,
        Argument::Named { value, .. } => &value.value,
    }
}

#[test]
fn parses_the_user_facing_positional_shape() {
    let document = parse("/bigFunc1 smallFunc1 arg1 arg2").unwrap();

    assert_eq!(document.calls.len(), 1);
    let call = &document.calls[0];
    assert_eq!(call.path.value.as_str(), "/bigFunc1");
    assert_eq!(call.method.value.as_str(), "smallFunc1");
    assert_eq!(
        call.arguments,
        vec![
            Argument::Positional(crate::ast::Spanned::new(
                Value::String("arg1".to_owned()),
                crate::ast::Span::new(21, 25),
            )),
            Argument::Positional(crate::ast::Spanned::new(
                Value::String("arg2".to_owned()),
                crate::ast::Span::new(26, 30),
            )),
        ]
    );
}

#[test]
fn parses_named_typed_and_escaped_values() {
    let document = parse(
        "/battle turn slot=2 force=true note=\"line one\\nline two\"\r\n/battle inspect false",
    )
    .unwrap();

    assert_eq!(document.calls.len(), 2);
    assert_eq!(
        argument_value(&document.calls[0].arguments[0]),
        &Value::Integer(2)
    );
    assert_eq!(
        argument_value(&document.calls[0].arguments[1]),
        &Value::Boolean(true)
    );
    assert_eq!(
        argument_value(&document.calls[0].arguments[2]),
        &Value::String("line one\nline two".to_owned())
    );
    assert_eq!(
        argument_value(&document.calls[1].arguments[0]),
        &Value::Boolean(false)
    );
}

#[test]
fn accepts_one_terminal_line_ending() {
    assert!(parse("/one run\n").is_ok());
    assert!(parse("/one run\r\n").is_ok());
}

#[test]
fn rejects_empty_statements() {
    for source in ["\n/one run", "/one run\n\n/two run", "/one run\n  "] {
        let failure = parse(source).unwrap_err();
        assert_eq!(
            failure.diagnostics()[0].kind,
            ParseDiagnosticKind::EmptyStatement
        );
    }
}

#[test]
fn rejects_shell_control_syntax_outside_quotes() {
    let cases = [
        ("/x run a|b", ForbiddenSyntax::Pipe),
        ("/x run a>b", ForbiddenSyntax::Redirection),
        ("/x run <in", ForbiddenSyntax::Redirection),
        ("/x run a;b", ForbiddenSyntax::StatementSeparator),
        ("/x run $HOME", ForbiddenSyntax::VariableExpansion),
        ("/x run $(other)", ForbiddenSyntax::CommandSubstitution),
        ("/x run `other`", ForbiddenSyntax::CommandSubstitution),
    ];

    for (source, expected) in cases {
        let failure = parse(source).unwrap_err();
        assert_eq!(
            failure.diagnostics()[0].kind,
            ParseDiagnosticKind::ForbiddenSyntax(expected),
            "source: {source}"
        );
    }
}

#[test]
fn treats_shell_metacharacters_inside_quotes_as_data() {
    let document = parse("/x run \"$HOME | <in >out; $(noop) `noop`\"").unwrap();
    assert_eq!(
        argument_value(&document.calls[0].arguments[0]),
        &Value::String("$HOME | <in >out; $(noop) `noop`".to_owned())
    );
}

#[test]
fn reports_invalid_names_and_values_with_source_locations() {
    let failure = parse("/ok run 1\nnot-a-path run\n/ok 1method\n/ok run huge=9223372036854775808")
        .unwrap_err();

    assert_eq!(failure.diagnostics().len(), 3);
    assert_eq!(
        failure.diagnostics()[0].kind,
        ParseDiagnosticKind::InvalidNodePath {
            value: "not-a-path".to_owned()
        }
    );
    assert_eq!(failure.diagnostics()[0].location.line, 2);
    assert_eq!(
        failure.diagnostics()[1].kind,
        ParseDiagnosticKind::InvalidMethodName {
            value: "1method".to_owned()
        }
    );
    assert_eq!(
        failure.diagnostics()[2].kind,
        ParseDiagnosticKind::IntegerOutOfRange {
            value: "9223372036854775808".to_owned()
        }
    );
    assert_eq!(failure.diagnostics()[2].location.line, 4);
    assert_eq!(failure.diagnostics()[2].location.column, 14);
}

#[test]
fn rejects_ambiguous_equals_and_token_adjacency() {
    let cases = [
        (
            "/x run name =value",
            ParseDiagnosticKind::WhitespaceAroundEquals,
        ),
        (
            "/x run name= value",
            ParseDiagnosticKind::WhitespaceAroundEquals,
        ),
        ("/x run name=", ParseDiagnosticKind::MissingArgumentValue),
        ("/x run foo\"bar\"", ParseDiagnosticKind::MissingWhitespace),
    ];

    for (source, expected) in cases {
        let failure = parse(source).unwrap_err();
        assert_eq!(failure.diagnostics()[0].kind, expected, "source: {source}");
    }
}

#[test]
fn reports_string_errors_at_the_escape_or_opening_quote() {
    let invalid_escape = parse("/x run \"bad\\q\"").unwrap_err();
    assert_eq!(
        invalid_escape.diagnostics()[0].kind,
        ParseDiagnosticKind::InvalidEscape { escape: 'q' }
    );
    assert_eq!(invalid_escape.diagnostics()[0].location.column, 12);

    let unterminated = parse("/x run \"open").unwrap_err();
    assert_eq!(
        unterminated.diagnostics()[0].kind,
        ParseDiagnosticKind::UnterminatedString
    );
    assert_eq!(unterminated.diagnostics()[0].location.column, 8);
}

#[test]
fn decodes_every_supported_string_escape() {
    let document = parse(r#"/x run "\"\\\n\r\t""#).unwrap();

    assert_eq!(
        argument_value(&document.calls[0].arguments[0]),
        &Value::String("\"\\\n\r\t".to_owned())
    );
}

#[test]
fn reports_a_trailing_backslash_as_an_unterminated_string() {
    let failure = parse("/x run \"trailing\\").unwrap_err();

    assert_eq!(
        failure.diagnostics()[0].kind,
        ParseDiagnosticKind::UnterminatedString
    );
    assert_eq!(failure.diagnostics()[0].span, Span::new(7, 17));
}

#[test]
fn rejects_control_characters_in_bare_values() {
    let failure = parse("/x run bare\u{1}").unwrap_err();

    assert_eq!(
        failure.diagnostics()[0].kind,
        ParseDiagnosticKind::UnexpectedCharacter { character: '\u{1}' }
    );
    assert_eq!(failure.diagnostics()[0].location.column, 12);
}

#[test]
fn rejects_control_characters_in_quoted_values() {
    let failure = parse("/x run \"quoted\u{1}\"").unwrap_err();

    assert_eq!(
        failure.diagnostics()[0].kind,
        ParseDiagnosticKind::UnexpectedCharacter { character: '\u{1}' }
    );
    assert_eq!(failure.diagnostics()[0].location.column, 15);
}

#[test]
fn rejects_a_quoted_node_path() {
    let failure = parse("\"/x\" run").unwrap_err();

    assert_eq!(
        failure.diagnostics()[0].kind,
        ParseDiagnosticKind::ExpectedNodePath
    );
}

#[test]
fn rejects_a_quoted_method_name() {
    let failure = parse("/x \"run\"").unwrap_err();

    assert_eq!(
        failure.diagnostics()[0].kind,
        ParseDiagnosticKind::ExpectedMethod
    );
}

#[test]
fn rejects_an_equals_token_as_an_argument() {
    let failure = parse("/x run =value").unwrap_err();

    assert_eq!(
        failure.diagnostics()[0].kind,
        ParseDiagnosticKind::ExpectedArgument
    );
}

#[test]
fn rejects_an_equals_token_as_a_named_value() {
    let failure = parse("/x run name==value").unwrap_err();

    assert_eq!(
        failure.diagnostics()[0].kind,
        ParseDiagnosticKind::MissingArgumentValue
    );
}

#[test]
fn rejects_an_invalid_parameter_name() {
    let failure = parse("/x run 1name=value").unwrap_err();

    assert_eq!(
        failure.diagnostics()[0].kind,
        ParseDiagnosticKind::InvalidParameterName {
            value: "1name".to_owned()
        }
    );
}

#[test]
fn rejects_an_out_of_range_positional_integer() {
    let failure = parse("/x run 9223372036854775808").unwrap_err();

    assert_eq!(
        failure.diagnostics()[0].kind,
        ParseDiagnosticKind::IntegerOutOfRange {
            value: "9223372036854775808".to_owned()
        }
    );
}

#[test]
fn reports_a_missing_method_at_the_end_of_the_call() {
    let failure = parse("/x").unwrap_err();

    assert_eq!(
        failure.diagnostics()[0].kind,
        ParseDiagnosticKind::ExpectedMethod
    );
    assert_eq!(failure.diagnostics()[0].span, Span::new(2, 2));
    assert_eq!(failure.diagnostics()[0].location.column, 3);
}

#[test]
fn requires_whitespace_between_the_path_and_method() {
    let failure = parse("/x\"run\"").unwrap_err();

    assert_eq!(
        failure.diagnostics()[0].kind,
        ParseDiagnosticKind::MissingWhitespace
    );
    assert_eq!(failure.diagnostics()[0].location.column, 3);
}

#[test]
fn consuming_a_failure_preserves_its_diagnostics() {
    let diagnostics = parse("bad run\n/x").unwrap_err().into_diagnostics();

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(
        diagnostics[0].kind,
        ParseDiagnosticKind::InvalidNodePath {
            value: "bad".to_owned()
        }
    );
    assert_eq!(diagnostics[1].kind, ParseDiagnosticKind::ExpectedMethod);
}

#[test]
fn formats_a_single_parse_failure_without_a_count_suffix() {
    let failure = parse("").unwrap_err();

    assert_eq!(failure.to_string(), "expected at least one call at 1:1");
}

#[test]
fn propagates_an_error_while_formatting_the_first_diagnostic() {
    struct RejectAll;

    impl fmt::Write for RejectAll {
        fn write_str(&mut self, _value: &str) -> fmt::Result {
            Err(fmt::Error)
        }
    }

    let failure = parse("bad run").unwrap_err();
    let result = fmt::write(&mut RejectAll, format_args!("{failure}"));

    assert_eq!(result, Err(fmt::Error));
}

#[test]
fn formats_a_multi_diagnostic_failure_with_the_remaining_count() {
    let failure = parse("bad run\n/x").unwrap_err();

    assert_eq!(
        failure.to_string(),
        "invalid node path `bad` at 1:1 (and 1 more diagnostics)"
    );
}

#[test]
fn propagates_an_error_while_formatting_the_remaining_count() {
    struct RejectSuffix;

    impl fmt::Write for RejectSuffix {
        fn write_str(&mut self, value: &str) -> fmt::Result {
            if value.starts_with(" (and ") {
                Err(fmt::Error)
            } else {
                Ok(())
            }
        }
    }

    let failure = parse("bad run\n/x").unwrap_err();
    let result = fmt::write(&mut RejectSuffix, format_args!("{failure}"));

    assert_eq!(result, Err(fmt::Error));
}

#[test]
fn formats_an_empty_failure_defensively() {
    let failure = ParseFailure {
        diagnostics: Vec::new(),
    };

    assert_eq!(failure.to_string(), "shell text could not be parsed");
}

#[test]
fn formats_every_structured_diagnostic_kind() {
    let cases = [
        (
            ParseDiagnosticKind::InvalidSourceBoundary,
            "source contains an invalid character boundary",
        ),
        (
            ParseDiagnosticKind::EmptyInput,
            "expected at least one call",
        ),
        (
            ParseDiagnosticKind::EmptyStatement,
            "empty statements are not allowed",
        ),
        (
            ParseDiagnosticKind::ExpectedNodePath,
            "expected a node path",
        ),
        (
            ParseDiagnosticKind::InvalidNodePath {
                value: "bad".to_owned(),
            },
            "invalid node path `bad`",
        ),
        (
            ParseDiagnosticKind::ExpectedMethod,
            "expected a method name",
        ),
        (
            ParseDiagnosticKind::InvalidMethodName {
                value: "1bad".to_owned(),
            },
            "invalid method name `1bad`",
        ),
        (
            ParseDiagnosticKind::ExpectedArgument,
            "expected an argument",
        ),
        (
            ParseDiagnosticKind::InvalidParameterName {
                value: "1bad".to_owned(),
            },
            "invalid parameter name `1bad`",
        ),
        (
            ParseDiagnosticKind::MissingArgumentValue,
            "expected a value after `=`",
        ),
        (
            ParseDiagnosticKind::WhitespaceAroundEquals,
            "whitespace around `=` is not allowed",
        ),
        (
            ParseDiagnosticKind::MissingWhitespace,
            "expected whitespace between tokens",
        ),
        (
            ParseDiagnosticKind::UnterminatedString,
            "unterminated string",
        ),
        (
            ParseDiagnosticKind::InvalidEscape { escape: 'q' },
            "unsupported escape sequence `\\q`",
        ),
        (
            ParseDiagnosticKind::IntegerOutOfRange {
                value: "999".to_owned(),
            },
            "integer `999` is outside the i64 range",
        ),
        (
            ParseDiagnosticKind::ForbiddenSyntax(ForbiddenSyntax::Pipe),
            "pipes is not supported",
        ),
        (
            ParseDiagnosticKind::ForbiddenSyntax(ForbiddenSyntax::Redirection),
            "redirection is not supported",
        ),
        (
            ParseDiagnosticKind::ForbiddenSyntax(ForbiddenSyntax::StatementSeparator),
            "semicolon statement separators is not supported",
        ),
        (
            ParseDiagnosticKind::ForbiddenSyntax(ForbiddenSyntax::VariableExpansion),
            "variable expansion is not supported",
        ),
        (
            ParseDiagnosticKind::ForbiddenSyntax(ForbiddenSyntax::CommandSubstitution),
            "command substitution is not supported",
        ),
        (
            ParseDiagnosticKind::UnexpectedCharacter { character: '\u{1}' },
            "unexpected character '\\u{1}'",
        ),
    ];

    for (kind, message) in cases {
        let diagnostic = diagnostic(kind, Span::new(2, 3), 4, 5);
        assert_eq!(diagnostic.to_string(), format!("{message} at 4:5"));
    }
}

#[test]
fn columns_count_characters_while_spans_use_bytes() {
    let failure = parse("/x run 日本語$HOME").unwrap_err();
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(
        diagnostic.kind,
        ParseDiagnosticKind::ForbiddenSyntax(ForbiddenSyntax::VariableExpansion)
    );
    assert_eq!(diagnostic.location.column, 11);
    assert_eq!(diagnostic.span.start, 16);
}
