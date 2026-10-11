//! `#[frost_bundle]` and `#[frost_mutable_bundle]`'s compile tests: see
//! `main.rs`.

#[test]
fn bundle_macros_compile_valid_impl_blocks_and_reject_invalid_ones() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/frost_bundle/pass/*.rs");
    cases.compile_fail("tests/frost_bundle/fail/*.rs");
}
