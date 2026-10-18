//! [`RenderLimits`]: how much of a value its text renders. A container nested past the
//! depth limit renders as `[...]` or `{...}`; once the text reaches the length limit, the
//! rest of each container renders as `...`. Each marker stands as an element would.
//!
//! The deep and wide cases render on a small stack (see [`deep`](crate::deep)), and their
//! assertions report lengths rather than the text, which can run to megabytes.

use frostlang::{FrostError, FrostMap, MapKey, RenderLimits, SpecialFloat, Value, ValueMap};

use crate::deep;
use crate::script::{self, Script};

fn int(i: i64) -> Value {
    Value::from(i)
}

fn array(elements: Vec<Value>) -> Value {
    Value::from(elements)
}

fn map(entries: Vec<(&str, Value)>) -> Value {
    Value::Map(FrostMap::from_iter(
        entries
            .into_iter()
            .map(|(key, value)| (MapKey::from(key), value)),
    ))
}

fn depth(max: usize) -> RenderLimits {
    RenderLimits::UNLIMITED.with_max_depth(Some(max))
}

fn len(max: usize) -> RenderLimits {
    RenderLimits::UNLIMITED.with_max_len(Some(max))
}

/// `[1, [2, [3, 4], 5], 6]`
fn three_levels() -> Value {
    array(vec![
        int(1),
        array(vec![int(2), array(vec![int(3), int(4)]), int(5)]),
        int(6),
    ])
}

/// The value's text in each style within `limits`, by name.
fn in_each_style(value: &Value, limits: RenderLimits) -> [(&'static str, String); 3] {
    [
        ("compact", value.to_frost_string_within(limits)),
        ("pretty", value.to_pretty_string_within(limits)),
        ("debug", value.to_debug_string_within(limits)),
    ]
}

// -- The limits themselves --

#[test]
fn the_defaults_are_new() {
    assert_eq!(RenderLimits::default(), RenderLimits::new());
    assert_eq!(RenderLimits::new().max_depth, Some(100));
    assert_eq!(RenderLimits::new().max_len, Some(16 << 20));
}

#[test]
fn brief_limits_are_shorter_and_as_deep() {
    assert_eq!(
        RenderLimits::brief().max_depth,
        RenderLimits::new().max_depth
    );
    assert_eq!(RenderLimits::brief().max_len, Some(1 << 10));
}

#[test]
fn each_limit_is_set_alone() {
    let limits = RenderLimits::new().with_max_depth(Some(3));
    assert_eq!(limits.max_depth, Some(3));
    assert_eq!(limits.max_len, RenderLimits::new().max_len);

    let limits = RenderLimits::new().with_max_len(None);
    assert_eq!(limits.max_depth, RenderLimits::new().max_depth);
    assert_eq!(limits.max_len, None);
}

// -- Depth --

#[test]
fn a_container_past_the_depth_limit_renders_as_its_marker() {
    let value = three_levels();
    assert_eq!(
        value.to_frost_string_within(depth(1)),
        "[ 1, [ 2, [...], 5 ], 6 ]"
    );
    assert_eq!(
        value.to_debug_string_within(depth(1)),
        "[ 1, [ 2, [...], 5 ], 6 ]"
    );
    assert_eq!(
        value.to_frost_string_within(depth(2)),
        "[ 1, [ 2, [ 3, 4 ], 5 ], 6 ]",
        "nothing is nested past the limit"
    );
}

#[test]
fn a_depth_limit_of_zero_elides_every_container_inside_the_value() {
    let value = three_levels();
    assert_eq!(value.to_frost_string_within(depth(0)), "[ 1, [...], 6 ]");
    assert_eq!(value.to_debug_string_within(depth(0)), "[ 1, [...], 6 ]");
    let expected = r"[
    1,
    [...],
    6
]";
    assert_eq!(value.to_pretty_string_within(depth(0)), expected);
}

#[test]
fn a_depth_marker_is_laid_out_as_an_element_when_pretty() {
    let expected = r"[
    1,
    [
        2,
        [...],
        5
    ],
    6
]";
    assert_eq!(three_levels().to_pretty_string_within(depth(1)), expected);
}

