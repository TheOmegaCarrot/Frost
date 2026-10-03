//! `LineFrontend`, the one-line-per-segment frontend.

use std::io::{self, Cursor};

use frost_repl::{Frontend, LineFrontend, Repl, ReplError};
use frost_runtime::Value;

/// Every segment `frontend` reads, until it ends.
fn read_all(mut frontend: impl Frontend) -> Vec<String> {
    let mut segments = Vec::new();
    while let Some(segment) = frontend.read_segment().unwrap() {
        segments.push(segment);
    }
    segments
}

/// What a session reading `lines` wrote: its output and its errors.
fn session(lines: &str) -> (String, String) {
    let (mut output, mut errors) = (Vec::new(), Vec::new());
    let mut frontend = LineFrontend::new(Cursor::new(lines), &mut output, &mut errors);
    Repl::new().run(&mut frontend).unwrap();
    (
        String::from_utf8(output).unwrap(),
        String::from_utf8(errors).unwrap(),
    )
}

/// The failure `source` gives as a REPL's first input.
fn failure(source: &str) -> ReplError {
    Repl::new()
        .evaluate(source)
        .expect_err("the input should fail")
}

// --- Reading ---

/// Every segment a default frontend reads from `lines`, and the prompts it
/// wrote.
fn reading(lines: &str) -> (Vec<String>, String) {
    let mut output = Vec::new();
    let segments = read_all(LineFrontend::new(
        Cursor::new(lines),
        &mut output,
        io::sink(),
    ));
    (segments, String::from_utf8(output).unwrap())
}

#[test]
fn each_complete_line_is_a_segment_without_its_line_ending() {
    let (segments, _) = reading("1 + 2\ndef x = 1\r\n\nlast");
    assert_eq!(segments, ["1 + 2", "def x = 1", "", "last"]);
}

#[test]
fn an_unfinished_segment_continues_onto_the_next_line() {
    let (segments, _) = reading("f(1,\n2)\n[\r\n3\r\n]\r\nif x:\n1\n");
    assert_eq!(segments, ["f(1,\n2)", "[\n3\n]", "if x:\n1"]);
}

#[test]
fn a_line_continuation_continues_a_segment_and_is_removed() {
    let (segments, _) = reading("1 + \\\n2\nlast\n");
    assert_eq!(segments, ["1 + \n2", "last"]);
}

#[test]
fn a_continuation_prompt_comes_before_each_further_line() {
    let (_, prompts) = reading("f(\n1,\n2)\nx\n");
    assert_eq!(prompts, "> . . > > \n");
}

#[test]
fn the_continuation_prompt_can_be_changed() {
    let mut output = Vec::new();
    read_all(
        LineFrontend::new(Cursor::new("f(\n1)\n"), &mut output, io::sink())
            .with_prompt(">>> ")
            .with_continuation_prompt("... "),
    );
    assert_eq!(String::from_utf8(output).unwrap(), ">>> ... >>> \n");
}

#[test]
fn input_ending_mid_segment_hands_over_the_segment_as_it_is() {
    // Unchanged, even a trailing line continuation: the compiler reports it.
    for (lines, expected) in [("f(1,\n2", "f(1,\n2"), ("1 + \\\n", "1 + \\")] {
        let (segments, _) = reading(lines);
        assert_eq!(segments, [expected], "{lines:?}");
    }
    let (_, prompts) = reading("f(\n");
    assert_eq!(prompts, "> . \n> \n", "each prompt's line ends");
}

#[test]
fn a_prompt_comes_before_each_line_and_a_newline_at_the_end() {
    let mut output = Vec::new();
    read_all(LineFrontend::new(
        Cursor::new("a\nb\n"),
        &mut output,
        io::sink(),
    ));
    assert_eq!(String::from_utf8(output).unwrap(), "> > > \n");
}

#[test]
fn the_prompt_can_be_changed() {
    let mut output = Vec::new();
    read_all(LineFrontend::new(Cursor::new("a\n"), &mut output, io::sink()).with_prompt("frost> "));
    assert_eq!(String::from_utf8(output).unwrap(), "frost> frost> \n");
}

#[test]
fn empty_input_ends_at_once() {
    let mut output = Vec::new();
    assert!(read_all(LineFrontend::new(Cursor::new(""), &mut output, io::sink())).is_empty());
}

// --- Rendering ---

#[test]
fn a_value_is_pretty_printed_on_its_own_line() {
    let value = Repl::new().evaluate("[1, 'two', {k: null}]").unwrap();
    let (mut output, mut errors) = (Vec::new(), Vec::new());
    LineFrontend::new(Cursor::new(""), &mut output, &mut errors)
        .render(Ok(&value))
        .unwrap();
    let expected = r#"[
    1,
    "two",
    {
        k: null
    }
]
"#;
    assert_eq!(String::from_utf8(output).unwrap(), expected);
    assert!(errors.is_empty());
}

#[test]
fn null_renders_as_nothing() {
    let (mut output, mut errors) = (Vec::new(), Vec::new());
    LineFrontend::new(Cursor::new(""), &mut output, &mut errors)
        .render(Ok(&Value::Null))
        .unwrap();
    assert!(output.is_empty() && errors.is_empty());
}

#[test]
fn a_failure_is_written_to_errors_as_displayed_on_its_own_line() {
    for source in ["nope", "1 / 0"] {
        let error = failure(source);
        let (mut output, mut errors) = (Vec::new(), Vec::new());
        LineFrontend::new(Cursor::new(""), &mut output, &mut errors)
            .render(Err(&error))
            .unwrap();
        assert!(output.is_empty(), "{source:?}");
        assert_eq!(
            String::from_utf8(errors).unwrap(),
            format!("{error}\n"),
            "{source:?}"
        );
    }
}

#[test]
fn a_session_prompts_shows_each_outcome_and_carries_on() {
    // A `def` and a Null write nothing but the next prompt.
    let (output, errors) = session("def x = 1\nnull\nnope\nx + 1\n");
    assert_eq!(output, "> > > > 2\n> \n");
    assert!(errors.contains("`nope` is not defined"), "{errors}");
    assert!(errors.ends_with('\n'), "{errors:?}");
}

#[test]
fn a_session_runs_a_segment_spanning_lines_as_one_input() {
    let (output, errors) = session("defn inc(x) ->\n  x + 1\ninc(1)\n");
    assert_eq!(output, "> . > 2\n> \n");
    assert_eq!(errors, "");
}

#[test]
fn text_is_written_unstyled_on_its_own_line() {
    let (mut output, mut errors) = (Vec::new(), Vec::new());
    let mut frontend = LineFrontend::new(Cursor::new(""), &mut output, &mut errors);
    assert!(!frontend.ansi_styling());
    frontend.render_text("first\nsecond").unwrap();
    assert_eq!(String::from_utf8(output).unwrap(), "first\nsecond\n");
    assert!(errors.is_empty());
}

#[test]
fn a_session_reads_a_metacommand_as_one_line() {
    // Its trailing `:` would leave Frost source unfinished. Read alone, the
    // source it disassembles ends early.
    let (output, errors) = session("def x = 1\n:bindings\n:disassemble if x:\n2\n");
    assert_eq!(output, "> > results  Array\nx        Int\n> > 2\n> \n");
    assert!(
        errors.contains("unexpected end of input"),
        "the `:disassemble` should fail to parse: {errors}"
    );
}
