use std::collections::BTreeMap;

use crate::model::{MethodName, ParameterName};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    String(String),
    Integer(i64),
    Boolean(bool),
    List(Vec<Value>),
    Record(BTreeMap<String, Value>),
    Unit,
}

// Rejected untrusted drafts may contain trees deeper than the compiler limit.
impl Drop for Value {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        move_children(self, &mut pending);
        while let Some(mut value) = pending.pop() {
            move_children(&mut value, &mut pending);
        }
    }
}

fn move_children(value: &mut Value, pending: &mut Vec<Value>) {
    match value {
        Value::List(values) => pending.append(values),
        Value::Record(values) => {
            pending.extend(std::mem::take(values).into_values());
        }
        _ => {}
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueType {
    String,
    Integer,
    Boolean,
    Enum(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterSchema {
    pub name: ParameterName,
    pub value_type: ValueType,
    pub required: bool,
    pub positional: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodSchema {
    pub(crate) name: MethodName,
    pub(crate) parameters: Vec<ParameterSchema>,
}

impl MethodSchema {
    pub fn new(name: MethodName, parameters: Vec<ParameterSchema>) -> Result<Self, SchemaError> {
        let mut seen = BTreeMap::new();
        let mut optional_positional_seen = false;
        for parameter in &parameters {
            if seen.insert(parameter.name.clone(), ()).is_some() {
                return Err(SchemaError::DuplicateParameter(
                    parameter.name.as_str().to_owned(),
                ));
            }
            if parameter.positional {
                if optional_positional_seen && parameter.required {
                    return Err(SchemaError::RequiredAfterOptional(
                        parameter.name.as_str().to_owned(),
                    ));
                }
                if !parameter.required {
                    optional_positional_seen = true;
                }
            }
        }
        Ok(Self { name, parameters })
    }

    pub fn name(&self) -> &MethodName {
        &self.name
    }

    pub fn parameters(&self) -> &[ParameterSchema] {
        &self.parameters
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchemaError {
    DuplicateParameter(String),
    RequiredAfterOptional(String),
}

#[cfg(test)]
#[path = "../tests/unit/schema.rs"]
mod tests;
