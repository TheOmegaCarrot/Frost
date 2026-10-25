//! CBOR: encoding, decoding, integers beyond the Int range, tags, and the
//! values CBOR has no form for.
//!
//! CBOR data is written as Frost Bytes literals, joined with `+` where that
//! shows its parts. The decoder's own refusals are pinned; the CBOR parser's
//! are only checked to be decode errors.

use crate::common::{
    FormatModule, assert_arity, assert_raises, assert_true, assert_values, borrowed, nested_arrays,
    nested_maps, type_errors,
};

const CBOR: FormatModule = FormatModule {
    name: "cbor",
    format: "CBOR",
};

/// A Frost Bytes literal of the CBOR data of [`nested_arrays`]: `depth`
/// arrays, each the only element of the one enclosing it.
fn nested_arrays_data(depth: usize) -> String {
    format!("x'{}80'", "81".repeat(depth - 1))
}

/// A Frost Bytes literal of the CBOR data of [`nested_maps`]: `depth` maps,
/// each the only entry of the one enclosing it, under `a`.
fn nested_maps_data(depth: usize) -> String {
    format!("x'{}a0'", "a16161".repeat(depth - 1))
}

// --- encode ---

#[test]
fn encode_writes_each_scalar() {
    assert_values(&[
        ("cbor.encode(null)", "x'f6'"),
        ("cbor.encode(false)", "x'f4'"),
        ("cbor.encode(true)", "x'f5'"),
        ("cbor.encode('')", "x'60'"),
        ("cbor.encode('hi')", "x'626869'"),
        ("cbor.encode(x'')", "x'40'"),
        ("cbor.encode(x'0001')", "x'420001'"),
    ]);
}

#[test]
fn encode_writes_each_int_in_its_smallest_form() {
    assert_values(&[
        ("cbor.encode(0)", "x'00'"),
        ("cbor.encode(23)", "x'17'"),
        ("cbor.encode(24)", "x'1818'"),
        ("cbor.encode(255)", "x'18ff'"),
        ("cbor.encode(256)", "x'190100'"),
        ("cbor.encode(65535)", "x'19ffff'"),
        ("cbor.encode(65536)", "x'1a00010000'"),
        ("cbor.encode(4294967295)", "x'1affffffff'"),
        ("cbor.encode(4294967296)", "x'1b0000000100000000'"),
        ("cbor.encode(9223372036854775807)", "x'1b7fffffffffffffff'"),
        ("cbor.encode(-1)", "x'20'"),
        ("cbor.encode(-24)", "x'37'"),
        ("cbor.encode(-25)", "x'3818'"),
        ("cbor.encode(-256)", "x'38ff'"),
        ("cbor.encode(-257)", "x'390100'"),
        ("cbor.encode(-65536)", "x'39ffff'"),
        ("cbor.encode(-65537)", "x'3a00010000'"),
        ("cbor.encode(-4294967296)", "x'3affffffff'"),
        ("cbor.encode(-4294967297)", "x'3b0000000100000000'"),
        (
            "cbor.encode(-9223372036854775807 - 1)",
            "x'3b7fffffffffffffff'",
        ),
    ]);
}

#[test]
fn encode_writes_every_float_as_a_float64() {
    assert_values(&[
        ("cbor.encode(1.5)", "x'fb3ff8000000000000'"),
        ("cbor.encode(0.0)", "x'fb0000000000000000'"),
        ("cbor.encode(-0.0)", "x'fb8000000000000000'"),
        ("cbor.encode(1.0)", "x'fb3ff0000000000000'"),
    ]);
}

#[test]
fn encode_writes_special_floats_bit_for_bit() {
    assert_values(&[
        (
            "cbor.encode(math.special_float('inf'))",
            "x'fb7ff0000000000000'",
        ),
        (
            "cbor.encode(math.special_float('-inf'))",
            "x'fbfff0000000000000'",
        ),
        (
            "cbor.encode(math.special_float('NaN'))",
            "x'fb7ff8000000000000'",
        ),
        (
            "cbor.encode(math.special_float('-NaN'))",
            "x'fbfff8000000000000'",
        ),
    ]);
}

