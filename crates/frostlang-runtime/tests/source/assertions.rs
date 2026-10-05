//! Assertions shared by the library tests: those of the globals (`library_*`)
//! and of the standard library's modules (`std_*`).
//!
//! A test file declares its [`Library`] with [`library_assertions!`], which
//! defines the assertions as free functions in that file.

use std::fmt::Display;
use std::sync::Arc;

use frostlang_runtime::{Importer, ImporterBuilder, Stdlib, StdlibModule};

use crate::source::Script;

/// What a test's scripts run with: the globals alone, or one standard library
/// module besides, bound to a name.
#[derive(Clone, Copy)]
pub(crate) struct Library {
    module: Option<Module>,
}

#[derive(Clone, Copy)]
struct Module {
    make: fn() -> StdlibModule,
    binding: &'static str,
}

impl Library {
    /// The globals alone.
    pub(crate) const GLOBALS: Library = Library { module: None };

    /// The module `make` makes, alone, bound as `binding`. Each script gets a
    /// fresh module, so no state carries over from one to the next.
    pub(crate) const fn module(make: fn() -> StdlibModule, binding: &'static str) -> Library {
        Library {
            module: Some(Module { make, binding }),
        }
    }

    /// An importer providing only the module, newly made.
    pub(crate) fn importer(&self) -> Arc<Importer> {
        let module = self.module.expect("a module library");
        let stdlib = Stdlib::new()
            .with_module((module.make)())
            .expect("a lone module is accepted");
        ImporterBuilder::new().with_stdlib(stdlib).build()
    }

    /// `expression` as a script, binding the module first if there is one.
    pub(crate) fn source(&self, expression: &str) -> String {
        match self.module {
            None => expression.to_string(),
            Some(module) => format!(
                r"
                def {} = import('std.{}')
                {expression}
                ",
                module.binding,
                (module.make)().name()
            ),
        }
    }

    /// `expression`, ready to run.
    pub(crate) fn script(&self, expression: &str) -> Script {
        let script = Script::new(&self.source(expression));
        match self.module {
            None => script,
            Some(_) => script.importer(self.importer()),
        }
    }

    /// Assert each `expression` runs to the value of the Frost expression
    /// `expected`.
    pub(crate) fn assert_values(&self, cases: &[(&str, &str)]) {
        for (expression, expected) in cases {
            assert_eq!(
                self.script(expression).run(),
                self.script(expected).run(),
                "{expression:?} is {expected}"
            );
        }
    }

    /// Assert each `expression` raises exactly `message`.
    pub(crate) fn assert_raises(&self, cases: &[(&str, &str)]) {
        for (expression, message) in cases {
            assert_eq!(self.script(expression).raises(), *message, "{expression:?}");
        }
    }

    /// Assert each `expression` raises exactly what `equivalent` raises.
    pub(crate) fn assert_raises_as(&self, cases: &[(&str, &str)]) {
        for (expression, equivalent) in cases {
            assert_eq!(
                self.script(expression).raises(),
                self.script(equivalent).raises(),
                "{expression:?} raises as {equivalent:?} does"
            );
        }
    }

    /// Assert `function`, a global or a path within the module such as
    /// `b64.encode`, raises its arity error when called with each count in
    /// `counts`. `expects` is how the error states its arity, such as `1` or
    /// `"between 2 and 3"`.
    pub(crate) fn assert_arity(&self, function: &str, expects: impl Display, counts: &[usize]) {
        let (call, name) = match self.module {
            None => (function.to_string(), function.to_string()),
            Some(module) => (
                format!("{}.{function}", module.binding),
                format!("{}.{function}", (module.make)().name()),
            ),
        };
        self.assert_arity_of(&call, &name, expects, counts);
    }

    /// Assert `call`, an expression for a function such as `r.int`, raises the
    /// arity error of the function named `function` when called with each count
    /// in `counts`. `expects` is as for [`Library::assert_arity`].
    pub(crate) fn assert_arity_of(
        &self,
        call: &str,
        function: &str,
        expects: impl Display,
        counts: &[usize],
    ) {
        for &argc in counts {
            let expression = format!("{call}({})", vec!["null"; argc].join(", "));
            assert_eq!(
                self.script(&expression).raises(),
                format!(
                    "Function {function} expects {expects} arguments, but was called with {argc}"
                ),
                "{expression:?}"
            );
        }
    }

    /// What `expression` prints.
    pub(crate) fn printed(&self, expression: &str) -> Vec<String> {
        self.script(expression).printed()
    }
}

/// Defines, in the test file it is used in, a `LIBRARY` constant of the
/// [`Library`] given, and each of its methods as a free function over it.
// Each test binary uses its own subset of the harness.
#[allow(unused_macros)]
macro_rules! library_assertions {
    ($library:expr) => {
        const LIBRARY: $crate::source::assertions::Library = $library;

        #[allow(dead_code)]
        fn script(expression: &str) -> $crate::source::Script {
            LIBRARY.script(expression)
        }

        #[allow(dead_code)]
        fn assert_values(cases: &[(&str, &str)]) {
            LIBRARY.assert_values(cases)
        }

        #[allow(dead_code)]
        fn assert_raises(cases: &[(&str, &str)]) {
            LIBRARY.assert_raises(cases)
        }

        #[allow(dead_code)]
        fn assert_raises_as(cases: &[(&str, &str)]) {
            LIBRARY.assert_raises_as(cases)
        }

        #[allow(dead_code)]
        fn assert_arity(function: &str, expects: impl std::fmt::Display, counts: &[usize]) {
            LIBRARY.assert_arity(function, expects, counts)
        }

        #[allow(dead_code)]
        fn assert_arity_of(
            call: &str,
            function: &str,
            expects: impl std::fmt::Display,
            counts: &[usize],
        ) {
            LIBRARY.assert_arity_of(call, function, expects, counts)
        }

        #[allow(dead_code)]
        fn printed(expression: &str) -> Vec<String> {
            LIBRARY.printed(expression)
        }
    };
}

#[allow(unused_imports)]
pub(crate) use library_assertions;
