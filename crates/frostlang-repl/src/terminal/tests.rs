//! White-box: how `:strings` and the configuration set how String results are
//! shown, and how diagnostics are drawn.
//! The frontend writes results to the process's standard output, and
//! diagnostics to its standard error, which the public API cannot observe, so
//! these read the setting it keeps or the text it would write.
//!
//! A refused argument is reported on standard error, as at a terminal, so
//! those cases print a line there.

use frostlang::Value;

use crate::terminal::settings::StringDisplay;
use crate::{Frontend, Invocation, TerminalFrontend};

/// Run `segment`, a `:strings` metacommand, on `frontend`.
#[track_caller]
fn run(frontend: &mut TerminalFrontend, segment: &str) {
    let invocation = Invocation::parse(segment).expect("a metacommand");
    frontend
        .metacommand(&invocation)
        .expect("the metacommand does not end the session");
}

#[test]
fn strings_sets_how_string_results_are_shown() {
    let mut frontend = TerminalFrontend::new();
    assert_eq!(frontend.strings, StringDisplay::Quoted, "the default");
    run(&mut frontend, ":strings plain");
    assert_eq!(
        frontend.strings,
        StringDisplay::Plain,
        "after `:strings plain`"
    );
    run(&mut frontend, ":strings quoted");
    assert_eq!(
        frontend.strings,
        StringDisplay::Quoted,
        "after `:strings quoted`"
    );
}

#[test]
fn strings_shows_the_setting_without_changing_it() {
    let mut frontend = TerminalFrontend::new();
    run(&mut frontend, ":strings plain");
    run(&mut frontend, ":strings");
    assert_eq!(
        frontend.strings,
        StringDisplay::Plain,
        "a bare `:strings` changes nothing"
    );
}

#[test]
fn strings_refuses_anything_but_a_name_and_changes_nothing() {
    for argument in ["nope", "Plain", "plain quoted"] {
        let mut frontend = TerminalFrontend::new();
        run(&mut frontend, ":strings plain");
        run(&mut frontend, &format!(":strings {argument}"));
        assert_eq!(
            frontend.strings,
            StringDisplay::Plain,
            "`:strings {argument}` changes nothing"
        );
    }
}

#[test]
fn configuring_applies_the_sections_strings_or_the_default() {
    let mut frontend = TerminalFrontend::new().with_color(false);
    let configure = |frontend: &mut TerminalFrontend, section| {
        frontend
            .configure(section)
            .expect("configuring does not end the session");
    };
    configure(
        &mut frontend,
        Some(Value::map([("strings", Value::from("plain"))])),
    );
    assert_eq!(frontend.strings, StringDisplay::Plain, "the section's");
    configure(&mut frontend, None);
    assert_eq!(frontend.strings, StringDisplay::Quoted, "no section");
    configure(
        &mut frontend,
        Some(Value::map([("strings", Value::from("raw"))])),
    );
    assert_eq!(
        frontend.strings,
        StringDisplay::Quoted,
        "an invalid section sets nothing"
    );
}

#[cfg(feature = "graphical-diagnostics")]
#[test]
fn diagnostics_draw_in_the_charset_and_color_the_frontend_shows() {
    use frostlang::compile::{CompilerOptions, compile_program};

    let Err(diagnostics) = compile_program("script.frst", "nope", CompilerOptions::new()) else {
        panic!("an unbound name does not compile");
    };
    for unicode in [false, true] {
        for color in [false, true] {
            let frontend = TerminalFrontend::new()
                .with_unicode(unicode)
                .with_color(color);
            let drawn = frontend.render_diagnostics(&diagnostics);
            let style = format!("unicode {unicode}, color {color}");
            assert!(drawn.contains("nope"), "{style}: {drawn}");
            assert_eq!(
                drawn.contains('\x1b'),
                color,
                "{style}: colored exactly when the frontend is, but gave {drawn:?}"
            );
            assert_eq!(
                drawn.is_ascii(),
                !unicode,
                "{style}: ASCII exactly when the frontend is, but gave {drawn:?}"
            );
        }
    }
}
