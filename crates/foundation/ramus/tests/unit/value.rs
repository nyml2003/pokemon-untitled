use super::{LiteralError, parse_bare_literal, parse_quoted_literal};
use crate::schema::Value;

#[test]
fn classifies_bare_literals_without_guessing_quoted_values() {
    assert_eq!(parse_bare_literal("true"), Ok(Value::Boolean(true)));
    assert_eq!(parse_bare_literal("-42"), Ok(Value::Integer(-42)));
    assert_eq!(parse_bare_literal("-"), Ok(Value::String("-".to_owned())));
    assert_eq!(
        parse_bare_literal("12ms"),
        Ok(Value::String("12ms".to_owned()))
    );
    assert_eq!(
        parse_quoted_literal("true".to_owned()),
        Value::String("true".to_owned())
    );
}

#[test]
fn reports_integer_overflow_instead_of_reclassifying_it_as_text() {
    assert_eq!(
        parse_bare_literal("9223372036854775808"),
        Err(LiteralError::IntegerOutOfRange)
    );
}
