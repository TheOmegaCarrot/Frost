//! [`ScriptedFrontend`]: a frontend driven by code, for testing.

use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use frost_runtime::Value;

use crate::{Frontend, ReplError};

/// A [`Frontend`] driven by code: it reads the segments it was given, then
/// ends, and records the outcome of each in its [`Transcript`].
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
}

impl ScriptedFrontend {
    /// A frontend that reads `segments` in order, recording to a new
    /// [`Transcript`].
    pub fn new<S: Into<String>>(segments: impl IntoIterator<Item = S>) -> Self {
        Self {
            segments: segments.into_iter().map(Into::into).collect(),
            transcript: Transcript::default(),
        }
    }

    /// Record to `transcript` instead, after whatever it holds already.
    ///
    /// This lets a test read the outcomes of a frontend it cannot reach once
    /// the session is over, such as one made by a factory.
    pub fn with_transcript(mut self, transcript: Transcript) -> Self {
        self.transcript = transcript;
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
            .push(outcome.cloned().map_err(Clone::clone));
        Ok(())
    }
}

/// The outcomes a [`ScriptedFrontend`] recorded, in order. A clone shares the
/// same record.
#[derive(Clone, Default, Debug)]
pub struct Transcript(Arc<Mutex<Vec<Result<Value, ReplError>>>>);

impl Transcript {
    /// A copy of every outcome recorded so far.
    pub fn outcomes(&self) -> Vec<Result<Value, ReplError>> {
        self.lock().clone()
    }

    fn lock(&self) -> MutexGuard<'_, Vec<Result<Value, ReplError>>> {
        // A panic mid-push leaves the record whole: each push is a single step.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
