//! MessagePack: encoding, decoding, unsigned integers beyond the Int range,
//! and the values MessagePack has no form for.
//!
//! MessagePack data is written as Frost Bytes literals, joined with `+` where
//! that shows its parts.

use crate::common::{
    FormatModule, assert_arity, assert_raises, assert_true, assert_values, borrowed, nested_arrays,
    nested_maps, type_errors,
};

const MSGPACK: FormatModule = FormatModule {
    name: "msgpack",
    format: "MessagePack",
};

/// A Frost Bytes literal of the MessagePack data of [`nested_arrays`]:
/// `depth` arrays, each the only element of the one enclosing it.
fn nested_arrays_data(depth: usize) -> String {
    format!("x'{}90'", "91".repeat(depth - 1))
}

// --- encode ---

#[test]
fn encode_writes_each_scalar() {
    assert_values(&[
        ("msgpack.encode(null)", "x'c0'"),
        ("msgpack.encode(false)", "x'c2'"),
        ("msgpack.encode(true)", "x'c3'"),
        ("msgpack.encode('')", "x'a0'"),
        ("msgpack.encode('hi')", "x'a26869'"),
        ("msgpack.encode(x'')", "x'c400'"),
        ("msgpack.encode(x'0001')", "x'c4020001'"),
    ]);
}

#[test]
fn encode_writes_each_int_in_its_smallest_form() {
    assert_values(&[
        ("msgpack.encode(0)", "x'00'"),
        ("msgpack.encode(127)", "x'7f'"),
        ("msgpack.encode(128)", "x'cc80'"),
        ("msgpack.encode(255)", "x'ccff'"),
        ("msgpack.encode(256)", "x'cd0100'"),
        ("msgpack.encode(65535)", "x'cdffff'"),
        ("msgpack.encode(65536)", "x'ce00010000'"),
        ("msgpack.encode(4294967295)", "x'ceffffffff'"),
        ("msgpack.encode(4294967296)", "x'cf0000000100000000'"),
        (
            "msgpack.encode(9223372036854775807)",
            "x'cf7fffffffffffffff'",
        ),
        ("msgpack.encode(-1)", "x'ff'"),
        ("msgpack.encode(-32)", "x'e0'"),
        ("msgpack.encode(-33)", "x'd0df'"),
        ("msgpack.encode(-128)", "x'd080'"),
        ("msgpack.encode(-129)", "x'd1ff7f'"),
        (
            "msgpack.encode(-9223372036854775807 - 1)",
            "x'd38000000000000000'",
        ),
    ]);
}

#[test]
fn encode_writes_every_float_as_a_float64() {
    assert_values(&[
        ("msgpack.encode(1.5)", "x'cb3ff8000000000000'"),
        ("msgpack.encode(0.0)", "x'cb0000000000000000'"),
        ("msgpack.encode(-0.0)", "x'cb8000000000000000'"),
        ("msgpack.encode(1.0)", "x'cb3ff0000000000000'"),
    ]);
}

#[test]
fn encode_writes_special_floats_bit_for_bit() {
    assert_values(&[
        (
            "msgpack.encode(math.special_float('inf'))",
            "x'cb7ff0000000000000'",
        ),
        (
            "msgpack.encode(math.special_float('-inf'))",
            "x'cbfff0000000000000'",
        ),
        (
            "msgpack.encode(math.special_float('NaN'))",
            "x'cb7ff8000000000000'",
        ),
        (
            "msgpack.encode(math.special_float('-NaN'))",
            "x'cbfff8000000000000'",
        ),
    ]);
}

#[test]
fn encode_writes_arrays_and_maps() {
    assert_values(&[
        ("msgpack.encode([])", "x'90'"),
        ("msgpack.encode({})", "x'80'"),
        ("msgpack.encode([1, [2]])", "x'92019102'"),
        ("msgpack.encode({a: 1})", "x'81a16101'"),
    ]);
}

#[test]
fn encode_writes_each_length_in_its_smallest_form() {
    assert_values(&[
        // A fixstr holds up to 31 bytes, then a str8.
        ("msgpack.encode(tile('a', 31))", "x'bf' + tile(x'61', 31)"),
        ("msgpack.encode(tile('a', 32))", "x'd920' + tile(x'61', 32)"),
        // A fixarray holds up to 15 elements, then an array16.
        ("msgpack.encode(tile([0], 15))", "x'9f' + tile(x'00', 15)"),
        (
            "msgpack.encode(tile([0], 16))",
            "x'dc0010' + tile(x'00', 16)",
        ),
    ]);
}

