//! [`ScriptedFrontend`]: a frontend driven by code, for testing.

use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use frost_runtime::Value;

use crate::{Frontend, ReplError};

/// A [`Frontend`] driven by code: it reads the segments it was given, then
/// ends, and records what it is shown in its [`Transcript`].
///
/// ```
/// use frost_repl::{Repl, ScriptedFrontend};
/// use frost_runtime::Value;
///
/// let mut frontend = ScriptedFrontend::new(["def x = 20", "x + 1"]);
/// Repl::new().run(&mut frontend).unwrap();
/// let values: Vec<Value> = frontend
///     .transcript()
///     .outcomes()
///     .into_iter()
///     .map(Result::unwrap)
///     .collect();
/// assert_eq!(values, [Value::Null, Value::Int(21)]);
/// ```
pub struct ScriptedFrontend {
    segments: VecDeque<String>,
    transcript: Transcript,
    ansi_styling: bool,
}

impl ScriptedFrontend {
    /// A frontend that reads `segments` in order, recording to a new
    /// [`Transcript`], and takes text without ANSI styling.
    pub fn new<S: Into<String>>(segments: impl IntoIterator<Item = S>) -> Self {
        Self {
            segments: segments.into_iter().map(Into::into).collect(),
            transcript: Transcript::default(),
            ansi_styling: false,
        }
    }

    /// Record to `transcript` instead, after whatever it holds already.
    ///
    /// This lets a test read what a frontend was shown once it cannot reach
    /// the frontend, such as one a factory made.
    pub fn with_transcript(mut self, transcript: Transcript) -> Self {
        self.transcript = transcript;
        self
    }

    /// Set what [`Frontend::ansi_styling`] answers.
    pub fn with_ansi_styling(mut self, ansi_styling: bool) -> Self {
        self.ansi_styling = ansi_styling;
        self
    }

    /// The transcript this frontend records to.
    pub fn transcript(&self) -> Transcript {
        self.transcript.clone()
    }
}

impl Frontend for ScriptedFrontend {
    fn read_segment(&mut self) -> io::Result<Option<String>> {
        Ok(self.segments.pop_front())
    }

    fn render(&mut self, outcome: Result<&Value, &ReplError>) -> io::Result<()> {
        self.transcript
            .lock()
            .outcomes
            .push(outcome.cloned().map_err(Clone::clone));
        Ok(())
    }

    fn render_text(&mut self, text: &str) -> io::Result<()> {
        self.transcript.lock().texts.push(text.to_string());
        Ok(())
    }

    fn ansi_styling(&self) -> bool {
        self.ansi_styling
    }
}

/// What a [`ScriptedFrontend`] was shown, in order. A clone shares the same
/// record.
#[derive(Clone, Default, Debug)]
pub struct Transcript(Arc<Mutex<Record>>);

#[derive(Default, Debug)]
struct Record {
    outcomes: Vec<Result<Value, ReplError>>,
    texts: Vec<String>,
}

impl Transcript {
    /// A copy of every outcome [rendered](Frontend::render) so far.
    pub fn outcomes(&self) -> Vec<Result<Value, ReplError>> {
        self.lock().outcomes.clone()
    }

    /// A copy of every text [rendered](Frontend::render_text) so far.
    pub fn texts(&self) -> Vec<String> {
        self.lock().texts.clone()
    }

    fn lock(&self) -> MutexGuard<'_, Record> {
        // A panic mid-push leaves the record whole: each push is a single step.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