#[test]
fn encode_writes_arrays_and_maps_with_their_lengths() {
    assert_values(&[
        ("cbor.encode([])", "x'80'"),
        ("cbor.encode({})", "x'a0'"),
        ("cbor.encode([1, [2]])", "x'82018102'"),
        ("cbor.encode({a: 1})", "x'a1616101'"),
    ]);
}

#[test]
fn encode_writes_each_length_in_its_smallest_form() {
    assert_values(&[
        // A length up to 23 is in the initial byte, then in the next bytes,
        // as few as hold it.
        ("cbor.encode(tile('a', 23))", "x'77' + tile(x'61', 23)"),
        ("cbor.encode(tile('a', 24))", "x'7818' + tile(x'61', 24)"),
        ("cbor.encode(tile('a', 255))", "x'78ff' + tile(x'61', 255)"),
        (
            "cbor.encode(tile('a', 256))",
            "x'790100' + tile(x'61', 256)",
        ),
        ("cbor.encode(tile(x'00', 24))", "x'5818' + tile(x'00', 24)"),
        ("cbor.encode(tile([0], 23))", "x'97' + tile(x'00', 23)"),
        ("cbor.encode(tile([0], 24))", "x'9818' + tile(x'00', 24)"),
    ]);
}

#[test]
fn encode_writes_every_key_type_as_itself() {
    // Entries come in key order: Bool, Int, Float, String, then Bytes.
    assert_values(&[(
        "cbor.encode({[x'01']: 5, s: 4, [1.5]: 3, [1]: 2, [true]: 1})",
        "x'a5' + x'f501' + x'0102' + x'fb3ff8000000000000' + x'03' + x'617304' + x'410105'",
    )]);
}

#[test]
fn encode_refuses_what_cbor_has_no_form_for() {
    assert_raises(&[
        ("cbor.encode(fn -> 1)", &CBOR.cannot_encode("Function")),
        ("cbor.encode(test.widget)", &CBOR.cannot_encode("<Widget>")),
        (
            "cbor.encode(toml.datetime('1979-05-27'))",
            &CBOR.cannot_encode("<TOML Datetime: 1979-05-27>"),
        ),
        (
            "cbor.encode({a: [1, len]})",
            &CBOR.cannot_encode_at("Function", "a[1]"),
        ),
        (
            "cbor.encode({[x'01']: [test.widget]})",
            &CBOR.cannot_encode_at("<Widget>", "[x'01'][0]"),
        ),
    ]);
}

#[test]
fn encode_nests_as_deep_as_decode_reads() {
    assert_true(&[
        &format!(
            "
            def v = {}
            cbor.decode(cbor.encode(v)) == v
            ",
            nested_arrays(127)
        ),
        &format!(
            "
            def v = {}
            cbor.decode(cbor.encode(v)) == v
            ",
            nested_maps(127)
        ),
    ]);
    let too_deep = CBOR.too_deep(127);
    assert_raises(&[
        (&format!("cbor.encode({})", nested_arrays(128)), &too_deep),
        (&format!("cbor.encode({})", nested_maps(128)), &too_deep),
        (
            &format!("cbor.encode({})", nested_arrays(100_000)),
            &too_deep,
        ),
    ]);
}

// --- decode ---

#[test]
fn decode_reads_every_integer_form() {
    assert_values(&[
        // Unsigned, of each width, within the Int range, in its smallest form
        // or not.
        ("cbor.decode(x'05')", "5"),
        ("cbor.decode(x'1805')", "5"),
        ("cbor.decode(x'190005')", "5"),
        ("cbor.decode(x'1a00000005')", "5"),
        ("cbor.decode(x'1b0000000000000005')", "5"),
        ("cbor.decode(x'1b7fffffffffffffff')", "9223372036854775807"),
        // Negative, of each width.
        ("cbor.decode(x'20')", "-1"),
        ("cbor.decode(x'3800')", "-1"),
        ("cbor.decode(x'390000')", "-1"),
        ("cbor.decode(x'3a00000000')", "-1"),
        ("cbor.decode(x'3b0000000000000000')", "-1"),
        (
            "cbor.decode(x'3b7fffffffffffffff')",
            "-9223372036854775807 - 1",
        ),
    ]);
}