#[test]
fn encode_writes_every_key_type_as_itself() {
    // Entries come in key order: Bool, Int, Float, String, then Bytes.
    assert_values(&[(
        "msgpack.encode({[x'01']: 5, s: 4, [1.5]: 3, [1]: 2, [true]: 1})",
        "x'85' + x'c301' + x'0102' + x'cb3ff8000000000000' + x'03' + x'a17304' + x'c4010105'",
    )]);
}

#[test]
fn encode_refuses_what_msgpack_has_no_form_for() {
    assert_raises(&[
        (
            "msgpack.encode(fn -> 1)",
            &MSGPACK.cannot_encode("Function"),
        ),
        (
            "msgpack.encode(test.widget)",
            &MSGPACK.cannot_encode("<Widget>"),
        ),
        (
            "msgpack.encode(toml.datetime('1979-05-27'))",
            &MSGPACK.cannot_encode("<TOML Datetime: 1979-05-27>"),
        ),
        (
            "msgpack.encode({a: [1, len]})",
            &MSGPACK.cannot_encode_at("Function", "a[1]"),
        ),
        (
            "msgpack.encode({[x'01']: [test.widget]})",
            &MSGPACK.cannot_encode_at("<Widget>", "[x'01'][0]"),
        ),
    ]);
}

#[test]
fn encode_nests_as_deep_as_decode_reads() {
    assert_true(&[
        &format!(
            "
            def v = {}
            msgpack.decode(msgpack.encode(v)) == v
            ",
            nested_arrays(127)
        ),
        &format!(
            "
            def v = {}
            msgpack.decode(msgpack.encode(v)) == v
            ",
            nested_maps(127)
        ),
    ]);
    let too_deep = MSGPACK.too_deep(127);
    assert_raises(&[
        (
            &format!("msgpack.encode({})", nested_arrays(128)),
            &too_deep,
        ),
        (&format!("msgpack.encode({})", nested_maps(128)), &too_deep),
        (
            &format!("msgpack.encode({})", nested_arrays(100_000)),
            &too_deep,
        ),
    ]);
}

// --- decode ---

#[test]
fn decode_reads_every_int_form() {
    assert_values(&[
        // Fixints.
        ("msgpack.decode(x'05')", "5"),
        ("msgpack.decode(x'ff')", "-1"),
        // Unsigned, of each width, within the Int range.
        ("msgpack.decode(x'cc05')", "5"),
        ("msgpack.decode(x'cd0005')", "5"),
        ("msgpack.decode(x'ce00000005')", "5"),
        ("msgpack.decode(x'cf0000000000000005')", "5"),
        (
            "msgpack.decode(x'cf7fffffffffffffff')",
            "9223372036854775807",
        ),
        // Signed, of each width.
        ("msgpack.decode(x'd0ff')", "-1"),
        ("msgpack.decode(x'd1ffff')", "-1"),
        ("msgpack.decode(x'd2ffffffff')", "-1"),
        ("msgpack.decode(x'd3ffffffffffffffff')", "-1"),
        ("msgpack.decode(x'd005')", "5"),
        (
            "msgpack.decode(x'd38000000000000000')",
            "-9223372036854775807 - 1",
        ),
    ]);
}

#[test]
fn decode_reads_every_other_form() {
    assert_values(&[
        ("msgpack.decode(x'c0')", "null"),
        ("msgpack.decode(x'c2')", "false"),
        ("msgpack.decode(x'c3')", "true"),
        // A float32 is the Float it equals.
        ("msgpack.decode(x'ca3fc00000')", "1.5"),
        ("msgpack.decode(x'cb3ff8000000000000')", "1.5"),
        // Strings, of each width.
        ("msgpack.decode(x'a26869')", "'hi'"),
        ("msgpack.decode(x'd9026869')", "'hi'"),
        ("msgpack.decode(x'da00026869')", "'hi'"),
        ("msgpack.decode(x'db000000026869')", "'hi'"),
        // Bins, of each width.
        ("msgpack.decode(x'c4020001')", "x'0001'"),
        ("msgpack.decode(x'c500020001')", "x'0001'"),
        ("msgpack.decode(x'c6000000020001')", "x'0001'"),
        // Arrays and maps, of each width.
        ("msgpack.decode(x'920102')", "[1, 2]"),
        ("msgpack.decode(x'dc00020102')", "[1, 2]"),
        ("msgpack.decode(x'dd000000020102')", "[1, 2]"),
        ("msgpack.decode(x'81a16101')", "{a: 1}"),
        ("msgpack.decode(x'de0001a16101')", "{a: 1}"),
        ("msgpack.decode(x'df00000001a16101')", "{a: 1}"),
    ]);
}

