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
#[path = "../tests/unit/value.rs"]
mod tests;