#[test]
fn decode_reads_every_other_form() {
    assert_values(&[
        ("cbor.decode(x'f6')", "null"),
        ("cbor.decode(x'f4')", "false"),
        ("cbor.decode(x'f5')", "true"),
        // A float16 or float32 is the Float it equals.
        ("cbor.decode(x'f93e00')", "1.5"),
        ("cbor.decode(x'fa3fc00000')", "1.5"),
        ("cbor.decode(x'fb3ff8000000000000')", "1.5"),
        // Text strings, of each width.
        ("cbor.decode(x'626869')", "'hi'"),
        ("cbor.decode(x'78026869')", "'hi'"),
        ("cbor.decode(x'7900026869')", "'hi'"),
        ("cbor.decode(x'7a000000026869')", "'hi'"),
        ("cbor.decode(x'7b00000000000000026869')", "'hi'"),
        // Byte strings, of each width.
        ("cbor.decode(x'420001')", "x'0001'"),
        ("cbor.decode(x'58020001')", "x'0001'"),
        ("cbor.decode(x'5900020001')", "x'0001'"),
        ("cbor.decode(x'5a000000020001')", "x'0001'"),
        ("cbor.decode(x'5b00000000000000020001')", "x'0001'"),
        // Arrays and maps, of each width.
        ("cbor.decode(x'820102')", "[1, 2]"),
        ("cbor.decode(x'98020102')", "[1, 2]"),
        ("cbor.decode(x'9900020102')", "[1, 2]"),
        ("cbor.decode(x'9a000000020102')", "[1, 2]"),
        ("cbor.decode(x'9b00000000000000020102')", "[1, 2]"),
        ("cbor.decode(x'a1616101')", "{a: 1}"),
        ("cbor.decode(x'b801616101')", "{a: 1}"),
        ("cbor.decode(x'b90001616101')", "{a: 1}"),
        ("cbor.decode(x'ba00000001616101')", "{a: 1}"),
        ("cbor.decode(x'bb0000000000000001616101')", "{a: 1}"),
    ]);
}

#[test]
fn decode_reads_indefinite_lengths() {
    assert_values(&[
        // Text and byte strings in chunks, ended by a break.
        ("cbor.decode(x'7f' + x'6161' + x'6162' + x'ff')", "'ab'"),
        ("cbor.decode(x'7fff')", "''"),
        ("cbor.decode(x'5f' + x'4100' + x'4101' + x'ff')", "x'0001'"),
        // Arrays and maps, ended by a break.
        ("cbor.decode(x'9f' + x'0102' + x'ff')", "[1, 2]"),
        ("cbor.decode(x'9fff')", "[]"),
        ("cbor.decode(x'9f' + x'9f01ff' + x'ff')", "[[1]]"),
        ("cbor.decode(x'bf' + x'616101' + x'ff')", "{a: 1}"),
        ("cbor.decode(x'bfff')", "{}"),
    ]);
}

#[test]
fn decode_reads_special_floats_bit_for_bit() {
    assert_true(&[
        "cbor.decode(x'fb7ff8000000000000') == math.special_float('NaN')",
        "cbor.decode(x'fbfff8000000000000') == math.special_float('-NaN')",
        "cbor.decode(x'fb7ff0000000000000') == math.special_float('inf')",
        "cbor.decode(x'fbfff0000000000000') == math.special_float('-inf')",
        "cbor.decode(x'fb8000000000000000') == -0.0",
        // A float32 too.
        "cbor.decode(x'fa7fc00000') == math.special_float('NaN')",
        "cbor.decode(x'faffc00000') == math.special_float('-NaN')",
        "cbor.decode(x'fa7f800000') == math.special_float('inf')",
        "cbor.decode(x'faff800000') == math.special_float('-inf')",
        "cbor.decode(x'fa80000000') == -0.0",
        // A float32 NaN keeps its sign and payload.
        "cbor.decode(x'fa7fc00001') == cbor.decode(x'fb7ff8000020000000')",
        "cbor.decode(x'faffc00001') == cbor.decode(x'fbfff8000020000000')",
        // And a float16.
        "cbor.decode(x'f97e00') == math.special_float('NaN')",
        "cbor.decode(x'f9fe00') == math.special_float('-NaN')",
        "cbor.decode(x'f97c00') == math.special_float('inf')",
        "cbor.decode(x'f9fc00') == math.special_float('-inf')",
        "cbor.decode(x'f98000') == -0.0",
    ]);
}

