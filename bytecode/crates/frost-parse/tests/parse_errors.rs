use frost_parse::parse::parse_program;

#[test]
fn lex_error_on_invalid_character() {
    let result = parse_program("test.frst", "def x = ~");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("unexpected character"), "error was: {err}");
}

#[test]
fn lex_error_includes_filename() {
    let result = parse_program("my_script.frst", "def x = ~");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("my_script.frst"), "error was: {err}");
}

#[test]
fn lex_error_points_at_bad_character() {
    let result = parse_program("test.frst", "def x = ~");
    let err = result.unwrap_err().to_string();
    // The error should mention "unrecognized" (the label text)
    assert!(err.contains("unrecognized"), "error was: {err}");
}

#[test]
fn lex_error_on_invalid_character_mid_input() {
    let result = parse_program("test.frst", "def x = 42\ndef y = ~\ndef z = 1");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("unexpected character"), "error was: {err}");
}

#[test]
fn lex_error_on_unclosed_string() {
    let result = parse_program("test.frst", "def x = 'hello");
    assert!(result.is_err());
}

#[test]
fn lex_error_on_unclosed_format_string() {
    let result = parse_program("test.frst", "def x = $'hello ${name}");
    assert!(result.is_err());
}

#[test]
fn valid_input_does_not_lex_error() {
    // This will hit the todo!() in parse_program, but it should NOT
    // fail during lexing. We catch the panic to verify lexing succeeded.
    let result = std::panic::catch_unwind(|| {
        parse_program("test.frst", "def x = 42")
    });
    // Should panic (todo!) rather than returning Err (lex failure)
    assert!(result.is_err(), "expected todo!() panic, not a lex error");
}
