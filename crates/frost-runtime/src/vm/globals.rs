//! Predefined globals available in every Frost program without importing.
//!
//! The `define_globals!` invocation below is the single source of truth for the
//! global names, their slot order (used by `LoadGlobal`), and their constructors.
//! Each global is built by a `*_global()` constructor that lives in a topic
//! submodule. Some globals are written in Frost; see [`generated`].

mod collections;
mod debug;
mod functions;
mod generated;
mod import;
mod mutable_cell;
mod operators;
mod output;
mod strings;
mod types;

use std::sync::{Arc, LazyLock};

use crate::{Arity, Bytecode, Closure, CompiledFunction, FormatVersion, Value};

use collections::*;
use debug::*;
use functions::*;
use import::*;
use mutable_cell::*;
use operators::*;
use output::*;
use strings::*;
use types::*;

/// Whether a predefined global may be evaluated at compile time.
///
/// A `Pure` global is deterministic and free of side effects, host interaction,
/// and mutable state, so a call to it over constant arguments can be constant
/// folded. `Impure` globals (I/O, mutation, and imports) are never folded.
/// Purity of a callback argument is a separate matter: an impure callback keeps
/// its own call site unfoldable regardless of this flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purity {
    /// Eligible for constant folding.
    Pure,
    /// Never constant folded.
    Impure,
}

// The fixed set of predefined globals, shared by every `Vm`; never host-configurable.
// The compiler's seam onto it is `GLOBAL_NAMES` and `GLOBAL_PURITY`.
#[derive(Debug, Clone)]
pub(crate) struct GlobalSet(Vec<Value>);

// Sync-only macro: each `name [purity] => init` entry expands into the name,
// purity, and slot initializer lists in the same order, so a global's slot in
// `GLOBAL_NAMES`, `GLOBAL_PURITY`, and the value vector cannot drift apart.
macro_rules! define_globals {
    ($($name:literal [$purity:ident] => $init:expr),* $(,)?) => {
        /// The predefined global names, in slot order:
        /// a name's index in this list is its `LoadGlobal` slot.
        /// The set is fixed: a host supplies its own values through captures or imports instead.
        pub const GLOBAL_NAMES: &[&str] = &[ $($name),* ];

        /// The [`Purity`] of each global, in the same slot order as
        /// [`GLOBAL_NAMES`]: the compiler's seam for deciding whether a call to
        /// a global is a candidate for constant folding.
        pub const GLOBAL_PURITY: &[Purity] = &[ $(Purity::$purity),* ];

        impl GlobalSet {
            fn build_defaults() -> Self {
                GlobalSet ( vec![ $($init),* ] )
            }
        }
    };
}