#[test]
fn decode_reads_every_key_type() {
    assert_values(&[(
        "cbor.decode(x'a5' + x'f501' + x'0102' + x'fb3ff8000000000000' + x'03' + x'617304' + x'410105')",
        "{[true]: 1, [1]: 2, [1.5]: 3, s: 4, [x'01']: 5}",
    )]);
}

#[test]
fn decode_reads_a_bignum_as_the_integer_it_holds() {
    assert_values(&[
        // Tag 2, positive: the bytes are the integer, big-endian.
        ("cbor.decode(x'c2' + x'40')", "0"),
        ("cbor.decode(x'c2' + x'4105')", "5"),
        (
            "cbor.decode(x'c2' + x'487fffffffffffffff')",
            "9223372036854775807",
        ),
        // Tag 3, negative: -1 minus the integer the bytes are.
        ("cbor.decode(x'c3' + x'40')", "-1"),
        ("cbor.decode(x'c3' + x'4100')", "-1"),
        (
            "cbor.decode(x'c3' + x'487fffffffffffffff')",
            "-9223372036854775807 - 1",
        ),
        ("cbor.decode(x'82' + x'c24105' + x'c34105')", "[5, -6]"),
        // As a Map key, as the integer is.
        ("cbor.decode(x'a1' + x'c24105' + x'02')", "{[5]: 2}"),
        // Beyond the Int range, within the unsigned 64-bit range.
        (
            "cbor.decode(x'c2' + x'48ffffffffffffffff')",
            "u64.from_string('18446744073709551615')",
        ),
        // Leading zero bytes count for nothing, however many.
        (
            "cbor.decode(x'c2' + x'4a0000ffffffffffffffff')",
            "u64.from_string('18446744073709551615')",
        ),
        (
            &format!("cbor.decode(x'c2' + x'51' + x'{}01')", "00".repeat(16)),
            "1",
        ),
    ]);
}

#[test]
fn a_bignum_encodes_back_as_the_integer_it_holds() {
    assert_values(&[
        ("cbor.encode(cbor.decode(x'c2' + x'4105'))", "x'05'"),
        (
            "cbor.encode(cbor.decode(x'c2' + x'48ffffffffffffffff'))",
            "x'1bffffffffffffffff'",
        ),
    ]);
}

#[test]
fn decode_refuses_a_bignum_of_anything_but_a_definite_length_byte_string() {
    CBOR.assert_cannot_decode(&[
        // An integer.
        "cbor.decode(x'c2' + x'01')",
        // A text string.
        "cbor.decode(x'c2' + x'6105')",
        // An indefinite-length byte string.
        "cbor.decode(x'c2' + x'5f' + x'4105' + x'ff')",
    ]);
}

#[test]
fn decode_refuses_an_integer_neither_int_nor_uint64_holds() {
    CBOR.assert_cannot_decode_because(&[
        // A negative integer below the Int range.
        (
            "cbor.decode(x'3b8000000000000000')",
            "integer -9223372036854775809 is beyond the Int range, 9 bytes in",
        ),
        (
            "cbor.decode(x'3bffffffffffffffff')",
            "integer -18446744073709551616 is beyond the Int range",
        ),
        // A bignum beyond both the Int and Uint64 ranges.
        (
            "cbor.decode(x'c3' + x'488000000000000000')",
            "integer -9223372036854775809 is beyond the Int range",
        ),
        (
            "cbor.decode(x'c2' + x'49010000000000000000')",
            "integer 18446744073709551616 is beyond the Int range",
        ),
    ]);
    // 17 significant bytes are past what `minicbor` reads, so it refuses them
    // itself, and only the error is checked.
    CBOR.assert_cannot_decode(&[&format!(
        "cbor.decode(x'c2' + x'51' + x'{}')",
        "ff".repeat(17)
    )]);
}

