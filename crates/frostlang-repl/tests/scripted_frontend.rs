//! `ScriptedFrontend` and its `Transcript`: a frontend that test code drives.

use frostlang_repl::{Frontend, Repl, ReplError, ScriptedFrontend, Transcript};
use frostlang_runtime::Value;

#[test]
fn it_reads_its_segments_in_order_then_ends_for_good() {
    let mut frontend = ScriptedFrontend::new(["first", "second line\nand more"]);
    assert_eq!(frontend.read_segment().unwrap().as_deref(), Some("first"));
    assert_eq!(
        frontend.read_segment().unwrap().as_deref(),
        Some("second line\nand more")
    );
    for _ in 0..3 {
        assert_eq!(frontend.read_segment().unwrap(), None);
    }
}

#[test]
fn it_records_each_outcome_as_given() {
    let error = Repl::new().evaluate("nope").unwrap_err();
    let mut frontend = ScriptedFrontend::new(Vec::<String>::new());
    frontend.render(Ok(&Value::Int(1))).unwrap();
    frontend.render(Ok(&Value::Null)).unwrap();
    frontend.render(Err(&error)).unwrap();
    let outcomes = frontend.transcript().outcomes();
    assert!(
        matches!(
            outcomes.as_slice(),
            [
                Ok(Value::Int(1)),
                Ok(Value::Null),
                Err(ReplError::Compile(_))
            ]
        ),
        "{outcomes:?}"
    );
    let Err(recorded) = &outcomes[2] else {
        unreachable!()
    };
    assert_eq!(recorded.to_string(), error.to_string());
}

#[test]
fn a_new_frontend_starts_with_an_empty_transcript() {
    let frontend = ScriptedFrontend::new(["1"]);
    assert!(frontend.transcript().outcomes().is_empty());
}

#[test]
fn its_transcript_outlives_it() {
    let mut frontend = ScriptedFrontend::new(["1", "2"]);
    Repl::new().run(&mut frontend).unwrap();
    let transcript = frontend.transcript();
    drop(frontend);
    let outcomes = transcript.outcomes();
    assert!(
        matches!(outcomes.as_slice(), [Ok(Value::Int(1)), Ok(Value::Int(2))]),
        "{outcomes:?}"
    );
}

#[test]
fn frontends_given_one_transcript_record_to_it_in_turn() {
    // As a factory making a frontend for each session would.
    let transcript = Transcript::default();
    for segment in ["1", "2"] {
        let mut frontend = ScriptedFrontend::new([segment]).with_transcript(transcript.clone());
        Repl::new().run(&mut frontend).unwrap();
    }
    let outcomes = transcript.outcomes();
    assert!(
        matches!(outcomes.as_slice(), [Ok(Value::Int(1)), Ok(Value::Int(2))]),
        "{outcomes:?}"
    );
}

#[test]
fn it_records_each_text_apart_from_outcomes() {
    let mut frontend = ScriptedFrontend::new(Vec::<String>::new());
    frontend.render_text("one").unwrap();
    frontend.render(Ok(&Value::Int(2))).unwrap();
    frontend.render_text("three\nlines").unwrap();
    let transcript = frontend.transcript();
    assert_eq!(transcript.texts(), ["one", "three\nlines"]);
    assert!(
        matches!(transcript.outcomes().as_slice(), [Ok(Value::Int(2))]),
        "{:?}",
        transcript.outcomes()
    );
}

#[test]
fn it_takes_ansi_styling_only_if_told_to() {
    assert!(!ScriptedFrontend::new(["1"]).ansi_styling());
    assert!(
        ScriptedFrontend::new(["1"])
            .with_ansi_styling(true)
            .ansi_styling()
    );
    assert!(
        !ScriptedFrontend::new(["1"])
            .with_ansi_styling(false)
            .ansi_styling()
    );
}

#[test]
fn a_transcripts_clones_share_its_record() {
    let transcript = Transcript::default();
    let clone = transcript.clone();
    let mut frontend = ScriptedFrontend::new(Vec::<String>::new()).with_transcript(clone);
    frontend.render(Ok(&Value::Int(7))).unwrap();
    let outcomes = transcript.outcomes();
    assert!(
        matches!(outcomes.as_slice(), [Ok(Value::Int(7))]),
        "{outcomes:?}"
    );
}