// The `[Pure]`/`[Impure]` tag is each global's compile-time-fold eligibility; see `Purity`.
// Default to `Pure`; `Impure` marks the only globals with an effect or context of their own:
// the I/O of print/mprint, mutable_cell's mutable state,
// import (the gateway to every other effect), and imported (which reads the running Vm).
// A higher-order global stays `Pure` (an impure callback stops its own fold),
// and so does a raising one (error/assert): a fold that reaches a raise just abandons.
define_globals! {
    // --- Types ---
    "is_null"               [Pure]   => is_null_global(),
    "is_int"                [Pure]   => is_int_global(),
    "is_float"              [Pure]   => is_float_global(),
    "is_bool"               [Pure]   => is_bool_global(),
    "is_string"             [Pure]   => is_string_global(),
    "is_bytes"              [Pure]   => is_bytes_global(),
    "is_array"              [Pure]   => is_array_global(),
    "is_map"                [Pure]   => is_map_global(),
    "is_function"           [Pure]   => is_function_global(),
    "is_nonnull"            [Pure]   => is_nonnull_global(),
    "is_numeric"            [Pure]   => is_numeric_global(),
    "is_primitive"          [Pure]   => is_primitive_global(),
    "is_structured"         [Pure]   => is_structured_global(),
    "is_flat"               [Pure]   => is_flat_global(),
    "type"                  [Pure]   => type_global(),
    "to_string"             [Pure]   => to_string_global(),
    "pretty"                [Pure]   => pretty_global(),
    "to_int"                [Pure]   => to_int_global(),
    "to_float"              [Pure]   => to_float_global(),
    "to_bytes"              [Pure]   => to_bytes_global(),
    "from_utf8"             [Pure]   => from_utf8_global(),

    // --- Strings ---
    "split"                 [Pure]   => split_global(),
    "lines"                 [Pure]   => lines_global(),
    "join"                  [Pure]   => join_global(),
    "replace"               [Pure]   => replace_global(),
    "trim"                  [Pure]   => trim_global(),
    "trim_left"             [Pure]   => trim_left_global(),
    "trim_right"            [Pure]   => trim_right_global(),
    "to_upper"              [Pure]   => to_upper_global(),
    "to_lower"              [Pure]   => to_lower_global(),
    "contains"              [Pure]   => contains_global(),
    "starts_with"           [Pure]   => starts_with_global(),
    "ends_with"             [Pure]   => ends_with_global(),

    // --- Operators ---
    "plus"                  [Pure]   => plus_global(),
    "minus"                 [Pure]   => minus_global(),
    "times"                 [Pure]   => times_global(),
    "divide"                [Pure]   => divide_global(),
    "mod"                   [Pure]   => mod_global(),
    "equal"                 [Pure]   => equal_global(),
    "not_equal"             [Pure]   => not_equal_global(),
    "less_than"             [Pure]   => less_than_global(),
    "less_than_or_equal"    [Pure]   => less_than_or_equal_global(),
    "greater_than"          [Pure]   => greater_than_global(),
    "greater_than_or_equal" [Pure]   => greater_than_or_equal_global(),

    // --- Collections ---
    "keys"                  [Pure]   => keys_global(),
    "values"                [Pure]   => values_global(),
    "map_keys"              [Pure]   => map_keys_global(),
    "map_values"            [Pure]   => map_values_global(),
    "len"                   [Pure]   => len_global(),
    "range"                 [Pure]   => range_global(),
    "nulls"                 [Pure]   => nulls_global(),
    "repeat"                [Pure]   => repeat_global(),
    "tile"                  [Pure]   => tile_global(),
    "id"                    [Pure]   => id_global(),
    "has"                   [Pure]   => has_global(),
    "includes"              [Pure]   => includes_global(),
    "index"                 [Pure]   => index_global(),
    "dig"                   [Pure]   => dig_global(),
    "slice"                 [Pure]   => slice_global(),
    "stride"                [Pure]   => stride_global(),
    "take"                  [Pure]   => take_global(),
    "drop"                  [Pure]   => drop_global(),
    "tail"                  [Pure]   => tail_global(),
    "drop_tail"             [Pure]   => drop_tail_global(),
    "slide"                 [Pure]   => slide_global(),
    "chunk"                 [Pure]   => chunk_global(),
    "reverse"               [Pure]   => reverse_global(),
    "take_while"            [Pure]   => take_while_global(),
    "drop_while"            [Pure]   => drop_while_global(),
    "chunk_by"              [Pure]   => chunk_by_global(),
    "flatten"               [Pure]   => flatten_global(),
    "zip"                   [Pure]   => zip_global(),
    "zip_with"              [Pure]   => zip_with_global(),
    "xprod"                 [Pure]   => xprod_global(),
    "xprod_with"            [Pure]   => xprod_with_global(),
    "transform"             [Pure]   => transform_global(),
    "flat_map"              [Pure]   => flat_map_global(),
    "select"                [Pure]   => select_global(),
    "reject"                [Pure]   => reject_global(),
    "fold"                  [Pure]   => fold_global(),
    "sum"                   [Pure]   => sum_global(),
    "product"               [Pure]   => product_global(),
    "sorted"                [Pure]   => sorted_global(),
    "sort_by"               [Pure]   => sort_by_global(),
    "any"                   [Pure]   => any_global(),
    "all"                   [Pure]   => all_global(),
    "none"                  [Pure]   => none_global(),
    "find"                  [Pure]   => find_global(),
    "group_by"              [Pure]   => group_by_global(),
    "count_by"              [Pure]   => count_by_global(),
    "scan"                  [Pure]   => scan_global(),
    "partition"             [Pure]   => partition_global(),
    "map_into"              [Pure]   => map_into_global(),
    "to_entries"            [Pure]   => to_entries_global(),
    "from_entries"          [Pure]   => from_entries_global(),
    "dissoc"                [Pure]   => dissoc_global(),
    "each"                  [Pure]   => each_global(),

    // --- Output ---
    "print"                 [Impure] => print_global(),
    "mformat"               [Pure]   => mformat_global(),
    "mprint"                [Impure] => mprint_global(),

    // --- Functions / combinators ---
    "call"                  [Pure]   => call_global(),
    "try_call"              [Pure]   => try_call_global(),
    "error"                 [Pure]   => error_global(),
    "and_then"              [Pure]   => and_then_global(),
    "or_else"               [Pure]   => or_else_global(),
    "inv"                   [Pure]   => inv_global(),
    "curry"                 [Pure]   => curry_global(),
    "bcurry"                [Pure]   => bcurry_global(),
    "collect"               [Pure]   => collect_global(),
    "spread"                [Pure]   => spread_global(),
    "rev_args"              [Pure]   => rev_args_global(),
    "tap"                   [Pure]   => tap_global(),
    "const"                 [Pure]   => const_global(),
    "compose"               [Pure]   => compose_global(),

    // --- Debug ---
    "assert"                [Pure]   => assert_global(),
    "debug_dump"            [Pure]   => debug_dump_global(),

    // --- Mutable cell ---
    "mutable_cell"          [Impure] => mutable_cell_global(),

    // --- Import ---
    "imported"              [Impure] => imported_global(),
    "import"                [Impure] => import_global(),
}

/// Build a slot-free, capture-free hand-rolled bytecode closure global from `name`,
/// `arity`, and `code`, which follows the calling convention (see [`Bytecode`]).
fn bytecode_global(name: &'static str, arity: Arity, code: Vec<Bytecode>) -> Value {
    Value::Closure(Arc::new(Closure {
        captures: Vec::new(),
        function: Arc::new(CompiledFunction {
            version: FormatVersion,
            name: name.to_string(),
            arity,
            num_captures: 0,
            name_table: Vec::new(),
            constants: Vec::new(),
            key_constants: Vec::new(),
            child_fns: Vec::new(),
            code,
        }),
    }))
}

static DEFAULT_GLOBALS: LazyLock<Arc<GlobalSet>> =
    LazyLock::new(|| Arc::new(GlobalSet::build_defaults()));

impl GlobalSet {
    pub(super) fn defaults() -> Arc<Self> {
        DEFAULT_GLOBALS.clone()
    }

    /// Get the value at a global slot index.
    pub(super) fn get(&self, idx: usize) -> &Value {
        &self.0[idx]
    }
}
