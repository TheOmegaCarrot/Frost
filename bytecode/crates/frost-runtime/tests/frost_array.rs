use frost_runtime::{FrostArray, Value};

fn sample_array() -> FrostArray {
    FrostArray::new(&[Value::from(10i64), Value::from(20i64), Value::from(30i64)])
}

// -- Construction --

#[test]
fn new_from_slice() {
    let arr = FrostArray::new(&[Value::from(1i64), Value::from(2i64)]);
    assert_eq!(arr.len(), 2);
}

#[test]
fn from_vec() {
    let arr = FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]);
    assert_eq!(arr.len(), 2);
}

#[test]
fn from_slice_ref() {
    let elems = [Value::from(1i64)];
    let arr = FrostArray::from(elems.as_slice());
    assert_eq!(arr.len(), 1);
}

#[test]
fn collect_from_iterator() {
    let arr: FrostArray = (0..5).map(|i| Value::from(i as i64)).collect();
    assert_eq!(arr.len(), 5);
}

#[test]
fn empty() {
    let arr = FrostArray::new(&[]);
    assert!(arr.is_empty());
    assert_eq!(arr.len(), 0);
}

// -- Indexing --

#[test]
fn index_by_usize() {
    let arr = sample_array();
    assert!(matches!(arr[0], Value::Int(10)));
    assert!(matches!(arr[2], Value::Int(30)));
}

#[test]
#[should_panic]
fn index_out_of_bounds_panics() {
    let arr = sample_array();
    let _ = &arr[3];
}

// -- Frost-semantic indexing --

#[test]
fn frost_get_positive() {
    let arr = sample_array();
    assert!(matches!(arr.frost_get(0), Some(Value::Int(10))));
    assert!(matches!(arr.frost_get(2), Some(Value::Int(30))));
}

#[test]
fn frost_get_negative() {
    let arr = sample_array();
    assert!(matches!(arr.frost_get(-1), Some(Value::Int(30))));
    assert!(matches!(arr.frost_get(-3), Some(Value::Int(10))));
}

#[test]
fn frost_get_out_of_bounds() {
    let arr = sample_array();
    assert!(arr.frost_get(3).is_none());
    assert!(arr.frost_get(100).is_none());
    assert!(arr.frost_get(-4).is_none());
    assert!(arr.frost_get(-100).is_none());
}

#[test]
fn frost_get_empty_array() {
    let arr = FrostArray::new(&[]);
    assert!(arr.frost_get(0).is_none());
    assert!(arr.frost_get(-1).is_none());
}

// -- Iteration --

#[test]
fn iterate_by_ref() {
    let arr = sample_array();
    let values: Vec<_> = arr.iter().collect();
    assert_eq!(values.len(), 3);
}

#[test]
fn for_loop_borrows() {
    let arr = sample_array();
    let mut count = 0;
    for _ in &arr {
        count += 1;
    }
    assert_eq!(count, 3);
    // arr is still usable after the loop
    assert_eq!(arr.len(), 3);
}

// -- Slicing --

#[test]
fn as_slice_full() {
    let arr = sample_array();
    let s = arr.as_slice();
    assert_eq!(s.len(), 3);
}

#[test]
fn as_slice_subslice() {
    let arr = sample_array();
    let tail = &arr.as_slice()[1..];
    assert_eq!(tail.len(), 2);
    assert!(matches!(tail[0], Value::Int(20)));
}

#[test]
fn as_slice_to_new_array() {
    let arr = sample_array();
    let tail = FrostArray::new(&arr.as_slice()[1..]);
    assert_eq!(tail.len(), 2);
    assert!(matches!(tail.frost_get(0), Some(Value::Int(20))));
}
