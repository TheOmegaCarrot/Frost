use frost_runtime::Value;

#[test]
fn null() {
    assert_eq!(Value::Null.type_name(), "Null");
}

#[test]
fn bool() {
    assert_eq!(Value::from(true).type_name(), "Bool");
}

#[test]
fn int() {
    assert_eq!(Value::from(42i64).type_name(), "Int");
}

#[test]
fn float() {
    let v: Value = 3.14.try_into().unwrap();
    assert_eq!(v.type_name(), "Float");
}

#[test]
fn string() {
    assert_eq!(Value::from("hi").type_name(), "String");
}
