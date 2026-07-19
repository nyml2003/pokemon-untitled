//! Pure streaming compiler for the first narrative DSL slice.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use narrative_cps::{
    ActorId, ContinuationId, CpsNode, EventId, ScriptDirection, ScriptId, ScriptProgram, TextId,
};
use narrative_token::{
    ByteStream, LexError, Lexer, ResourceToken, SourceSpan, Symbol, Token, TokenKind,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticCode {
    Lexical,
    Expected,
    InvalidResource,
    InvalidDirection,
    UnsupportedStatement,
    StatementAfterEnd,
    MissingEnd,
    InvalidProgram,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    code: DiagnosticCode,
    span: SourceSpan,
    message: String,
}

impl Diagnostic {
    fn new(code: DiagnosticCode, span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            code,
            span,
            message: message.into(),
        }
    }

    pub const fn code(&self) -> DiagnosticCode {
        self.code
    }

    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileOutcome {
    program: Option<ScriptProgram>,
    diagnostics: Vec<Diagnostic>,
}

impl CompileOutcome {
    pub fn program(&self) -> Option<&ScriptProgram> {
        self.program.as_ref()
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn is_success(&self) -> bool {
        self.program.is_some() && self.diagnostics.is_empty()
    }
}

pub struct Compiler;

impl Compiler {
    pub fn compile<S: ByteStream>(stream: S) -> CompileOutcome {
        match Parser::new(stream).parse_script() {
            Ok(program) => CompileOutcome {
                program: Some(program),
                diagnostics: Vec::new(),
            },
            Err(diagnostic) => CompileOutcome {
                program: None,
                diagnostics: vec![diagnostic],
            },
        }
    }
}

enum ParsedStatement {
    Move(ScriptDirection),
    Face(ScriptDirection),
    Say(TextId),
    Wait(EventId),
    End,
}

struct Parser<S> {
    lexer: Lexer<S>,
    lookahead: Option<Token>,
}

impl<S: ByteStream> Parser<S> {
    fn new(stream: S) -> Self {
        Self {
            lexer: Lexer::new(stream),
            lookahead: None,
        }
    }

    fn parse_script(mut self) -> Result<ScriptProgram, Diagnostic> {
        self.expect_keyword("script")?;
        let (name, span) = self.expect_identifier()?;
        let id = ScriptId::new(format!("script:{name}"))
            .map_err(|error| Diagnostic::new(DiagnosticCode::Expected, span, error.to_string()))?;
        self.expect_symbol(Symbol::LeftParen, "'('")?;
        let actor = if self.check_symbol(Symbol::RightParen)? {
            None
        } else {
            Some(self.parse_named_actor()?)
        };
        self.expect_symbol(Symbol::RightParen, "')'")?;
        self.expect_symbol(Symbol::LeftBrace, "'{'")?;

        let mut statements = Vec::new();
        let mut saw_end = false;
        while !self.check_symbol(Symbol::RightBrace)? {
            let statement_token = self.peek()?;
            if matches!(statement_token.kind(), TokenKind::Eof) {
                return Err(Diagnostic::new(
                    DiagnosticCode::Expected,
                    statement_token.span(),
                    "expected '}'",
                ));
            }
            if saw_end {
                return Err(Diagnostic::new(
                    DiagnosticCode::StatementAfterEnd,
                    statement_token.span(),
                    "statement follows end",
                ));
            }
            let statement = self.parse_statement()?;
            saw_end = matches!(statement, ParsedStatement::End);
            statements.push(statement);
        }
        self.expect_symbol(Symbol::RightBrace, "'}'")?;
        self.expect_eof()?;
        if !saw_end {
            return Err(Diagnostic::new(
                DiagnosticCode::MissingEnd,
                span,
                "script must end with end();",
            ));
        }
        build_program(id, actor, statements, span)
    }

    fn parse_statement(&mut self) -> Result<ParsedStatement, Diagnostic> {
        let (name, span) = self.expect_identifier()?;
        match name.as_str() {
            "move" => Ok(ParsedStatement::Move(self.parse_named_direction()?)),
            "face" => Ok(ParsedStatement::Face(self.parse_named_direction()?)),
            "say" => {
                let (resource, span) = self.parse_named_resource("text", "text")?;
                TextId::new(resource.as_str())
                    .map(ParsedStatement::Say)
                    .map_err(|error| {
                        Diagnostic::new(DiagnosticCode::InvalidResource, span, error.to_string())
                    })
            }
            "wait" => {
                let (resource, span) = self.parse_named_resource("event", "event")?;
                EventId::new(resource.as_str())
                    .map(ParsedStatement::Wait)
                    .map_err(|error| {
                        Diagnostic::new(DiagnosticCode::InvalidResource, span, error.to_string())
                    })
            }
            "end" => {
                self.expect_symbol(Symbol::LeftParen, "'('")?;
                self.expect_symbol(Symbol::RightParen, "')'")?;
                self.expect_symbol(Symbol::Semicolon, "';'")?;
                Ok(ParsedStatement::End)
            }
            _ => Err(Diagnostic::new(
                DiagnosticCode::UnsupportedStatement,
                span,
                format!("unsupported statement '{name}'"),
            )),
        }
    }

    fn parse_named_actor(&mut self) -> Result<ActorId, Diagnostic> {
        self.expect_keyword("actor")?;
        self.expect_symbol(Symbol::Colon, "':'")?;
        let (resource, span) = self.expect_resource("actor")?;
        ActorId::new(resource.as_str()).map_err(|error| {
            Diagnostic::new(DiagnosticCode::InvalidResource, span, error.to_string())
        })
    }

    fn parse_named_direction(&mut self) -> Result<ScriptDirection, Diagnostic> {
        self.expect_symbol(Symbol::LeftParen, "'('")?;
        self.expect_keyword("direction")?;
        self.expect_symbol(Symbol::Colon, "':'")?;
        let (direction, span) = self.expect_identifier()?;
        self.expect_symbol(Symbol::RightParen, "')'")?;
        self.expect_symbol(Symbol::Semicolon, "';'")?;
        ScriptDirection::from_keyword(&direction).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::InvalidDirection,
                span,
                format!("invalid direction '{direction}'"),
            )
        })
    }

    fn parse_named_resource(
        &mut self,
        parameter: &'static str,
        namespace: &'static str,
    ) -> Result<(ResourceToken, SourceSpan), Diagnostic> {
        self.expect_symbol(Symbol::LeftParen, "'('")?;
        self.expect_keyword(parameter)?;
        self.expect_symbol(Symbol::Colon, "':'")?;
        let resource = self.expect_resource(namespace)?;
        self.expect_symbol(Symbol::RightParen, "')'")?;
        self.expect_symbol(Symbol::Semicolon, "';'")?;
        Ok(resource)
    }

    fn expect_keyword(&mut self, expected: &str) -> Result<(), Diagnostic> {
        let token = self.next()?;
        if matches!(token.kind(), TokenKind::Identifier(value) if value == expected) {
            return Ok(());
        }
        Err(expected_token(token.span(), expected))
    }

    fn expect_identifier(&mut self) -> Result<(String, SourceSpan), Diagnostic> {
        let token = self.next()?;
        match token.kind() {
            TokenKind::Identifier(value) => Ok((value.clone(), token.span())),
            _ => Err(expected_token(token.span(), "identifier")),
        }
    }

    fn expect_resource(
        &mut self,
        expected_namespace: &str,
    ) -> Result<(ResourceToken, SourceSpan), Diagnostic> {
        let token = self.next()?;
        match token.kind() {
            TokenKind::Resource(resource) if resource.namespace() == expected_namespace => {
                Ok((resource.clone(), token.span()))
            }
            TokenKind::Resource(resource) => Err(Diagnostic::new(
                DiagnosticCode::InvalidResource,
                token.span(),
                format!(
                    "expected {expected_namespace}: resource, got {}:",
                    resource.namespace()
                ),
            )),
            _ => Err(expected_token(token.span(), "resource ID")),
        }
    }

    fn expect_symbol(&mut self, expected: Symbol, expected_name: &str) -> Result<(), Diagnostic> {
        let token = self.next()?;
        if token.kind() == &TokenKind::Symbol(expected) {
            return Ok(());
        }
        Err(expected_token(token.span(), expected_name))
    }

    fn expect_eof(&mut self) -> Result<(), Diagnostic> {
        let token = self.next()?;
        if matches!(token.kind(), TokenKind::Eof) {
            return Ok(());
        }
        Err(expected_token(token.span(), "end of file"))
    }

    fn check_symbol(&mut self, expected: Symbol) -> Result<bool, Diagnostic> {
        Ok(self.peek()?.kind() == &TokenKind::Symbol(expected))
    }

    fn peek(&mut self) -> Result<Token, Diagnostic> {
        if let Some(token) = &self.lookahead {
            return Ok(token.clone());
        }
        let token = self.lex()?;
        self.lookahead = Some(token.clone());
        Ok(token)
    }

    fn next(&mut self) -> Result<Token, Diagnostic> {
        if let Some(token) = self.lookahead.take() {
            return Ok(token);
        }
        self.lex()
    }

    fn lex(&mut self) -> Result<Token, Diagnostic> {
        self.lexer.next_token().map_err(lexical_diagnostic)
    }
}