#[test]
fn decode_reads_special_floats_bit_for_bit() {
    assert_true(&[
        "msgpack.decode(x'cb7ff8000000000000') == math.special_float('NaN')",
        "msgpack.decode(x'cbfff8000000000000') == math.special_float('-NaN')",
        "msgpack.decode(x'cb7ff0000000000000') == math.special_float('inf')",
        "msgpack.decode(x'cbfff0000000000000') == math.special_float('-inf')",
        "msgpack.decode(x'cb8000000000000000') == -0.0",
        // A float32 too.
        "msgpack.decode(x'ca7fc00000') == math.special_float('NaN')",
        "msgpack.decode(x'ca7f800000') == math.special_float('inf')",
        "msgpack.decode(x'caff800000') == math.special_float('-inf')",
        "msgpack.decode(x'ca80000000') == -0.0",
    ]);
}

#[test]
fn decode_reads_every_key_type() {
    assert_values(&[(
        "msgpack.decode(x'85' + x'c301' + x'0102' + x'cb3ff8000000000000' + x'03' + x'a17304' + x'c4010105')",
        "{[true]: 1, [1]: 2, [1.5]: 3, s: 4, [x'01']: 5}",
    )]);
}

#[test]
fn decode_reads_a_str_that_is_not_utf8_as_bytes() {
    // As it reads a bin.
    assert_values(&[("msgpack.decode(x'a2fffe')", "x'fffe'")]);
}

#[test]
fn decode_refuses_extension_types() {
    let reason = "an extension type has no Frost form";
    MSGPACK.assert_cannot_decode_because(&[
        ("msgpack.decode(x'd4072a')", reason),
        ("msgpack.decode(x'd5070102')", reason),
        ("msgpack.decode(x'c70307010203')", reason),
        // The standard timestamp is an extension type too.
        ("msgpack.decode(x'd6ff00000001')", reason),
        ("msgpack.decode(x'91d4072a')", reason),
    ]);
}

#[test]
fn decode_refuses_a_key_frost_cannot_hold() {
    MSGPACK.assert_cannot_decode_because(&[
        (
            "msgpack.decode(x'81c001')",
            "Type Null is not a valid Map key",
        ),
        (
            "msgpack.decode(x'81910102')",
            "Type Array is not a valid Map key",
        ),
        (
            "msgpack.decode(x'818002')",
            "Type Map is not a valid Map key",
        ),
        // An Opaque is named by its rendering.
        (
            "msgpack.decode(x'81cb7ff800000000000002')",
            "<SpecialFloat: NaN> is not a valid Map key",
        ),
        (
            "msgpack.decode(x'81cfffffffffffffffff02')",
            "<Uint64: 18446744073709551615> is not a valid Map key",
        ),
    ]);
}

#[test]
fn decode_refuses_an_array_or_map_key_before_reading_within_it() {
    // Each Map's key is another Map, 1000 deep: refused at the first, rather
    // than nesting past the depth limit.
    let maps_keyed_by_maps = format!("x'{}'", "81".repeat(1000));
    MSGPACK.assert_cannot_decode_because(&[
        (
            &format!("msgpack.decode({maps_keyed_by_maps})"),
            "Type Map is not a valid Map key",
        ),
        (
            "msgpack.decode(x'8191919191')",
            "Type Array is not a valid Map key",
        ),
    ]);
}