#[test]
fn decode_skips_the_self_described_cbor_tag_at_the_start() {
    assert_values(&[
        ("cbor.decode(x'd9d9f7' + x'01')", "1"),
        ("cbor.decode(x'd9d9f7' + x'820102')", "[1, 2]"),
    ]);
    // It counts in the positions errors give.
    CBOR.assert_cannot_decode_because(&[
        (
            "cbor.decode(x'd9d9f7' + x'a2' + x'0101' + x'0102')",
            "duplicate key 1, 7 bytes in",
        ),
        ("cbor.decode(x'd9d9f7' + x'0102')", "1 byte after the value"),
    ]);
    // Anywhere past the start, even straight after a first one, it is a tag
    // like any other. And at the start, a value must follow it.
    CBOR.assert_cannot_decode(&[
        "cbor.decode(x'81' + x'd9d9f701')",
        "cbor.decode(x'd9d9f7' + x'd9d9f701')",
        "cbor.decode(x'd9d9f7')",
    ]);
}

#[test]
fn decode_refuses_tags() {
    CBOR.assert_cannot_decode(&[
        // Datetimes, as text and as seconds since the epoch.
        "cbor.decode(x'c0' + x'74' + x'323031332d30332d32315432303a30343a30305a')",
        "cbor.decode(x'c1' + x'1a514b67b0')",
        // A URI, and embedded CBOR.
        "cbor.decode(x'd820' + x'60')",
        "cbor.decode(x'd818' + x'4101')",
        // Within a value.
        "cbor.decode(x'81' + x'c11a514b67b0')",
        "cbor.decode(x'a1' + x'6161' + x'c11a514b67b0')",
        "cbor.decode(x'a1' + x'c101' + x'02')",
    ]);
}

#[test]
fn decode_refuses_undefined_and_other_simple_values() {
    CBOR.assert_cannot_decode(&[
        "cbor.decode(x'f7')",
        "cbor.decode(x'81f7')",
        // Unassigned simple values, in one byte and in two.
        "cbor.decode(x'f0')",
        "cbor.decode(x'f820')",
        "cbor.decode(x'f8ff')",
    ]);
}

#[test]
fn decode_refuses_text_that_is_not_utf8() {
    CBOR.assert_cannot_decode(&[
        "cbor.decode(x'62fffe')",
        // In a chunk of an indefinite-length string too.
        "cbor.decode(x'7f' + x'62fffe' + x'ff')",
        // A chunk splitting a character is not UTF-8 by itself.
        "cbor.decode(x'7f' + x'61c3' + x'61a9' + x'ff')",
    ]);
}

#[test]
fn decode_refuses_a_key_frost_cannot_hold() {
    CBOR.assert_cannot_decode_because(&[
        (
            "cbor.decode(x'a1' + x'f6' + x'01')",
            "Type Null is not a valid Map key, 2 bytes in",
        ),
        (
            "cbor.decode(x'a1' + x'820102' + x'03')",
            "Type Array is not a valid Map key",
        ),
        (
            "cbor.decode(x'a1' + x'a0' + x'02')",
            "Type Map is not a valid Map key",
        ),
        // An Opaque is named by its rendering.
        (
            "cbor.decode(x'a1' + x'fb7ff8000000000000' + x'02')",
            "<SpecialFloat: NaN> is not a valid Map key",
        ),
        (
            "cbor.decode(x'a1' + x'1bffffffffffffffff' + x'02')",
            "<Uint64: 18446744073709551615> is not a valid Map key",
        ),
    ]);
}

#[test]
fn decode_refuses_an_array_or_map_key_before_reading_within_it() {
    // Each Map's key is another Map, 1000 deep: refused at the first, rather
    // than nesting past the depth limit.
    let maps_keyed_by_maps = format!("x'{}'", "a1".repeat(1000));
    CBOR.assert_cannot_decode_because(&[
        (
            &format!("cbor.decode({maps_keyed_by_maps})"),
            "Type Map is not a valid Map key",
        ),
        (
            "cbor.decode(x'a181818181')",
            "Type Array is not a valid Map key",
        ),
    ]);
}

