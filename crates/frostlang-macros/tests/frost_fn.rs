//! `#[frost_fn]`'s compile tests: see `main.rs`.

#[test]
fn frost_fn_compiles_valid_declarations_and_rejects_invalid_ones() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/frost_fn/pass/*.rs");
    cases.compile_fail("tests/frost_fn/fail/*.rs");
}
