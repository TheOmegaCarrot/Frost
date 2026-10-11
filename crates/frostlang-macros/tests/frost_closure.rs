//! `frost_closure!`'s compile tests: see `main.rs`.

#[test]
fn frost_closure_compiles_valid_closures_and_rejects_invalid_ones() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/frost_closure/pass/*.rs");
    cases.compile_fail("tests/frost_closure/fail/*.rs");
}
