//! Tests for `TypeSet`: building sets in const context and at runtime, the
//! operators, and the queries. Membership of the named categories is covered by
//! `value_type_category`.

use frostlang::{FrostType, Param, Params, TypeSet};

type Ft = FrostType;

// ============================================================
// Const construction
// ============================================================

#[test]
fn or_chains_from_a_type_and_from_a_set() {
    const FROM_TYPE: TypeSet = Ft::Int.or(Ft::Float);
    const FROM_SET: TypeSet = Ft::INT.or(Ft::Float);
    const CHAINED: TypeSet = Ft::String.or(Ft::Bytes).or(Ft::Array);

    assert_eq!(FROM_TYPE, Ft::Int | Ft::Float);
    assert_eq!(FROM_SET, Ft::Int | Ft::Float);
    assert_eq!(CHAINED, Ft::String | Ft::Bytes | Ft::Array);
}

#[test]
fn or_with_a_type_already_present_changes_nothing() {
    const SAME: TypeSet = Ft::NUMERIC.or(Ft::Int);
    assert_eq!(SAME, Ft::NUMERIC);
}

#[test]
fn union_combines_sets() {
    const NUMBER_OR_NULL: TypeSet = Ft::NUMERIC.union(Ft::NULL);
    const OVERLAPPING: TypeSet = Ft::PRIMITIVE.union(Ft::FLAT);

    assert_eq!(NUMBER_OR_NULL, Ft::Int | Ft::Float | Ft::Null);
    assert_eq!(OVERLAPPING, Ft::PRIMITIVE);
    assert_eq!(TypeSet::EMPTY.union(Ft::MAP), Ft::MAP);
}

// Checked at compile time: a wrong answer fails the build.
const _: () = assert!(TypeSet::EMPTY.is_empty(), "EMPTY is empty");
const _: () = assert!(!Ft::NULL.is_empty(), "NULL is not empty");

#[test]
fn a_set_built_inline_in_a_params_literal_compiles() {
    // The point is that this compiles: a set built by const calls is usable
    // directly in the `&[...]` literal, without its own `const` first.
    const PARAMS: Params = Params::new(&[
        Param::of(Ft::Int.or(Ft::Float)),
        Param::of(Ft::FLAT.union(Ft::STRUCTURED)).named("collection"),
    ]);

    let types: Vec<TypeSet> = PARAMS.as_slice().iter().map(Param::types).collect();
    assert_eq!(
        types,
        [
            Ft::Int | Ft::Float,
            Ft::String | Ft::Bytes | Ft::Array | Ft::Map
        ]
    );
}

// ============================================================
// Operators
// ============================================================

#[test]
fn bitor_combines_any_mix_of_types_and_sets() {
    let expected = Ft::NUMERIC.or(Ft::Null);

    assert_eq!(Ft::Int | Ft::Float | Ft::Null, expected, "type | type");
    assert_eq!(Ft::Null | Ft::NUMERIC, expected, "type | set");
    assert_eq!(Ft::NUMERIC | Ft::Null, expected, "set | type");
    assert_eq!(Ft::NUMERIC | Ft::NULL, expected, "set | set");
}

#[test]
fn bitand_intersects() {
    assert_eq!(Ft::PRIMITIVE & Ft::NUMERIC, Ft::NUMERIC);
    assert_eq!(Ft::PRIMITIVE & Ft::Int, Ft::INT);
    assert_eq!(Ft::FLAT & Ft::STRUCTURED, TypeSet::EMPTY);
}

#[test]
fn sub_removes() {
    assert_eq!(Ft::NUMERIC - Ft::Float, Ft::INT);
    assert_eq!(
        Ft::PRIMITIVE - Ft::FLAT,
        Ft::Null | Ft::Bool | Ft::Int | Ft::Float
    );
    assert_eq!(
        Ft::NUMERIC - Ft::Map,
        Ft::NUMERIC,
        "removing an absent type"
    );
}

// ============================================================
// Conversions and queries
// ============================================================

#[test]
fn from_a_type_is_the_set_of_only_it() {
    assert_eq!(TypeSet::from(Ft::Opaque), Ft::OPAQUE);
}

#[test]
fn collects_from_types() {
    let collected: TypeSet = [Ft::Map, Ft::Array, Ft::Map].into_iter().collect();
    assert_eq!(collected, Ft::STRUCTURED);
}

#[test]
fn default_is_empty() {
    assert_eq!(TypeSet::default(), TypeSet::EMPTY);
}

#[test]
fn contains_reports_membership() {
    assert!(Ft::NUMERIC.contains(Ft::Float));
    assert!(!Ft::NUMERIC.contains(Ft::String));
    assert!(!TypeSet::EMPTY.contains(Ft::Null));
}

#[test]
fn len_counts_types() {
    assert_eq!(TypeSet::EMPTY.len(), 0);
    assert_eq!(Ft::FUNCTION.len(), 1);
    assert_eq!(Ft::PRIMITIVE.len(), 6);
}

#[test]
fn iter_yields_types_in_declaration_order() {
    let set = Ft::Map | Ft::Null | Ft::String;
    let types: Vec<Ft> = set.iter().collect();
    assert_eq!(types, [Ft::Null, Ft::String, Ft::Map]);
}

#[test]
fn iter_over_every_type_follows_frost_type_exactly() {
    // Every type, so each one's position is checked, not just a sample's.
    let types: Vec<Ft> = Ft::ANY.iter().collect();
    assert_eq!(
        types,
        [
            Ft::Null,
            Ft::Bool,
            Ft::Int,
            Ft::Float,
            Ft::String,
            Ft::Bytes,
            Ft::Array,
            Ft::Map,
            Ft::Function,
            Ft::Opaque,
        ],
        "TypeSet iterates in FrostType's declaration order"
    );
}

#[test]
fn debug_lists_the_types() {
    assert_eq!(format!("{:?}", Ft::Int | Ft::Float), "{Int, Float}");
    assert_eq!(format!("{:?}", TypeSet::EMPTY), "{}");
}
