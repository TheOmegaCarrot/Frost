//! Runs the corpus: one trial per script, then a summary of the verdicts.
//! See `corpus/README.md`.

use std::path::Path;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use frostlang_corpus::{Mode, Tally, check, discover};
use libtest_mimic::{Arguments, Trial};

fn main() -> ExitCode {
    let arguments = Arguments::from_args();
    let mode = Mode::from_env();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let entries = discover(&root).expect("the corpus is readable");
    let tally = Arc::new(Mutex::new(Tally::new(mode)));
    let trials = entries
        .into_iter()
        .map(|entry| {
            let tally = Arc::clone(&tally);
            Trial::test(entry.name.clone(), move || {
                let verdict = check(&entry, mode);
                tally.lock().unwrap().record(&entry.name, &verdict);
                match verdict.report.failure() {
                    Some(reason) => Err(reason.into()),
                    None => Ok(()),
                }
            })
        })
        .collect();
    let conclusion = libtest_mimic::run(&arguments, trials);
    if !arguments.list {
        println!("{}\n", tally.lock().unwrap());
    }
    conclusion.exit_code()
}
