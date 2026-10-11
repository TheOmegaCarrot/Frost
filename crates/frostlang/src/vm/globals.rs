//! Predefined globals available in every Frost program without importing.
//!
//! The `define_globals!` invocation below is the single source of truth for the
//! global names, their slot order (used by `LoadGlobal`), and their constructors.
//! Each global is built by a constructor that lives in a topic submodule: a
//! `*_global()` function, or the `*_frost()` one that `#[frost_fn]` generates.
//! Some globals are written in Frost; see [`generated`].

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

use crate::bytecode::{Bytecode, CompiledFunction, FormatVersion};
use crate::{Arity, Closure, Value};

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
    "is_opaque"             [Pure]   => is_opaque_global(),
    "is_nonnull"            [Pure]   => is_nonnull_global(),
    "is_numeric"            [Pure]   => is_numeric_global(),
    "is_primitive"          [Pure]   => is_primitive_global(),
    "is_structured"         [Pure]   => is_structured_global(),
    "is_flat"               [Pure]   => is_flat_global(),
    "type"                  [Pure]   => type_frost(),
    "to_string"             [Pure]   => to_string_global(),
    "pretty"                [Pure]   => pretty_frost(),
    "to_int"                [Pure]   => to_int_global(),
    "to_float"              [Pure]   => to_float_global(),
    "to_bytes"              [Pure]   => to_bytes_frost(),
    "from_utf8"             [Pure]   => from_utf8_frost(),

    // --- Strings ---
    "split"                 [Pure]   => split_frost(),
    "split_once"            [Pure]   => split_once_frost(),
    "lines"                [Pure]   => lines_frost(),
    "join"                  [Pure]   => join_frost(),
    "replace"               [Pure]   => replace_frost(),
    "trim"                  [Pure]   => trim_global(),
    "trim_left"             [Pure]   => trim_left_global(),
    "trim_right"            [Pure]   => trim_right_global(),
    "to_upper"              [Pure]   => to_upper_global(),
    "to_lower"              [Pure]   => to_lower_global(),
    "contains"              [Pure]   => contains_frost(),
    "starts_with"           [Pure]   => starts_with_frost(),
    "ends_with"             [Pure]   => ends_with_frost(),

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
    "keys"                  [Pure]   => keys_frost(),
    "values"                [Pure]   => values_frost(),
    "map_keys"              [Pure]   => map_keys_frost(),
    "map_values"            [Pure]   => map_values_frost(),
    "len"                   [Pure]   => len_global(),
    "range"                 [Pure]   => range_frost(),
    "nulls"                 [Pure]   => nulls_frost(),
    "repeat"                [Pure]   => repeat_frost(),
    "tile"                  [Pure]   => tile_global(),
    "id"                    [Pure]   => id_global(),
    "has"                   [Pure]   => has_frost(),
    "includes"              [Pure]   => includes_frost(),
    "index"                 [Pure]   => index_global(),
    "dig"                   [Pure]   => dig_global(),
    "slice"                 [Pure]   => slice_global(),
    "stride"                [Pure]   => stride_global(),
    "take"                  [Pure]   => take_global(),
    "drop"                  [Pure]   => drop_global(),
    "tail"                  [Pure]   => tail_global(),
    "drop_tail"             [Pure]   => drop_tail_global(),
    "slide"                 [Pure]   => slide_frost(),
    "chunk"                 [Pure]   => chunk_frost(),
    "reverse"               [Pure]   => reverse_global(),
    "take_while"            [Pure]   => take_while_frost(),
    "drop_while"            [Pure]   => drop_while_frost(),
    "chunk_by"              [Pure]   => chunk_by_frost(),
    "flatten"               [Pure]   => flatten_frost(),
    "zip"                   [Pure]   => zip_frost(),
    "zip_with"              [Pure]   => zip_with_frost(),
    "xprod"                 [Pure]   => xprod_frost(),
    "xprod_with"            [Pure]   => xprod_with_frost(),
    "transform"             [Pure]   => transform_frost(),
    "flat_map"              [Pure]   => flat_map_frost(),
    "select"                [Pure]   => select_frost(),
    "reject"                [Pure]   => reject_frost(),
    "fold"                  [Pure]   => fold_frost(),
    "sum"                   [Pure]   => sum_frost(),
    "product"               [Pure]   => product_frost(),
    "sorted"                [Pure]   => sorted_frost(),
    "sort_by"               [Pure]   => sort_by_frost(),
    "minimum"               [Pure]   => minimum_global(),
    "maximum"               [Pure]   => maximum_global(),
    "min_by"                [Pure]   => min_by_global(),
    "max_by"                [Pure]   => max_by_global(),
    "any"                   [Pure]   => any_frost(),
    "all"                   [Pure]   => all_frost(),
    "none"                  [Pure]   => none_frost(),
    "find"                  [Pure]   => find_frost(),
    "find_index"            [Pure]   => find_index_frost(),
    "unique"                [Pure]   => unique_frost(),
    "unique_by"             [Pure]   => unique_by_frost(),
    "group_by"              [Pure]   => group_by_frost(),
    "count_by"              [Pure]   => count_by_frost(),
    "scan"                  [Pure]   => scan_frost(),
    "partition"             [Pure]   => partition_frost(),
    "map_into"              [Pure]   => map_into_frost(),
    "to_entries"            [Pure]   => to_entries_frost(),
    "from_entries"          [Pure]   => from_entries_frost(),
    "dissoc"                [Pure]   => dissoc_frost(),
    "each"                  [Pure]   => each_frost(),

    // --- Output ---
    "print"                 [Impure] => print_frost(),
    "mformat"               [Pure]   => mformat_frost(),
    "mprint"                [Impure] => mprint_frost(),

    // --- Functions / combinators ---
    "call"                  [Pure]   => call_global(),
    "try_call"              [Pure]   => try_call_frost(),
    "error"                 [Pure]   => error_global(),
    "and_then"              [Pure]   => and_then_frost(),
    "or_else"               [Pure]   => or_else_frost(),
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
    "assert"                [Pure]   => assert_frost(),
    "debug_dump"            [Pure]   => debug_dump_frost(),

    // --- Mutable cell ---
    "mutable_cell"          [Impure] => mutable_cell_frost(),

    // --- Import ---
    "imported"              [Impure] => imported_frost(),
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
            origin: None,
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