#[test]
fn a_map_past_the_depth_limit_renders_as_its_marker() {
    let value = map(vec![("a", map(vec![("b", map(vec![("c", int(1))]))]))]);
    assert_eq!(
        value.to_frost_string_within(depth(1)),
        r#"{ ["a"]: { ["b"]: {...} } }"#
    );
    let expected = r"{
    a: {
        b: {...}
    }
}";
    assert_eq!(value.to_pretty_string_within(depth(1)), expected);
}

#[test]
fn arrays_and_maps_within_each_other_render_their_own_markers() {
    let in_an_array = array(vec![map(vec![("a", array(vec![int(1)]))])]);
    assert_eq!(
        in_an_array.to_frost_string_within(depth(1)),
        r#"[ { ["a"]: [...] } ]"#
    );
    let in_a_map = map(vec![("a", array(vec![array(vec![int(1)])]))]);
    assert_eq!(
        in_a_map.to_frost_string_within(depth(1)),
        r#"{ ["a"]: [ [...] ] }"#
    );
    assert_eq!(
        in_a_map.to_frost_string_within(depth(0)),
        r#"{ ["a"]: [...] }"#
    );
}

#[test]
fn an_empty_container_past_the_depth_limit_renders_as_itself() {
    let value = array(vec![
        array(vec![]),
        map(vec![]),
        array(vec![int(1)]),
        map(vec![("a", int(1))]),
    ]);
    assert_eq!(
        value.to_frost_string_within(depth(0)),
        "[ [], {}, [...], {...} ]"
    );
}

#[test]
fn a_value_holding_no_container_ignores_the_depth_limit() {
    for value in [int(1), Value::from("text"), Value::Null] {
        for ((style, limited), (_, unlimited)) in in_each_style(&value, depth(0))
            .into_iter()
            .zip(in_each_style(&value, RenderLimits::UNLIMITED))
        {
            assert_eq!(limited, unlimited, "{style}: {unlimited}");
        }
    }
}

// -- Length --

#[test]
fn the_rest_of_a_container_past_the_length_limit_renders_as_one_marker() {
    let value = array(vec![int(1), int(2), int(3)]);
    // The text reaches 5 bytes, `[ 1, `, before the second element, and 8 before the
    // third.
    assert_eq!(value.to_frost_string_within(len(5)), "[ 1, ... ]");
    assert_eq!(value.to_frost_string_within(len(6)), "[ 1, 2, ... ]");
    assert_eq!(value.to_frost_string_within(len(8)), "[ 1, 2, ... ]");
    assert_eq!(value.to_frost_string_within(len(9)), "[ 1, 2, 3 ]");
    assert_eq!(
        map(vec![("a", int(1)), ("b", int(2))]).to_frost_string_within(len(0)),
        "{ ... }"
    );
}

#[test]
fn a_length_limit_of_zero_elides_every_element() {
    let value = array(vec![int(1), int(2)]);
    assert_eq!(value.to_frost_string_within(len(0)), "[ ... ]");
    assert_eq!(value.to_debug_string_within(len(0)), "[ ... ]");
    assert_eq!(value.to_pretty_string_within(len(0)), "[\n    ...\n]");
}

#[test]
fn each_open_container_ends_with_its_own_length_marker() {
    let value = array(vec![array(vec![int(1), int(2)]), array(vec![int(3)])]);
    assert_eq!(value.to_frost_string_within(len(7)), "[ [ 1, ... ], ... ]");
}

#[test]
fn a_length_marker_is_laid_out_as_an_element_when_pretty() {
    let expected = r"[
    1,
    ...
]";
    let value = array(vec![int(1), int(2), int(3)]);
    assert_eq!(value.to_pretty_string_within(len(7)), expected);
}

/// A Map entry, its key and value, renders whole once begun, whether its key is
/// shorthand or computed.
#[test]
fn a_map_entry_begun_within_the_length_limit_renders_whole() {
    let long = Value::from("x".repeat(20));
    let shorthand = map(vec![("alpha", long.clone()), ("beta", int(2))]);
    let expected = r#"{
    alpha: "xxxxxxxxxxxxxxxxxxxx",
    ...
}"#;
    assert_eq!(shorthand.to_pretty_string_within(len(7)), expected);

    let computed = Value::from(ValueMap::from([
        (MapKey::from(1), long),
        (MapKey::from("beta"), int(2)),
    ]));
    let expected = r#"{
    [1]: "xxxxxxxxxxxxxxxxxxxx",
    ...
}"#;
    assert_eq!(computed.to_pretty_string_within(len(7)), expected);
}

