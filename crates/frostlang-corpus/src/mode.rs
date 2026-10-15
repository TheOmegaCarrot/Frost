use frostlang::compile::{CompilerOptions, Optimization, OptimizationOptions};

/// Which compiler options each script runs under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The compiler's default options only: the quick check.
    Default,
    /// No optimization, every optimization, each one alone, and all but each
    /// one, which must all agree.
    Permutations,
    /// Every permutation of the optimization options, which must all agree.
    Exhaustive,
}

impl Mode {
    /// The mode the environment selects: [`Exhaustive`](Self::Exhaustive) with
    /// `EXHAUSTIVE_COMPILER_TESTS` set, otherwise
    /// [`Permutations`](Self::Permutations) with `FROST_CORPUS_PERMUTATIONS`
    /// set, otherwise [`Default`](Self::Default). Each is set to any value.
    pub fn from_env() -> Self {
        if std::env::var_os("EXHAUSTIVE_COMPILER_TESTS").is_some() {
            Self::Exhaustive
        } else if std::env::var_os("FROST_CORPUS_PERMUTATIONS").is_some() {
            Self::Permutations
        } else {
            Self::Default
        }
    }

    /// The options to run under. The first is the baseline the others must
    /// agree with.
    pub fn options(self) -> Vec<CompilerOptions> {
        let defaults = CompilerOptions::new();
        let count = Optimization::ALL.len();
        let all: u64 = (1 << count) - 1;
        // The same selection as the `frostlang` crate's own script tests make.
        let bits: Vec<u64> = match self {
            Self::Default => return vec![defaults],
            Self::Exhaustive => (0..=all).collect(),
            Self::Permutations => {
                let alone = (0..count).map(|index| 1 << index);
                let all_but = alone.clone().map(|bit| all & !bit);
                [0, all].into_iter().chain(alone).chain(all_but).collect()
            }
        };
        bits.into_iter()
            .map(|bits| defaults.with_optimization(optimization_from_bits(bits)))
            .collect()
    }

    /// A short description, for the summary.
    pub fn describe(self) -> &'static str {
        match self {
            Self::Default => "default optimizations",
            Self::Permutations => "optimization permutations",
            Self::Exhaustive => "every optimization permutation",
        }
    }
}

/// The options with `Optimization::ALL[n]` on where bit `n` of `bits` is set.
fn optimization_from_bits(bits: u64) -> OptimizationOptions {
    Optimization::ALL.iter().enumerate().fold(
        OptimizationOptions::NONE,
        |options, (index, &optimization)| options.with(optimization, bits & (1 << index) != 0),
    )
}