fn build_program(
    id: ScriptId,
    actor: Option<ActorId>,
    statements: Vec<ParsedStatement>,
    span: SourceSpan,
) -> Result<ScriptProgram, Diagnostic> {
    let mut continuations = BTreeMap::new();
    for (index, statement) in statements.into_iter().enumerate() {
        let id = ContinuationId::new(index as u32);
        let node = match statement {
            ParsedStatement::Move(direction) => CpsNode::Move {
                direction,
                next: ContinuationId::new((index + 1) as u32),
            },
            ParsedStatement::Face(direction) => CpsNode::Face {
                direction,
                next: ContinuationId::new((index + 1) as u32),
            },
            ParsedStatement::Say(text) => CpsNode::Say {
                text,
                next: ContinuationId::new((index + 1) as u32),
            },
            ParsedStatement::Wait(event) => CpsNode::Wait {
                event,
                resume: ContinuationId::new((index + 1) as u32),
            },
            ParsedStatement::End => CpsNode::End,
        };
        continuations.insert(id, node);
    }
    ScriptProgram::with_actor(id, actor, ContinuationId::new(0), continuations)
        .map_err(|error| Diagnostic::new(DiagnosticCode::InvalidProgram, span, error.to_string()))
}

fn expected_token(span: SourceSpan, expected: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::Expected,
        span,
        format!("expected {expected}"),
    )
}

fn lexical_diagnostic(error: LexError) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::Lexical, error.span(), error.to_string())
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