#[test]
fn decode_refuses_a_key_given_twice() {
    MSGPACK.assert_cannot_decode_because(&[
        ("msgpack.decode(x'82a16101a16102')", r#"duplicate key "a""#),
        ("msgpack.decode(x'8201010101')", "duplicate key 1"),
    ]);
    // Int and Float keys of one number are distinct, as in a Frost Map.
    assert_values(&[(
        "msgpack.decode(x'82' + x'0101' + x'cb3ff0000000000000' + x'02')",
        "{[1]: 1, [1.0]: 2}",
    )]);
}

#[test]
fn decode_refuses_anything_after_the_value() {
    MSGPACK.assert_cannot_decode_because(&[
        ("msgpack.decode(x'0102')", "1 byte after the value"),
        ("msgpack.decode(x'01020304')", "3 bytes after the value"),
        ("msgpack.decode(x'90c0')", "1 byte after the value"),
    ]);
}

#[test]
fn decode_refuses_data_that_ends_early() {
    MSGPACK.assert_cannot_decode_because(&[
        ("msgpack.decode(x'')", "the data ends early, 0 bytes in"),
        ("msgpack.decode(x'9201')", "the data ends early, 2 bytes in"),
        ("msgpack.decode(x'a36869')", "the data ends early"),
        ("msgpack.decode(x'cd01')", "the data ends early"),
    ]);
}

#[test]
fn decode_refuses_lengths_claiming_more_than_the_data_holds() {
    // Each claims about 4 billion elements, entries, or bytes.
    MSGPACK.assert_cannot_decode_because(&[
        ("msgpack.decode(x'ddffffffff')", "the data ends early"),
        ("msgpack.decode(x'dfffffffff')", "the data ends early"),
        ("msgpack.decode(x'dbffffffff')", "the data ends early"),
        ("msgpack.decode(x'c6ffffffff')", "the data ends early"),
        // 127 nested Arrays, each claiming about 4 billion elements: the
        // room reserved in advance stays small at every level.
        (
            &format!("msgpack.decode(x'{}')", "ddffffffff".repeat(127)),
            "the data ends early",
        ),
    ]);
}

#[test]
fn decode_refuses_the_marker_msgpack_reserves() {
    MSGPACK.assert_cannot_decode_because(&[
        (
            "msgpack.decode(x'c1')",
            "byte 0xc1, which MessagePack reserves, 1 byte in",
        ),
        (
            "msgpack.decode(x'9201c1')",
            "byte 0xc1, which MessagePack reserves, 3 bytes in",
        ),
    ]);
}

#[test]
fn decode_reads_as_deep_as_encode_writes() {
    assert_true(&[&format!(
        "msgpack.decode({}) == {}",
        nested_arrays_data(127),
        nested_arrays(127)
    )]);
    MSGPACK.assert_cannot_decode_because(&[(
        &format!("msgpack.decode({})", nested_arrays_data(128)),
        "Arrays and Maps nest more than 127 deep",
    )]);
}

// --- unsigned integers beyond the Int range ---
//
// Their Opaque, and the functions of `ext.formats.u64`, are tested in
// `uint64.rs`.

#[test]
fn an_unsigned_integer_beyond_the_int_range_decodes_to_an_opaque() {
    assert_values(&[
        (
            "to_string(msgpack.decode(x'cfffffffffffffffff'))",
            "'<Uint64: 18446744073709551615>'",
        ),
        (
            "to_string(msgpack.decode(x'cf8000000000000000'))",
            "'<Uint64: 9223372036854775808>'",
        ),
    ]);
    assert_true(&["is_opaque(msgpack.decode(x'cfffffffffffffffff'))"]);
}

#[test]
fn an_unsigned_integer_beyond_the_int_range_decodes_to_an_opaque_within_a_value() {
    assert_true(&[r"
        def big = u64.from_string('18446744073709551615')
        msgpack.decode(x'92cfffffffffffffffff81a161cf8000000000000000') == [
            big,
            {a: u64.from_string('9223372036854775808')},
        ]
    "]);
}

#[test]
fn an_unsigned_integer_opaque_encodes_back_as_itself() {
    assert_values(&[
        (
            "msgpack.encode(msgpack.decode(x'cfffffffffffffffff'))",
            "x'cfffffffffffffffff'",
        ),
        (
            "msgpack.encode([u64.from_string('9223372036854775808')])",
            "x'91cf8000000000000000'",
        ),
    ]);
}

#[test]
fn unsigned_integer_opaques_are_equal_by_value() {
    assert_true(&[
        "msgpack.decode(x'cfffffffffffffffff') == u64.from_string('18446744073709551615')",
        "msgpack.decode(x'cfffffffffffffffff') != msgpack.decode(x'cffffffffffffffffe')",
    ]);
}

// --- round trips ---

/// Values covering every type MessagePack has a form for, and their edges.
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
            msgpack.decode(msgpack.encode(v)) == v
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
            filter(floats, fn f -> msgpack.decode(msgpack.encode(f)) != f)
            ",
            floats.join(", ")
        ),
        "[]",
    )]);
}

// --- arity and types ---

#[test]
fn encode_takes_one_value() {
    assert_arity("msgpack.encode", "1", &[0, 2]);
}

#[test]
fn decode_takes_one_bytes_value() {
    assert_arity("msgpack.decode", "1", &[0, 2]);
    let cases = type_errors(
        "msgpack.decode",
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
