//! `LineIndex`: byte offsets to 1-based lines and columns, the column counted
//! in bytes, with lines ending at LF as the parser reads them.

use frostlang_parse::LineIndex;

/// Where `offset` falls in `source`, as `(line, column)`.
fn location(source: &str, offset: u32) -> (u32, u32) {
    let location = LineIndex::new(source).location(offset);
    (location.line(), location.column())
}

#[test]
fn the_first_byte_is_line_1_column_1() {
    assert_eq!(location("abc", 0), (1, 1));
}

#[test]
fn columns_advance_along_a_line() {
    assert_eq!(location("abc", 2), (1, 3));
}

#[test]
fn a_line_starts_after_each_lf() {
    let source = "ab\ncd\n\nef";
    assert_eq!(location(source, 3), (2, 1), "`c` starts line 2");
    assert_eq!(location(source, 4), (2, 2), "`d`");
    assert_eq!(location(source, 6), (3, 1), "the empty line 3");
    assert_eq!(location(source, 7), (4, 1), "`e` starts line 4");
}

#[test]
fn an_lf_ends_its_own_line() {
    assert_eq!(location("ab\ncd", 2), (1, 3));
}

#[test]
fn a_crlf_ends_a_line_once() {
    let source = "ab\r\ncd";
    assert_eq!(location(source, 2), (1, 3), "the CR is on line 1");
    assert_eq!(location(source, 3), (1, 4), "the LF is on line 1");
    assert_eq!(location(source, 4), (2, 1), "`c` starts line 2");
}

#[test]
fn a_lone_cr_ends_no_line() {
    assert_eq!(location("ab\rcd", 3), (1, 4), "`c` stays on line 1");
}

#[test]
fn columns_count_bytes_not_characters() {
    // `é` is two bytes and `𝄞` four.
    let source = "é𝄞x";
    assert_eq!(location(source, 2), (1, 3), "`𝄞` follows two bytes");
    assert_eq!(location(source, 6), (1, 7), "`x` follows six bytes");
}

#[test]
fn columns_count_bytes_from_the_start_of_their_own_line() {
    let source = "ééé\néx";
    assert_eq!(location(source, 7), (2, 1), "`é` starts line 2");
    assert_eq!(
        location(source, 9),
        (2, 3),
        "`x` follows two bytes on line 2"
    );
}

#[test]
fn an_offset_inside_a_character_is_its_own_column() {
    // `𝄞` is bytes 1 to 4.
    for offset in 1..5 {
        assert_eq!(location("a𝄞b", offset), (1, offset + 1), "offset {offset}");
    }
}

#[test]
fn the_end_of_the_source_is_one_past_its_last_byte() {
    assert_eq!(location("ab", 2), (1, 3));
    assert_eq!(
        location("ab\n", 3),
        (2, 1),
        "after a final LF, the end is on a new line"
    );
    assert_eq!(location("", 0), (1, 1), "an empty source");
}

#[test]
fn an_offset_past_the_end_is_placed_as_if_the_last_line_went_on() {
    assert_eq!(location("ab\ncd", 100), (2, 98));
    assert_eq!(location("", 7), (1, 8));
}

#[test]
fn equal_sources_give_equal_indexes() {
    assert_eq!(LineIndex::new("a\nb"), LineIndex::new("x\ny"));
    assert_ne!(LineIndex::new("a\nb"), LineIndex::new("ab\n"));
}

#[test]
fn a_location_displays_as_line_colon_column() {
    let location = LineIndex::new("a\nbc").location(3);
    assert_eq!(location.to_string(), "2:2");
}

#[test]
fn locations_order_by_line_then_column() {
    let index = LineIndex::new("abc\nd");
    assert!(
        index.location(2) < index.location(4),
        "line 1 column 3 before line 2 column 1"
    );
    assert!(
        index.location(0) < index.location(1),
        "column 1 before column 2"
    );
}
