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