#[test]
fn decode_refuses_a_key_given_twice() {
    CBOR.assert_cannot_decode_because(&[
        (
            "cbor.decode(x'a2' + x'616101' + x'616102')",
            r#"duplicate key "a", 6 bytes in"#,
        ),
        ("cbor.decode(x'a2' + x'0101' + x'0101')", "duplicate key 1"),
        // In an indefinite-length map too.
        (
            "cbor.decode(x'bf' + x'0101' + x'0101' + x'ff')",
            "duplicate key 1",
        ),
    ]);
    // Int and Float keys of one number are distinct, as in a Frost Map.
    assert_values(&[(
        "cbor.decode(x'a2' + x'0101' + x'fb3ff0000000000000' + x'02')",
        "{[1]: 1, [1.0]: 2}",
    )]);
}

#[test]
fn decode_refuses_anything_after_the_value() {
    CBOR.assert_cannot_decode_because(&[
        ("cbor.decode(x'0102')", "1 byte after the value"),
        ("cbor.decode(x'01020304')", "3 bytes after the value"),
        ("cbor.decode(x'80f6')", "1 byte after the value"),
    ]);
}

#[test]
fn decode_refuses_malformed_data() {
    CBOR.assert_cannot_decode(&[
        // Ending early.
        "cbor.decode(x'')",
        "cbor.decode(x'8201')",
        "cbor.decode(x'636869')",
        "cbor.decode(x'1901')",
        "cbor.decode(x'9f01')",
        // A break with nothing to end.
        "cbor.decode(x'ff')",
        "cbor.decode(x'8201ff')",
        // Additional information CBOR reserves.
        "cbor.decode(x'1c')",
        "cbor.decode(x'fc')",
        "cbor.decode(x'fd')",
        "cbor.decode(x'fe')",
        // A simple value under 32 in two bytes, even `false`.
        "cbor.decode(x'f814')",
        // An indefinite-length string holding a chunk of another type.
        "cbor.decode(x'7f' + x'4100' + x'ff')",
    ]);
}

#[test]
fn decode_refuses_lengths_claiming_more_than_the_data_holds() {
    // Each claims about 2^64 elements, entries, or bytes.
    CBOR.assert_cannot_decode(&[
        "cbor.decode(x'9bffffffffffffffff')",
        "cbor.decode(x'bbffffffffffffffff')",
        "cbor.decode(x'7bffffffffffffffff')",
        "cbor.decode(x'5bffffffffffffffff')",
        // 127 nested arrays, each claiming about 2^64 elements: the room
        // reserved in advance stays small at every level.
        &format!("cbor.decode(x'{}')", "9bffffffffffffffff".repeat(127)),
    ]);
}

#[test]
fn decode_reads_as_deep_as_encode_writes() {
    assert_true(&[
        &format!(
            "cbor.decode({}) == {}",
            nested_arrays_data(127),
            nested_arrays(127)
        ),
        &format!(
            "cbor.decode({}) == {}",
            nested_maps_data(127),
            nested_maps(127)
        ),
    ]);
    CBOR.assert_cannot_decode_because(&[
        (
            &format!("cbor.decode({})", nested_arrays_data(128)),
            "Arrays and Maps nest more than 127 deep",
        ),
        // The innermost Map is empty, and counts toward the depth all the
        // same.
        (
            &format!("cbor.decode({})", nested_maps_data(128)),
            "Arrays and Maps nest more than 127 deep",
        ),
        // Indefinite-length arrays too, never ended.
        (
            &format!("cbor.decode(x'{}')", "9f".repeat(1000)),
            "Arrays and Maps nest more than 127 deep",
        ),
    ]);
}

// --- unsigned integers beyond the Int range ---
//
// Their Opaque, and the functions of `ext.formats.u64`, are tested in
// `uint64.rs`.

