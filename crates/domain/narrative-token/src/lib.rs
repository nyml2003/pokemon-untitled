//! Pure streaming lexer primitives for the narrative DSL.

#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceSpan {
    start: usize,
    end: usize,
}

impl SourceSpan {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn start(self) -> usize {
        self.start
    }

    pub const fn end(self) -> usize {
        self.end
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceError {
    message: String,
}

impl SourceError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for SourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SourceError {}

pub type ByteChunk = Vec<u8>;

pub trait ByteStream {
    fn next_chunk(&mut self) -> Option<Result<ByteChunk, SourceError>>;
}

pub struct SliceByteStream<'a> {
    source: &'a [u8],
    offset: usize,
    chunk_size: usize,
}

impl<'a> SliceByteStream<'a> {
    pub const fn new(source: &'a [u8]) -> Self {
        Self {
            source,
            offset: 0,
            chunk_size: usize::MAX,
        }
    }

    pub const fn with_chunk_size(source: &'a [u8], chunk_size: usize) -> Self {
        Self {
            source,
            offset: 0,
            chunk_size: if chunk_size == 0 { 1 } else { chunk_size },
        }
    }
}

impl ByteStream for SliceByteStream<'_> {
    fn next_chunk(&mut self) -> Option<Result<ByteChunk, SourceError>> {
        if self.offset == self.source.len() {
            return None;
        }
        let end = self
            .offset
            .saturating_add(self.chunk_size)
            .min(self.source.len());
        let chunk = self.source[self.offset..end].to_vec();
        self.offset = end;
        Some(Ok(chunk))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Symbol {
    Hash,
    LeftBracket,
    RightBracket,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Semicolon,
    Colon,
    Arrow,
    Equal,
    GreaterThan,
    LessThan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceToken {
    namespace: String,
    name: String,
}

impl ResourceToken {
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn as_str(&self) -> String {
        format!("{}:{}", self.namespace, self.name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Identifier(String),
    Integer(u64),
    Boolean(bool),
    Resource(ResourceToken),
    Path(String),
    Variable(String),
    Symbol(Symbol),
    Eof,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    kind: TokenKind,
    span: SourceSpan,
}

impl Token {
    pub const fn new(kind: TokenKind, span: SourceSpan) -> Self {
        Self { kind, span }
    }

    pub fn kind(&self) -> &TokenKind {
        &self.kind
    }

    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LexErrorKind {
    Source(SourceError),
    NonAscii(u8),
    UnexpectedByte(u8),
    Expected(&'static str),
    InvalidInteger,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexError {
    kind: LexErrorKind,
    span: SourceSpan,
}

impl LexError {
    fn new(kind: LexErrorKind, span: SourceSpan) -> Self {
        Self { kind, span }
    }

    pub fn kind(&self) -> &LexErrorKind {
        &self.kind
    }

    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

impl fmt::Display for LexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            LexErrorKind::Source(error) => write!(formatter, "source error: {error}"),
            LexErrorKind::NonAscii(byte) => write!(formatter, "non-ASCII byte 0x{byte:02x}"),
            LexErrorKind::UnexpectedByte(byte) => write!(formatter, "unexpected byte 0x{byte:02x}"),
            LexErrorKind::Expected(expected) => write!(formatter, "expected {expected}"),
            LexErrorKind::InvalidInteger => formatter.write_str("invalid integer"),
        }
    }
}

impl std::error::Error for LexError {}

pub struct Lexer<S> {
    stream: S,
    pending: VecDeque<u8>,
    offset: usize,
    complete: bool,
}

impl<S: ByteStream> Lexer<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            pending: VecDeque::new(),
            offset: 0,
            complete: false,
        }
    }

    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_trivia()?;
        let start = self.offset;
        let Some(byte) = self.take_byte()? else {
            return Ok(Token::new(TokenKind::Eof, SourceSpan::new(start, start)));
        };

        let kind = match byte {
            byte if is_identifier_start(byte) => self.identifier_or_resource(byte)?,
            b'0'..=b'9' => self.integer(byte, start)?,
            b'/' => self.path(start)?,
            b'$' => self.variable(start)?,
            b'#' => TokenKind::Symbol(Symbol::Hash),
            b'[' => TokenKind::Symbol(Symbol::LeftBracket),
            b']' => TokenKind::Symbol(Symbol::RightBracket),
            b'(' => TokenKind::Symbol(Symbol::LeftParen),
            b')' => TokenKind::Symbol(Symbol::RightParen),
            b'{' => TokenKind::Symbol(Symbol::LeftBrace),
            b'}' => TokenKind::Symbol(Symbol::RightBrace),
            b',' => TokenKind::Symbol(Symbol::Comma),
            b';' => TokenKind::Symbol(Symbol::Semicolon),
            b':' => TokenKind::Symbol(Symbol::Colon),
            b'=' if self.peek_byte()? == Some(b'>') => {
                self.take_byte()?;
                TokenKind::Symbol(Symbol::Arrow)
            }
            b'=' if self.peek_byte()? == Some(b'=') => {
                self.take_byte()?;
                TokenKind::Symbol(Symbol::Equal)
            }
            b'>' => TokenKind::Symbol(Symbol::GreaterThan),
            b'<' => TokenKind::Symbol(Symbol::LessThan),
            byte => {
                return Err(LexError::new(
                    LexErrorKind::UnexpectedByte(byte),
                    SourceSpan::new(start, self.offset),
                ));
            }
        };

        Ok(Token::new(kind, SourceSpan::new(start, self.offset)))
    }

    fn skip_trivia(&mut self) -> Result<(), LexError> {
        loop {
            match (self.peek_byte()?, self.peek_nth(1)?) {
                (Some(b' ' | b'\t' | b'\r' | b'\n'), _) => {
                    self.take_byte()?;
                }
                (Some(b'/'), Some(b'/')) => {
                    self.take_byte()?;
                    self.take_byte()?;
                    while !matches!(self.peek_byte()?, None | Some(b'\n')) {
                        self.take_byte()?;
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn identifier_or_resource(&mut self, first: u8) -> Result<TokenKind, LexError> {
        let namespace = self.read_identifier(first)?;
        if self.peek_byte()? == Some(b':') && self.peek_nth(1)?.is_some_and(is_identifier_start) {
            self.take_byte()?;
            let name_start = self.take_byte()?.expect("peeked resource name exists");
            let name = self.read_identifier(name_start)?;
            return Ok(TokenKind::Resource(ResourceToken { namespace, name }));
        }
        if namespace == "true" {
            return Ok(TokenKind::Boolean(true));
        }
        if namespace == "false" {
            return Ok(TokenKind::Boolean(false));
        }
        Ok(TokenKind::Identifier(namespace))
    }

    fn integer(&mut self, first: u8, start: usize) -> Result<TokenKind, LexError> {
        let mut value = u64::from(first - b'0');
        if first == b'0' && self.peek_byte()?.is_some_and(|byte| byte.is_ascii_digit()) {
            while self.peek_byte()?.is_some_and(|byte| byte.is_ascii_digit()) {
                self.take_byte()?;
            }
            return Err(LexError::new(
                LexErrorKind::InvalidInteger,
                SourceSpan::new(start, self.offset),
            ));
        }
        while let Some(byte) = self.peek_byte()? {
            if !byte.is_ascii_digit() {
                break;
            }
            self.take_byte()?;
            value = value
                .checked_mul(10)
                .and_then(|value| value.checked_add(u64::from(byte - b'0')))
                .ok_or_else(|| {
                    LexError::new(
                        LexErrorKind::InvalidInteger,
                        SourceSpan::new(start, self.offset),
                    )
                })?;
        }
        Ok(TokenKind::Integer(value))
    }

    fn path(&mut self, start: usize) -> Result<TokenKind, LexError> {
        let Some(first) = self.take_byte()? else {
            return Err(LexError::new(
                LexErrorKind::Expected("path segment"),
                SourceSpan::new(start, self.offset),
            ));
        };
        if !is_identifier_start(first) {
            return Err(LexError::new(
                LexErrorKind::Expected("path segment"),
                SourceSpan::new(start, self.offset),
            ));
        }
        let mut path = format!("/{}", self.read_identifier(first)?);
        while self.peek_byte()? == Some(b'/') {
            self.take_byte()?;
            let Some(segment_start) = self.take_byte()? else {
                return Err(LexError::new(
                    LexErrorKind::Expected("path segment"),
                    SourceSpan::new(start, self.offset),
                ));
            };
            if !is_identifier_start(segment_start) {
                return Err(LexError::new(
                    LexErrorKind::Expected("path segment"),
                    SourceSpan::new(start, self.offset),
                ));
            }
            path.push('/');
            path.push_str(&self.read_identifier(segment_start)?);
        }
        Ok(TokenKind::Path(path))
    }

    fn variable(&mut self, start: usize) -> Result<TokenKind, LexError> {
        if self.take_byte()? != Some(b'{') {
            return Err(LexError::new(
                LexErrorKind::Expected("'{' after '$'"),
                SourceSpan::new(start, self.offset),
            ));
        }
        let Some(first) = self.take_byte()? else {
            return Err(LexError::new(
                LexErrorKind::Expected("variable name"),
                SourceSpan::new(start, self.offset),
            ));
        };
        if !is_identifier_start(first) {
            return Err(LexError::new(
                LexErrorKind::Expected("variable name"),
                SourceSpan::new(start, self.offset),
            ));
        }
        let name = self.read_identifier(first)?;
        if self.take_byte()? != Some(b'}') {
            return Err(LexError::new(
                LexErrorKind::Expected("'}' after variable name"),
                SourceSpan::new(start, self.offset),
            ));
        }
        Ok(TokenKind::Variable(name))
    }

    fn read_identifier(&mut self, first: u8) -> Result<String, LexError> {
        let mut identifier = String::from(char::from(first));
        while self.peek_byte()?.is_some_and(is_identifier_continue) {
            identifier.push(char::from(
                self.take_byte()?.expect("peeked identifier byte exists"),
            ));
        }
        Ok(identifier)
    }

    fn peek_byte(&mut self) -> Result<Option<u8>, LexError> {
        self.peek_nth(0)
    }

    fn peek_nth(&mut self, index: usize) -> Result<Option<u8>, LexError> {
        while self.pending.len() <= index && !self.complete {
            match self.stream.next_chunk() {
                Some(Ok(chunk)) => self.pending.extend(chunk),
                Some(Err(error)) => {
                    return Err(LexError::new(
                        LexErrorKind::Source(error),
                        SourceSpan::new(self.offset, self.offset),
                    ));
                }
                None => self.complete = true,
            }
        }
        Ok(self.pending.get(index).copied())
    }

    fn take_byte(&mut self) -> Result<Option<u8>, LexError> {
        let Some(byte) = self.peek_byte()? else {
            return Ok(None);
        };
        if !byte.is_ascii() {
            return Err(LexError::new(
                LexErrorKind::NonAscii(byte),
                SourceSpan::new(self.offset, self.offset + 1),
            ));
        }
        self.pending.pop_front();
        self.offset += 1;
        Ok(Some(byte))
    }
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_identifier_continue(byte: u8) -> bool {
    is_identifier_start(byte) || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(source: &[u8], chunk_size: usize) -> Result<Vec<Token>, LexError> {
        let mut lexer = Lexer::new(SliceByteStream::with_chunk_size(source, chunk_size));
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token()?;
            let done = matches!(token.kind(), TokenKind::Eof);
            tokens.push(token);
            if done {
                return Ok(tokens);
            }
        }
    }

    #[test]
    fn lexes_every_first_version_token_across_single_byte_chunks() {
        let actual = tokens(
            b"#[script] script patrol() { say(text: text:hello); wait(event: event:clear); end(); /world/actor face(target: ${actor}, amount: 42, ok: true); => script:next; }",
            1,
        )
        .unwrap();
        assert_eq!(actual[0].kind(), &TokenKind::Symbol(Symbol::Hash));
        assert_eq!(actual[2].kind(), &TokenKind::Identifier("script".into()));
        assert!(actual.iter().any(|token| {
            token.kind()
                == &TokenKind::Resource(ResourceToken {
                    namespace: "text".into(),
                    name: "hello".into(),
                })
        }));
        assert!(
            actual
                .iter()
                .any(|token| token.kind() == &TokenKind::Path("/world/actor".into()))
        );
        assert!(
            actual
                .iter()
                .any(|token| token.kind() == &TokenKind::Variable("actor".into()))
        );
        assert!(
            actual
                .iter()
                .any(|token| token.kind() == &TokenKind::Integer(42))
        );
        assert!(
            actual
                .iter()
                .any(|token| token.kind() == &TokenKind::Boolean(true))
        );
        assert!(matches!(actual.last().unwrap().kind(), TokenKind::Eof));
        assert_eq!(actual[0].span(), SourceSpan::new(0, 1));
    }

    #[test]
    fn trivia_and_resource_boundaries_are_preserved() {
        let mut whole = SliceByteStream::new(b"ab");
        assert_eq!(whole.next_chunk(), Some(Ok(b"ab".to_vec())));
        assert_eq!(whole.next_chunk(), None);

        let actual = tokens(b"// comment\nid: actor:guard false true", 0).unwrap();
        assert_eq!(actual[0].kind(), &TokenKind::Identifier("id".into()));
        assert_eq!(actual[1].kind(), &TokenKind::Symbol(Symbol::Colon));
        assert_eq!(
            actual[2].kind(),
            &TokenKind::Resource(ResourceToken {
                namespace: "actor".into(),
                name: "guard".into(),
            })
        );
        assert_eq!(actual[3].kind(), &TokenKind::Boolean(false));
        assert_eq!(actual[4].kind(), &TokenKind::Boolean(true));
        assert_eq!(
            actual[5].span().start(),
            b"// comment\nid: actor:guard false true".len()
        );
        assert_eq!(actual[5].span().end(), actual[5].span().start());
    }

    #[test]
    fn lexer_reports_invalid_input_with_spans_and_messages() {
        let cases = [
            (
                b"\xff".as_slice(),
                LexErrorKind::NonAscii(0xff),
                "non-ASCII byte 0xff",
            ),
            (
                b"@".as_slice(),
                LexErrorKind::UnexpectedByte(b'@'),
                "unexpected byte 0x40",
            ),
            (
                b"01".as_slice(),
                LexErrorKind::InvalidInteger,
                "invalid integer",
            ),
            (
                b"/".as_slice(),
                LexErrorKind::Expected("path segment"),
                "expected path segment",
            ),
            (
                b"${".as_slice(),
                LexErrorKind::Expected("variable name"),
                "expected variable name",
            ),
        ];
        for (source, expected, message) in cases {
            let error = tokens(source, 1).unwrap_err();
            assert_eq!(error.kind(), &expected);
            assert_eq!(error.to_string(), message);
            assert!(error.span().end() >= error.span().start());
        }
        for source in [
            b"/9".as_slice(),
            b"/abc/",
            b"/abc/9",
            b"$name",
            b"${9}",
            b"${name",
            b"18446744073709551616",
        ] {
            assert!(tokens(source, 1).is_err());
        }
        assert_eq!(
            tokens(b"==", 1).unwrap()[0].kind(),
            &TokenKind::Symbol(Symbol::Equal)
        );
        let comparisons = tokens(b"><", 1).unwrap();
        assert_eq!(
            comparisons[0].kind(),
            &TokenKind::Symbol(Symbol::GreaterThan)
        );
        assert_eq!(comparisons[1].kind(), &TokenKind::Symbol(Symbol::LessThan));
    }

    #[test]
    fn lexer_propagates_source_failures() {
        struct FailingStream;

        impl ByteStream for FailingStream {
            fn next_chunk(&mut self) -> Option<Result<ByteChunk, SourceError>> {
                Some(Err(SourceError::new("read failed")))
            }
        }

        let mut lexer = Lexer::new(FailingStream);
        let error = lexer.next_token().unwrap_err();
        assert_eq!(
            error.kind(),
            &LexErrorKind::Source(SourceError::new("read failed"))
        );
        assert_eq!(error.to_string(), "source error: read failed");
        assert_eq!(SourceError::new("x").message(), "x");
    }
}
