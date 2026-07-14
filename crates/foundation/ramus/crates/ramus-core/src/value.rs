use crate::schema::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LiteralError {
    IntegerOutOfRange,
}

pub(crate) fn parse_bare_literal(source: &str) -> Result<Value, LiteralError> {
    match source {
        "true" => Ok(Value::Boolean(true)),
        "false" => Ok(Value::Boolean(false)),
        _ if is_integer_literal(source) => source
            .parse::<i64>()
            .map(Value::Integer)
            .map_err(|_| LiteralError::IntegerOutOfRange),
        _ => Ok(Value::String(source.to_owned())),
    }
}

pub(crate) fn parse_quoted_literal(decoded: String) -> Value {
    Value::String(decoded)
}

fn is_integer_literal(source: &str) -> bool {
    let digits = source.strip_prefix('-').unwrap_or(source);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
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
}