#[test]
fn an_unsigned_integer_beyond_the_int_range_decodes_to_an_opaque() {
    assert_values(&[
        (
            "to_string(cbor.decode(x'1bffffffffffffffff'))",
            "'<Uint64: 18446744073709551615>'",
        ),
        (
            "to_string(cbor.decode(x'1b8000000000000000'))",
            "'<Uint64: 9223372036854775808>'",
        ),
    ]);
    assert_true(&["is_opaque(cbor.decode(x'1bffffffffffffffff'))"]);
}

#[test]
fn an_unsigned_integer_beyond_the_int_range_decodes_to_an_opaque_within_a_value() {
    assert_true(&[r"
        def big = u64.from_string('18446744073709551615')
        cbor.decode(x'82' + x'1bffffffffffffffff' + x'a1' + x'6161' + x'1b8000000000000000') == [
            big,
            {a: u64.from_string('9223372036854775808')},
        ]
    "]);
}

#[test]
fn an_unsigned_integer_opaque_encodes_back_as_itself() {
    assert_values(&[
        (
            "cbor.encode(cbor.decode(x'1bffffffffffffffff'))",
            "x'1bffffffffffffffff'",
        ),
        (
            "cbor.encode([u64.from_string('9223372036854775808')])",
            "x'81' + x'1b8000000000000000'",
        ),
    ]);
}

#[test]
fn an_unsigned_integer_converts_between_cbor_and_messagepack() {
    // Both formats share the Opaque.
    assert_values(&[
        (
            "cbor.encode(msgpack.decode(x'cfffffffffffffffff'))",
            "x'1bffffffffffffffff'",
        ),
        (
            "msgpack.encode(cbor.decode(x'1bffffffffffffffff'))",
            "x'cfffffffffffffffff'",
        ),
    ]);
}

// --- round trips ---

/// Values covering every type CBOR has a form for, and their edges.
const REPRESENTABLE: &[&str] = &[
    "null",
    "true",
    "false",
    "0",
    "-1",
    "9223372036854775807",
    "-9223372036854775807 - 1",
    "0.0",
    "-0.0",
    "1.0",
    "0.1",
    "5e-324",
    "-1.7976931348623157e308",
    "math.special_float('inf')",
    "math.special_float('-NaN')",
    "u64.from_string('18446744073709551615')",
    "''",
    r"'\'\n\t\\\u{0}\u{1f}\u{7f}'",
    "'caf\u{e9} \u{1f600}'",
    "x''",
    "x'00ff'",
    "[]",
    "{}",
    "[[], {}, [[]], [1, 1.0, '1', x'31', null, true]]",
    "{[true]: 1, [0]: 2, [-0.0]: 3, [0.0]: 4, ['']: 5, [x'']: 6, a: {b: [x'01']}}",
];

#[test]
fn decoding_what_encode_gives_gives_the_value_encoded() {
    for value in REPRESENTABLE {
        assert_true(&[&format!(
            "
            def v = {value}
            cbor.decode(cbor.encode(v)) == v
            "
        )]);
    }
}

#[test]
fn every_float_round_trips_exactly() {
    // Bit patterns spread evenly over the whole `f64` range reach every
    // exponent and sign, subnormals included, each with a different mantissa.
    const SAMPLES: u64 = 4099;
    let floats: Vec<String> = (0..SAMPLES)
        .map(|i| f64::from_bits(i * (u64::MAX / SAMPLES)))
        .filter(|f| f.is_finite())
        .map(|f| format!("{f:?}"))
        .collect();
    assert_values(&[(
        &format!(
            "
            def floats = [{}]
            filter(floats, fn f -> cbor.decode(cbor.encode(f)) != f)
            ",
            floats.join(", ")
        ),
        "[]",
    )]);
}

// --- arity and types ---

#[test]
fn encode_takes_one_value() {
    assert_arity("cbor.encode", "1", &[0, 2]);
}

#[test]
fn decode_takes_one_bytes_value() {
    assert_arity("cbor.decode", "1", &[0, 2]);
    let cases = type_errors(
        "cbor.decode",
        "Bytes",
        "data",
        &[
            ("'x'", "String"),
            ("1", "Int"),
            ("null", "Null"),
            ("{}", "Map"),
        ],
    );
    assert_raises(&borrowed(&cases));
}
