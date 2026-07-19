use std::fmt;

use crate::ast::{Argument, Call, Document, Span, Spanned};
use crate::model::{MethodName, NodePath, ParameterName};
use crate::schema::Value;
use crate::value::{LiteralError, parse_bare_literal, parse_quoted_literal};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseLimits {
    pub max_source_bytes: usize,
    pub max_calls: usize,
    pub max_arguments_per_call: usize,
}

impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 64 * 1024,
            max_calls: 64,
            max_arguments_per_call: 64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForbiddenSyntax {
    Pipe,
    Redirection,
    StatementSeparator,
    VariableExpansion,
    CommandSubstitution,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseDiagnosticKind {
    SourceTooLarge,
    InvalidSourceBoundary,
    TooManyCalls,
    TooManyArguments,
    EmptyInput,
    EmptyStatement,
    ExpectedNodePath,
    InvalidNodePath { value: String },
    ExpectedMethod,
    InvalidMethodName { value: String },
    ExpectedArgument,
    InvalidParameterName { value: String },
    MissingArgumentValue,
    WhitespaceAroundEquals,
    MissingWhitespace,
    UnterminatedString,
    InvalidEscape { escape: char },
    IntegerOutOfRange { value: String },
    ForbiddenSyntax(ForbiddenSyntax),
    UnexpectedCharacter { character: char },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseDiagnostic {
    pub kind: ParseDiagnosticKind,
    pub span: Span,
    pub location: SourceLocation,
}

impl fmt::Display for ParseDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at {}:{}",
            DiagnosticMessage(&self.kind),
            self.location.line,
            self.location.column
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseFailure {
    diagnostics: Vec<ParseDiagnostic>,
}

impl ParseFailure {
    pub fn diagnostics(&self) -> &[ParseDiagnostic] {
        &self.diagnostics
    }

    pub fn into_diagnostics(self) -> Vec<ParseDiagnostic> {
        self.diagnostics
    }
}

impl fmt::Display for ParseFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(first) = self.diagnostics.first() else {
            return formatter.write_str("shell text could not be parsed");
        };
        write!(formatter, "{first}")?;
        if self.diagnostics.len() > 1 {
            write!(
                formatter,
                " (and {} more diagnostics)",
                self.diagnostics.len() - 1
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseFailure {}

/// Parses one or more newline-separated calls.
///
/// A single trailing line ending is accepted. Leading, whitespace-only, and
/// interior empty statements are rejected. Diagnostics are accumulated across
/// lines, with at most one diagnostic emitted for each malformed statement.
pub fn parse(source: &str) -> Result<Document, ParseFailure> {
    parse_with_limits(source, ParseLimits::default())
}

pub fn parse_with_limits(source: &str, limits: ParseLimits) -> Result<Document, ParseFailure> {
    if source.len() > limits.max_source_bytes {
        return Err(ParseFailure {
            diagnostics: vec![diagnostic(
                ParseDiagnosticKind::SourceTooLarge,
                Span::new(0, source.len()),
                1,
                1,
            )],
        });
    }
    if source.is_empty() {
        return Err(ParseFailure {
            diagnostics: vec![diagnostic(
                ParseDiagnosticKind::EmptyInput,
                Span::new(0, 0),
                1,
                1,
            )],
        });
    }

    let mut calls = Vec::new();
    let mut diagnostics = Vec::new();

    let lines = source_lines(source);
    if lines.len() > limits.max_calls {
        return Err(ParseFailure {
            diagnostics: vec![diagnostic(
                ParseDiagnosticKind::TooManyCalls,
                Span::new(0, source.len()),
                1,
                1,
            )],
        });
    }

    for line in lines {
        match parse_line(source, line, limits.max_arguments_per_call) {
            Ok(call) => calls.push(call),
            Err(error) => diagnostics.push(error),
        }
    }

    if !diagnostics.is_empty() {
        return Err(ParseFailure { diagnostics });
    }

    let (Some(first), Some(last)) = (calls.first(), calls.last()) else {
        return Err(ParseFailure {
            diagnostics: vec![diagnostic(
                ParseDiagnosticKind::EmptyInput,
                Span::new(0, source.len()),
                1,
                1,
            )],
        });
    };
    let span = Span::new(first.span.start, last.span.end);
    Ok(Document { calls, span })
}

#[derive(Clone, Copy)]
struct SourceLine {
    number: usize,
    start: usize,
    end: usize,
}

fn source_lines(source: &str) -> Vec<SourceLine> {
    let bytes = source.as_bytes();
    let mut lines = Vec::new();
    let mut line_start = 0;
    let mut line_number = 1;

    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'\n' {
            continue;
        }
        let end = if index > line_start && bytes[index - 1] == b'\r' {
            index - 1
        } else {
            index
        };
        lines.push(SourceLine {
            number: line_number,
            start: line_start,
            end,
        });
        line_start = index + 1;
        line_number += 1;
    }

    if line_start < source.len() {
        lines.push(SourceLine {
            number: line_number,
            start: line_start,
            end: source.len(),
        });
    }

    lines
}

fn parse_line(
    source: &str,
    line: SourceLine,
    max_arguments: usize,
) -> Result<Call, ParseDiagnostic> {
    let text = source_slice(source, line.start, line.end, line)?;
    if text.bytes().all(|byte| matches!(byte, b' ' | b'\t')) {
        return Err(source_diagnostic(
            source,
            ParseDiagnosticKind::EmptyStatement,
            Span::new(line.start, line.end),
            line,
        ));
    }

    let tokens = lex_line(source, line)?;
    let mut parser = LineParser {
        source,
        line,
        tokens,
        cursor: 0,
    };
    parser.parse_call(max_arguments)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TokenKind {
    Bare(String),
    Quoted(String),
    Equals,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Token {
    kind: TokenKind,
    span: Span,
}

fn lex_line(source: &str, line: SourceLine) -> Result<Vec<Token>, ParseDiagnostic> {
    let mut tokens = Vec::new();
    let mut offset = line.start;

    while offset < line.end {
        let character = character_at(source, line, offset)?;
        if matches!(character, ' ' | '\t') {
            offset += character.len_utf8();
            continue;
        }

        let start = offset;
        match character {
            '=' => {
                offset += 1;
                tokens.push(Token {
                    kind: TokenKind::Equals,
                    span: Span::new(start, offset),
                });
            }
            '"' => {
                let (value, end) = lex_quoted(source, line, start)?;
                offset = end;
                tokens.push(Token {
                    kind: TokenKind::Quoted(value),
                    span: Span::new(start, end),
                });
            }
            '|' => {
                return Err(forbidden_diagnostic(
                    source,
                    ForbiddenSyntax::Pipe,
                    start,
                    start + 1,
                    line,
                ));
            }
            '<' | '>' => {
                return Err(forbidden_diagnostic(
                    source,
                    ForbiddenSyntax::Redirection,
                    start,
                    start + 1,
                    line,
                ));
            }
            ';' => {
                return Err(forbidden_diagnostic(
                    source,
                    ForbiddenSyntax::StatementSeparator,
                    start,
                    start + 1,
                    line,
                ));
            }
            '$' => {
                let end = if source_slice(source, start + 1, line.end, line)?.starts_with('(') {
                    start + 2
                } else {
                    start + 1
                };
                let syntax = if end == start + 2 {
                    ForbiddenSyntax::CommandSubstitution
                } else {
                    ForbiddenSyntax::VariableExpansion
                };
                return Err(forbidden_diagnostic(source, syntax, start, end, line));
            }
            '`' => {
                return Err(forbidden_diagnostic(
                    source,
                    ForbiddenSyntax::CommandSubstitution,
                    start,
                    start + 1,
                    line,
                ));
            }
            character if character.is_control() => {
                return Err(source_diagnostic(
                    source,
                    ParseDiagnosticKind::UnexpectedCharacter { character },
                    Span::new(start, start + character.len_utf8()),
                    line,
                ));
            }
            _ => {
                offset = lex_bare_end(source, line, start)?;
                tokens.push(Token {
                    kind: TokenKind::Bare(source_slice(source, start, offset, line)?.to_owned()),
                    span: Span::new(start, offset),
                });
            }
        }
    }

    Ok(tokens)
}

fn lex_bare_end(source: &str, line: SourceLine, start: usize) -> Result<usize, ParseDiagnostic> {
    let mut end = start;
    for (relative, character) in source_slice(source, start, line.end, line)?.char_indices() {
        if matches!(
            character,
            ' ' | '\t' | '=' | '"' | '|' | '<' | '>' | ';' | '$' | '`'
        ) || character.is_control()
        {
            break;
        }
        end = start + relative + character.len_utf8();
    }
    Ok(end)
}

fn lex_quoted(
    source: &str,
    line: SourceLine,
    start: usize,
) -> Result<(String, usize), ParseDiagnostic> {
    let mut decoded = String::new();
    let mut offset = start + 1;

    while offset < line.end {
        let character = character_at(source, line, offset)?;
        match character {
            '"' => return Ok((decoded, offset + 1)),
            '\\' => {
                let escape_start = offset;
                offset += 1;
                if offset == line.end {
                    return Err(source_diagnostic(
                        source,
                        ParseDiagnosticKind::UnterminatedString,
                        Span::new(start, line.end),
                        line,
                    ));
                }
                let escape = character_at(source, line, offset)?;
                let value = match escape {
                    '"' => '"',
                    '\\' => '\\',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    _ => {
                        return Err(source_diagnostic(
                            source,
                            ParseDiagnosticKind::InvalidEscape { escape },
                            Span::new(escape_start, offset + escape.len_utf8()),
                            line,
                        ));
                    }
                };
                decoded.push(value);
                offset += escape.len_utf8();
            }
            character if character.is_control() => {
                return Err(source_diagnostic(
                    source,
                    ParseDiagnosticKind::UnexpectedCharacter { character },
                    Span::new(offset, offset + character.len_utf8()),
                    line,
                ));
            }
            _ => {
                decoded.push(character);
                offset += character.len_utf8();
            }
        }
    }

    Err(source_diagnostic(
        source,
        ParseDiagnosticKind::UnterminatedString,
        Span::new(start, line.end),
        line,
    ))
}

struct LineParser<'a> {
    source: &'a str,
    line: SourceLine,
    tokens: Vec<Token>,
    cursor: usize,
}

impl LineParser<'_> {
    fn parse_call(&mut self, max_arguments: usize) -> Result<Call, ParseDiagnostic> {
        let path_token = self.next_or_end(ParseDiagnosticKind::ExpectedNodePath)?;
        let path = match &path_token.kind {
            TokenKind::Bare(value) => NodePath::parse(value.clone()).map_err(|_| {
                self.at_token(
                    ParseDiagnosticKind::InvalidNodePath {
                        value: value.clone(),
                    },
                    &path_token,
                )
            })?,
            _ => return Err(self.at_token(ParseDiagnosticKind::ExpectedNodePath, &path_token)),
        };

        let method_token = self.next_or_end(ParseDiagnosticKind::ExpectedMethod)?;
        self.require_separation(&path_token, &method_token)?;
        let method = match &method_token.kind {
            TokenKind::Bare(value) => MethodName::new(value.clone()).map_err(|_| {
                self.at_token(
                    ParseDiagnosticKind::InvalidMethodName {
                        value: value.clone(),
                    },
                    &method_token,
                )
            })?,
            _ => return Err(self.at_token(ParseDiagnosticKind::ExpectedMethod, &method_token)),
        };

        let mut arguments = Vec::new();
        let mut previous_end = method_token.span.end;
        while let Some(next_span) = self.peek().map(|token| token.span) {
            if arguments.len() == max_arguments {
                return Err(self.at_span(ParseDiagnosticKind::TooManyArguments, next_span));
            }
            if next_span.start == previous_end {
                return Err(self.at_span(ParseDiagnosticKind::MissingWhitespace, next_span));
            }
            let argument = self.parse_argument()?;
            previous_end = argument.span().end;
            arguments.push(argument);
        }

        let end = arguments
            .last()
            .map(Argument::span)
            .map_or(method_token.span.end, |span| span.end);
        Ok(Call {
            path: Spanned::new(path, path_token.span),
            method: Spanned::new(method, method_token.span),
            arguments,
            span: Span::new(path_token.span.start, end),
        })
    }

    fn parse_argument(&mut self) -> Result<Argument, ParseDiagnostic> {
        let token = self.next_or_end(ParseDiagnosticKind::ExpectedArgument)?;
        match &token.kind {
            TokenKind::Quoted(value) => Ok(Argument::Positional(Spanned::new(
                parse_quoted_literal(value.clone()),
                token.span,
            ))),
            TokenKind::Equals => Err(self.at_token(ParseDiagnosticKind::ExpectedArgument, &token)),
            TokenKind::Bare(text) => {
                let text = text.clone();
                if let Some(equals_span) = self
                    .peek()
                    .filter(|next| matches!(next.kind, TokenKind::Equals))
                    .map(|equals| equals.span)
                {
                    if equals_span.start != token.span.end {
                        return Err(
                            self.at_span(ParseDiagnosticKind::WhitespaceAroundEquals, equals_span)
                        );
                    }
                    self.cursor += 1;
                    return self.parse_named_argument(&token, text, equals_span);
                }
                Ok(Argument::Positional(Spanned::new(
                    self.parse_bare_value(&text, token.span)?,
                    token.span,
                )))
            }
        }
    }

    fn parse_named_argument(
        &mut self,
        name_token: &Token,
        name_text: String,
        equals_span: Span,
    ) -> Result<Argument, ParseDiagnostic> {
        let name = ParameterName::new(name_text.clone()).map_err(|_| {
            self.at_token(
                ParseDiagnosticKind::InvalidParameterName { value: name_text },
                name_token,
            )
        })?;
        let Some(value_token) = self.next() else {
            return Err(self.at_span(
                ParseDiagnosticKind::MissingArgumentValue,
                Span::new(equals_span.end, equals_span.end),
            ));
        };
        if value_token.span.start != equals_span.end {
            return Err(self.at_token(ParseDiagnosticKind::WhitespaceAroundEquals, &value_token));
        }
        let value = match &value_token.kind {
            TokenKind::Bare(text) => self.parse_bare_value(text, value_token.span)?,
            TokenKind::Quoted(value) => parse_quoted_literal(value.clone()),
            TokenKind::Equals => {
                return Err(self.at_token(ParseDiagnosticKind::MissingArgumentValue, &value_token));
            }
        };
        Ok(Argument::Named {
            name: Spanned::new(name, name_token.span),
            value: Spanned::new(value, value_token.span),
            span: Span::new(name_token.span.start, value_token.span.end),
        })
    }

    fn parse_bare_value(&self, text: &str, span: Span) -> Result<Value, ParseDiagnostic> {
        parse_bare_literal(text).map_err(|error| match error {
            LiteralError::IntegerOutOfRange => self.at_span(
                ParseDiagnosticKind::IntegerOutOfRange {
                    value: text.to_owned(),
                },
                span,
            ),
        })
    }

    fn require_separation(&self, left: &Token, right: &Token) -> Result<(), ParseDiagnostic> {
        if left.span.end == right.span.start {
            Err(self.at_token(ParseDiagnosticKind::MissingWhitespace, right))
        } else {
            Ok(())
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.cursor)?.clone();
        self.cursor += 1;
        Some(token)
    }

    fn next_or_end(&mut self, kind: ParseDiagnosticKind) -> Result<Token, ParseDiagnostic> {
        let end = self.line.end;
        let line = self.line;
        let source = self.source;
        match self.next() {
            Some(token) => Ok(token),
            None => Err(source_diagnostic(source, kind, Span::new(end, end), line)),
        }
    }

    fn at_token(&self, kind: ParseDiagnosticKind, token: &Token) -> ParseDiagnostic {
        self.at_span(kind, token.span)
    }

    fn at_span(&self, kind: ParseDiagnosticKind, span: Span) -> ParseDiagnostic {
        source_diagnostic(self.source, kind, span, self.line)
    }
}

fn forbidden_diagnostic(
    source: &str,
    syntax: ForbiddenSyntax,
    start: usize,
    end: usize,
    line: SourceLine,
) -> ParseDiagnostic {
    source_diagnostic(
        source,
        ParseDiagnosticKind::ForbiddenSyntax(syntax),
        Span::new(start, end),
        line,
    )
}

fn character_at(source: &str, line: SourceLine, offset: usize) -> Result<char, ParseDiagnostic> {
    source
        .get(offset..line.end)
        .and_then(|text| text.chars().next())
        .ok_or_else(|| {
            source_diagnostic(
                source,
                ParseDiagnosticKind::InvalidSourceBoundary,
                Span::new(offset, offset),
                line,
            )
        })
}

fn source_slice(
    source: &str,
    start: usize,
    end: usize,
    line: SourceLine,
) -> Result<&str, ParseDiagnostic> {
    source.get(start..end).ok_or_else(|| {
        source_diagnostic(
            source,
            ParseDiagnosticKind::InvalidSourceBoundary,
            Span::new(start, end),
            line,
        )
    })
}

fn source_diagnostic(
    source: &str,
    kind: ParseDiagnosticKind,
    span: Span,
    line: SourceLine,
) -> ParseDiagnostic {
    let column = source
        .get(line.start..span.start)
        .map_or(1, |prefix| prefix.chars().count() + 1);
    diagnostic(kind, span, line.number, column)
}

fn diagnostic(
    kind: ParseDiagnosticKind,
    span: Span,
    line: usize,
    column: usize,
) -> ParseDiagnostic {
    ParseDiagnostic {
        kind,
        span,
        location: SourceLocation { line, column },
    }
}

struct DiagnosticMessage<'a>(&'a ParseDiagnosticKind);

impl fmt::Display for DiagnosticMessage<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            ParseDiagnosticKind::SourceTooLarge => formatter.write_str("source exceeds its limit"),
            ParseDiagnosticKind::InvalidSourceBoundary => {
                formatter.write_str("source contains an invalid character boundary")
            }
            ParseDiagnosticKind::TooManyCalls => {
                formatter.write_str("call count exceeds its limit")
            }
            ParseDiagnosticKind::TooManyArguments => {
                formatter.write_str("argument count exceeds its limit")
            }
            ParseDiagnosticKind::EmptyInput => formatter.write_str("expected at least one call"),
            ParseDiagnosticKind::EmptyStatement => {
                formatter.write_str("empty statements are not allowed")
            }
            ParseDiagnosticKind::ExpectedNodePath => formatter.write_str("expected a node path"),
            ParseDiagnosticKind::InvalidNodePath { value } => {
                write!(formatter, "invalid node path `{value}`")
            }
            ParseDiagnosticKind::ExpectedMethod => formatter.write_str("expected a method name"),
            ParseDiagnosticKind::InvalidMethodName { value } => {
                write!(formatter, "invalid method name `{value}`")
            }
            ParseDiagnosticKind::ExpectedArgument => formatter.write_str("expected an argument"),
            ParseDiagnosticKind::InvalidParameterName { value } => {
                write!(formatter, "invalid parameter name `{value}`")
            }
            ParseDiagnosticKind::MissingArgumentValue => {
                formatter.write_str("expected a value after `=`")
            }
            ParseDiagnosticKind::WhitespaceAroundEquals => {
                formatter.write_str("whitespace around `=` is not allowed")
            }
            ParseDiagnosticKind::MissingWhitespace => {
                formatter.write_str("expected whitespace between tokens")
            }
            ParseDiagnosticKind::UnterminatedString => formatter.write_str("unterminated string"),
            ParseDiagnosticKind::InvalidEscape { escape } => {
                write!(formatter, "unsupported escape sequence `\\{escape}`")
            }
            ParseDiagnosticKind::IntegerOutOfRange { value } => {
                write!(formatter, "integer `{value}` is outside the i64 range")
            }
            ParseDiagnosticKind::ForbiddenSyntax(syntax) => {
                write!(formatter, "{} is not supported", ForbiddenMessage(*syntax))
            }
            ParseDiagnosticKind::UnexpectedCharacter { character } => {
                write!(formatter, "unexpected character {character:?}")
            }
        }
    }
}

struct ForbiddenMessage(ForbiddenSyntax);

impl fmt::Display for ForbiddenMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.0 {
            ForbiddenSyntax::Pipe => "pipes",
            ForbiddenSyntax::Redirection => "redirection",
            ForbiddenSyntax::StatementSeparator => "semicolon statement separators",
            ForbiddenSyntax::VariableExpansion => "variable expansion",
            ForbiddenSyntax::CommandSubstitution => "command substitution",
        };
        formatter.write_str(message)
    }
}

#[cfg(test)]
#[path = "../tests/unit/parser.rs"]
mod tests;
