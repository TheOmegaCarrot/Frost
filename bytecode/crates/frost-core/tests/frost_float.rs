use frost_core::{FrostError, FrostFloat};

#[test]
fn accepts_normal_values() {
    assert!(FrostFloat::new(0.0).is_ok());
    assert!(FrostFloat::new(1.5).is_ok());
    assert!(FrostFloat::new(-42.0).is_ok());
    assert!(FrostFloat::new(1e308).is_ok());
    assert!(FrostFloat::new(5e-324).is_ok());
}

#[test]
fn rejects_nan() {
    assert!(FrostFloat::new(f64::NAN).is_err());
}

#[test]
fn rejects_infinity() {
    assert!(FrostFloat::new(f64::INFINITY).is_err());
    assert!(FrostFloat::new(f64::NEG_INFINITY).is_err());
}

#[test]
fn accepts_negative_zero() {
    assert!(FrostFloat::new(-0.0).is_ok());
}

#[test]
fn deref_gives_inner_f64() {
    let f = FrostFloat::new(3.14).unwrap();
    assert_eq!(*f, 3.14);
}

#[test]
fn equality() {
    let a = FrostFloat::new(1.5).unwrap();
    let b = FrostFloat::new(1.5).unwrap();
    let c = FrostFloat::new(2.5).unwrap();
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn negative_zero_equals_positive_zero() {
    let pos = FrostFloat::new(0.0).unwrap();
    let neg = FrostFloat::new(-0.0).unwrap();
    assert_eq!(pos, neg);
}

#[test]
fn negative_zero_ordering_equals_positive_zero() {
    let pos = FrostFloat::new(0.0).unwrap();
    let neg = FrostFloat::new(-0.0).unwrap();
    assert!(pos == neg);
    assert!(!(pos < neg));
    assert!(!(neg < pos));
}

#[test]
fn ordering() {
    let a = FrostFloat::new(-1.0).unwrap();
    let b = FrostFloat::new(0.0).unwrap();
    let c = FrostFloat::new(1.0).unwrap();
    assert!(a < b);
    assert!(b < c);
    assert!(a < c);
}

#[test]
fn deref_enables_f64_methods() {
    let f = FrostFloat::new(-2.5).unwrap();
    assert_eq!(f.abs(), 2.5);
    assert_eq!(f.floor(), -3.0);
}

#[test]
fn try_from_f64() {
    let ok: Result<FrostFloat, _> = 3.14.try_into();
    assert!(ok.is_ok());

    let nan: Result<FrostFloat, _> = f64::NAN.try_into();
    assert!(nan.is_err());
}

#[test]
fn rejection_is_recoverable_error() {
    let err = FrostFloat::new(f64::NAN).unwrap_err();
    assert!(matches!(err, FrostError::Recoverable(_)));
}
