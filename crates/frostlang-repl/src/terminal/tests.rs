//! White-box: how `:strings` and the configuration set how String results are
//! shown.
//! The frontend writes results to the process's standard output, which the
//! public API cannot observe, so these read the setting it keeps.
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
