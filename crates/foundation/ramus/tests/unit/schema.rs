use super::{MethodSchema, ParameterSchema, ValueType};
use crate::model::{MethodName, ParameterName};

#[test]
fn multiple_optional_positionals_preserve_the_ordering_rule() {
    let optional = |name| ParameterSchema {
        name: ParameterName::new(name).unwrap(),
        value_type: ValueType::String,
        required: false,
        positional: true,
    };

    assert!(
        MethodSchema::new(
            MethodName::new("run").unwrap(),
            vec![optional("first"), optional("second")],
        )
        .is_ok()
    );

    assert!(
        MethodSchema::new(
            MethodName::new("required").unwrap(),
            vec![ParameterSchema {
                name: ParameterName::new("value").unwrap(),
                value_type: ValueType::String,
                required: true,
                positional: true,
            }],
        )
        .is_ok()
    );
}
