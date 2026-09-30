//! `LineInput`, the one-line-per-segment input.

use std::io::Cursor;

use frost_repl::{LineInput, ReplInput};

/// Every segment `input` reads, until it ends.
fn read_all(input: LineInput<Cursor<&str>, &mut Vec<u8>>) -> Vec<String> {
    let mut input = input;
    let mut segments = Vec::new();
    while let Some(segment) = input.read_segment().unwrap() {
        segments.push(segment);
    }
    segments
}

#[test]
fn each_line_is_a_segment_without_its_line_ending() {
    let mut prompts = Vec::new();
    let segments = read_all(LineInput::new(
        Cursor::new("1 + 2\ndef x = 1\r\n\nlast"),
        &mut prompts,
    ));
    assert_eq!(segments, ["1 + 2", "def x = 1", "", "last"]);
}

#[test]
fn a_prompt_comes_before_each_line_and_a_newline_at_the_end() {
    let mut prompts = Vec::new();
    read_all(LineInput::new(Cursor::new("a\nb\n"), &mut prompts));
    assert_eq!(String::from_utf8(prompts).unwrap(), "> > > \n");
}

#[test]
fn the_prompt_can_be_changed() {
    let mut prompts = Vec::new();
    read_all(LineInput::new(Cursor::new("a\n"), &mut prompts).with_prompt("frost> "));
    assert_eq!(String::from_utf8(prompts).unwrap(), "frost> frost> \n");
}

#[test]
fn empty_input_ends_at_once() {
    let mut prompts = Vec::new();
    assert!(read_all(LineInput::new(Cursor::new(""), &mut prompts)).is_empty());
}
