use std::cmp::Ordering;

use frostlang::FrostFloat;

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
fn get_returns_inner_f64() {
    let f = FrostFloat::new(3.14).unwrap();
    assert_eq!(f.get(), 3.14);
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
fn negative_zero_does_not_equal_positive_zero() {
    let pos = FrostFloat::new(0.0).unwrap();
    let neg = FrostFloat::new(-0.0).unwrap();
    assert_ne!(pos, neg);
    assert_eq!(neg, FrostFloat::new(-0.0).unwrap());
}

#[test]
fn negative_zero_orders_just_before_positive_zero() {
    let pos = FrostFloat::new(0.0).unwrap();
    let neg = FrostFloat::new(-0.0).unwrap();
    // The Floats nearest zero: the smallest subnormals.
    let below = FrostFloat::new(-5e-324).unwrap();
    let above = FrostFloat::new(5e-324).unwrap();
    assert_eq!(neg.cmp(&pos), Ordering::Less);
    assert_eq!(pos.cmp(&neg), Ordering::Greater);
    assert_eq!(below.cmp(&neg), Ordering::Less);
    assert_eq!(above.cmp(&pos), Ordering::Greater);
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
fn try_from_f64() {
    let ok: Result<FrostFloat, _> = 3.14.try_into();
    assert!(ok.is_ok());

    let nan: Result<FrostFloat, _> = f64::NAN.try_into();
    assert!(nan.is_err());
}

#[test]
fn rejection_produces_error() {
    let err = FrostFloat::new(f64::NAN).unwrap_err();
    assert!(!err.message().is_empty());
}