/// A String, Bytes, or Opaque begun within the length limit renders whole.
#[test]
fn a_leaf_begun_within_the_length_limit_renders_whole() {
    let long = "x".repeat(100);
    let opaque = Value::opaque(SpecialFloat::new(f64::INFINITY).expect("special"));
    let leaves = [
        (Value::from(long.as_str()), format!(r#""{long}""#)),
        (
            Value::from(vec![0xffu8; 50]),
            format!("x'{}'", "ff".repeat(50)),
        ),
        (opaque.clone(), opaque.to_frost_string()),
    ];
    for (leaf, text) in leaves {
        let value = array(vec![leaf, int(2)]);
        assert_eq!(
            value.to_frost_string_within(len(3)),
            format!("[ {text}, ... ]")
        );
    }
}

#[test]
fn a_string_alone_is_never_cut() {
    let long = "x".repeat(100);
    let value = Value::from(long.as_str());
    assert_eq!(value.to_frost_string_within(len(1)), long);
    assert_eq!(value.to_pretty_string_within(len(1)), long);
    assert_eq!(
        value.to_debug_string_within(len(1)),
        format!(r#""{long}""#),
        "quoted, as debug renders a String"
    );
}

#[test]
fn both_limits_apply_together() {
    let value = array(vec![array(vec![array(vec![int(1)])]), int(2), int(3)]);
    let limits = RenderLimits::UNLIMITED
        .with_max_depth(Some(1))
        .with_max_len(Some(12));
    assert_eq!(value.to_frost_string_within(limits), "[ [ [...] ], ... ]");
}

// -- Deep and wide values --

/// Past the depth limit, a value renders the same however deeply it nests, in each
/// style, and without overflowing the stack.
#[test]
fn a_deeply_nested_value_renders_only_to_the_default_depth() {
    let max = RenderLimits::new().max_depth.expect("a depth limit");
    deep::on_a_small_stack(|| {
        for (shape, wrap, marker) in [
            ("Arrays", deep::in_an_array as fn(Value) -> Value, "[ "),
            ("Maps", deep::in_a_map, r#"{ ["below"]: "#),
        ] {
            let value = deep::nested(deep::DEPTH, int(1), wrap);
            let just_past = deep::nested(max + 2, int(1), wrap);
            for ((style, text), (_, expected)) in in_each_style(&value, RenderLimits::new())
                .into_iter()
                .zip(in_each_style(&just_past, RenderLimits::new()))
            {
                assert!(text == expected, "{shape}, {style}: as just past the limit");
            }

            let (open, close) = if shape == "Arrays" {
                ("[...]", " ]")
            } else {
                ("{...}", " }")
            };
            let expected = format!("{}{open}{}", marker.repeat(max + 1), close.repeat(max + 1));
            assert!(value.to_frost_string() == expected, "{shape}: compact");
        }
    });
}

/// `levels` levels, each holding the one below twice, so the value is small, but its
/// text in full holds 2^`levels` elements.
fn doubled(levels: usize) -> Value {
    (0..levels).fold(int(1), |below, _| array(vec![below.clone(), below]))
}

/// The most the text of a value `levels` deep can pass a length limit by: one element,
/// then for each open level a marker, separator, and closing bracket, each indented
/// when pretty.
fn most_past(levels: usize) -> usize {
    (levels + 1) * (8 + 2 * 4 * (levels + 1))
}

/// Text that holds the same contents in many places stops near the length limit. Each
/// value's text in full is several times the limit.
#[test]
fn a_value_sharing_its_contents_renders_only_to_the_length_limit() {
    deep::on_a_small_stack(|| {
        for (limits, levels) in [(RenderLimits::new(), 22), (RenderLimits::brief(), 10)] {
            let value = doubled(levels);
            let max = limits.max_len.expect("a length limit");
            for (style, text) in in_each_style(&value, limits) {
                assert!(
                    (max..=max + most_past(levels)).contains(&text.len()),
                    "{style}, {levels} levels within {limits:?}: {} bytes",
                    text.len()
                );
            }
        }
    });
}

/// An error raised with a value renders it in its message within the brief limits.
#[test]
fn an_error_with_a_value_renders_it_briefly() {
    deep::on_a_small_stack(|| {
        let max = RenderLimits::brief().max_depth.expect("a depth limit");
        let deep = deep::nested(deep::DEPTH, int(1), deep::in_an_array);
        let expected = format!("{}[...]{}", "[ ".repeat(max + 1), " ]".repeat(max + 1));
        assert!(
            FrostError::from_value(deep).message() == expected,
            "a deep value"
        );

        let max = RenderLimits::brief().max_len.expect("a length limit");
        let message = FrostError::from_value(doubled(10)).message().into_owned();
        assert!(
            (max..=max + most_past(10)).contains(&message.len()),
            "a wide value: {} bytes",
            message.len()
        );
    });
}

// -- From scripts --
//
// How deep or wide a value is comes from the host as `levels`, so that these test
// rendering rather than what the compiler makes of such a value. A wide value's text
// runs to megabytes, so a script rendering it runs under one set of options: with its
// value from the host, no optimization can change what it renders.

/// A script with `levels` as given.
fn script(source: &str, levels: i64) -> Script {
    Script::new(source).capture("levels", int(levels))
}

/// Each way a script renders a value renders within the default depth.
#[test]
fn a_script_renders_within_the_default_depth() {
    let max = RenderLimits::new().max_depth.expect("a depth limit");
    let source = format!(
        r#"
defn nest(v, n) -> if n == 0: v else: nest([v], n - 1)
def deep = nest(1, levels)
def just_past = nest(1, {just_past})
[
    to_string(deep) == to_string(just_past),
    $"${{deep}}" == to_string(just_past),
    pretty(deep) == pretty(just_past),
    debug_dump(deep) == debug_dump(just_past),
    mformat("${{v}}", {{v: deep}}) == to_string(just_past),
]
"#,
        just_past = max + 2
    );
    let tail = deep::on_a_small_stack(|| script(&source, 10_000).run());
    assert_eq!(tail, array(vec![Value::from(true); 5]), "{source}");
}

/// Each way a script renders a value renders within the default length.
#[test]
fn a_script_renders_within_the_default_length() {
    let source = r#"
defn doubled(v, n) -> if n == 0: v else: doubled([v, v], n - 1)
def wide = doubled(1, levels)
[
    len(to_string(wide)) < 17_000_000,
    len($"${wide}") < 17_000_000,
    len(pretty(wide)) < 17_000_000,
    len(debug_dump(wide)) < 17_000_000,
    len(mformat("${v}", {v: wide})) < 17_000_000,
]
"#;
    let tail = deep::on_a_small_stack(|| script(source, 22).run_under(script::UNOPTIMIZED));
    assert_eq!(tail, array(vec![Value::from(true); 5]), "{source}");
}

/// An error raised with a value from a script renders it within the brief limits.
#[test]
fn an_error_raised_by_a_script_renders_its_value_briefly() {
    let source = r"
defn doubled(v, n) -> if n == 0: v else: doubled([v, v], n - 1)
error(doubled(1, levels))
";
    let message = deep::on_a_small_stack(|| script(source, 10).raises());
    let max = RenderLimits::brief().max_len.expect("a length limit");
    assert!(
        (max..=max + most_past(10)).contains(&message.len()),
        "{} bytes",
        message.len()
    );
}

/// A `match` no arm fits raises naming the value, which renders within the limits even
/// when the error is caught.
#[test]
fn a_match_no_arm_fits_names_the_value_within_the_limits() {
    let source = r"
defn doubled(v, n) -> if n == 0: v else: doubled([v, v], n - 1)
def wide = doubled(1, levels)
def caught = try_call(fn -> match wide { 1 => 1 })
len(caught.error) < 17_000_000
";
    let tail = deep::on_a_small_stack(|| script(source, 22).run_under(script::UNOPTIMIZED));
    assert_eq!(tail, Value::from(true), "{source}");
}
