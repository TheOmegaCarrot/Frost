use frost_runtime::FrostError;

#[test]
fn display() {
    let err = FrostError::from_static("division by zero");
    assert_eq!(err.to_string(), "Error: division by zero");
}

#[test]
fn from_str() {
    let err: FrostError = "something went wrong".into();
    assert_eq!(err.message(), "something went wrong");
    assert!(err.backtrace().is_empty());
}

#[test]
fn from_string() {
    let err: FrostError = String::from("bad input").into();
    assert_eq!(err.message(), "bad input");
}
