use frost_core::FrostError;

#[test]
fn recoverable_display() {
    let err = FrostError::Recoverable("division by zero".into());
    assert_eq!(err.to_string(), "Error: division by zero");
}

#[test]
fn unrecoverable_display() {
    let err = FrostError::Unrecoverable("cannot bind to $1".into());
    assert_eq!(err.to_string(), "Error: cannot bind to $1");
}

#[test]
fn internal_display() {
    let err = FrostError::Internal("unreachable code reached".into());
    assert_eq!(err.to_string(), "INTERNAL ERROR: unreachable code reached");
}
