//! Predefined globals available in every Frost program without importing.
//!
//! The `define_globals!` invocation below is the single source of truth for the
//! global names, their slot order (used by `LoadGlobal`), and their constructors.
//! Each global is built by a constructor that lives in a topic submodule: a
//! `*_global()` function, or the definition from the `*_frost()` one that
//! `#[frost_fn]` generates, built under its base name.
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
use crate::{Arity, Closure, Params, Value};

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    "type"                  [Pure]   => type_frost().build(),
    "to_string"             [Pure]   => to_string_global(),
    "pretty"                [Pure]   => pretty_frost().build(),
    "to_int"                [Pure]   => to_int_frost().build(),
    "to_float"              [Pure]   => to_float_frost().build(),
    "to_bytes"              [Pure]   => to_bytes_frost().build(),
    "from_utf8"             [Pure]   => from_utf8_frost().build(),

    // --- Strings ---
    "split"                 [Pure]   => split_frost().build(),
    "split_once"            [Pure]   => split_once_frost().build(),
    "lines"                [Pure]   => lines_frost().build(),
    "join"                  [Pure]   => join_frost().build(),
    "replace"               [Pure]   => replace_frost().build(),
    "trim"                  [Pure]   => trim_global(),
    "trim_left"             [Pure]   => trim_left_global(),
    "trim_right"            [Pure]   => trim_right_global(),
    "to_upper"              [Pure]   => to_upper_global(),
    "to_lower"              [Pure]   => to_lower_global(),
    "contains"              [Pure]   => contains_frost().build(),
    "starts_with"           [Pure]   => starts_with_frost().build(),
    "ends_with"             [Pure]   => ends_with_frost().build(),

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
    "keys"                  [Pure]   => keys_frost().build(),
    "values"                [Pure]   => values_frost().build(),
    "map_keys"              [Pure]   => map_keys_frost().build(),
    "map_values"            [Pure]   => map_values_frost().build(),
    "len"                   [Pure]   => len_frost().build(),
    "range"                 [Pure]   => range_frost().build(),
    "nulls"                 [Pure]   => nulls_frost().build(),
    "repeat"                [Pure]   => repeat_frost().build(),
    "tile"                  [Pure]   => tile_frost().build(),
    "id"                    [Pure]   => id_global(),
    "has"                   [Pure]   => has_frost().build(),
    "index"                 [Pure]   => index_global(),
    "dig"                   [Pure]   => dig_global(),
    "dig_set"               [Pure]   => dig_set_frost().build(),
    "dig_update"            [Pure]   => dig_update_frost().build(),
    "slice"                 [Pure]   => slice_frost().build(),
    "stride"                [Pure]   => stride_frost().build(),
    "take"                  [Pure]   => take_global(),
    "drop"                  [Pure]   => drop_global(),
    "tail"                  [Pure]   => tail_global(),
    "drop_tail"             [Pure]   => drop_tail_global(),
    "slide"                 [Pure]   => slide_frost().build(),
    "chunk"                 [Pure]   => chunk_frost().build(),
    "reverse"               [Pure]   => reverse_frost().build(),
    "take_while"            [Pure]   => take_while_frost().build(),
    "drop_while"            [Pure]   => drop_while_frost().build(),
    "chunk_by"              [Pure]   => chunk_by_frost().build(),
    "flatten"               [Pure]   => flatten_frost().build(),
    "zip"                   [Pure]   => zip_frost().build(),
    "zip_with"              [Pure]   => zip_with_frost().build(),
    "xprod"                 [Pure]   => xprod_frost().build(),
    "xprod_with"            [Pure]   => xprod_with_frost().build(),
    "map"                   [Pure]   => map_frost().build(),
    "flat_map"              [Pure]   => flat_map_frost().build(),
    "filter"                [Pure]   => filter_frost().build(),
    "reject"                [Pure]   => reject_frost().build(),
    "reduce"                [Pure]   => reduce_frost().build(),
    "sum"                   [Pure]   => sum_frost().build(),
    "product"               [Pure]   => product_frost().build(),
    "sorted"                [Pure]   => sorted_frost().build(),
    "sort_by"               [Pure]   => sort_by_frost().build(),
    "minimum"               [Pure]   => minimum_global(),
    "maximum"               [Pure]   => maximum_global(),
    "min_by"                [Pure]   => min_by_global(),
    "max_by"                [Pure]   => max_by_global(),
    "any"                   [Pure]   => any_frost().build(),
    "all"                   [Pure]   => all_frost().build(),
    "none"                  [Pure]   => none_frost().build(),
    "find"                  [Pure]   => find_frost().build(),
    "find_index"            [Pure]   => find_index_frost().build(),
    "unique"                [Pure]   => unique_frost().build(),
    "unique_by"             [Pure]   => unique_by_frost().build(),
    "group_by"              [Pure]   => group_by_frost().build(),
    "count_by"              [Pure]   => count_by_frost().build(),
    "scan"                  [Pure]   => scan_frost().build(),
    "partition"             [Pure]   => partition_frost().build(),
    "map_into"              [Pure]   => map_into_frost().build(),
    "to_entries"            [Pure]   => to_entries_frost().build(),
    "from_entries"          [Pure]   => from_entries_frost().build(),
    "dissoc"                [Pure]   => dissoc_frost().build(),
    "deep_merge"            [Pure]   => deep_merge_frost().build(),
    "each"                  [Pure]   => each_frost().build(),

    // --- Output ---
    "print"                 [Impure] => print_frost().build(),
    "mformat"               [Pure]   => mformat_frost().build(),
    "mprint"                [Impure] => mprint_frost().build(),

    // --- Functions / combinators ---
    "call"                  [Pure]   => call_global(),
    "try_call"              [Pure]   => try_call_frost().build(),
    "error"                 [Pure]   => error_global(),
    "and_then"              [Pure]   => and_then_frost().build(),
    "or_else"               [Pure]   => or_else_frost().build(),
    "inv"                   [Pure]   => inv_global(),
    "curry"                 [Pure]   => curry_global(),
    "bcurry"                [Pure]   => bcurry_global(),
    "collect"               [Pure]   => collect_global(),
    "spread"                [Pure]   => spread_global(),
    "rev_args"              [Pure]   => rev_args_global(),
    "tap"                   [Pure]   => tap_global(),
    "const"                 [Pure]   => const_global(),
    "compose"               [Pure]   => compose_global(),
    "mutual"                [Pure]   => mutual_global(),

    // --- Debug ---
    "assert"                [Pure]   => assert_frost().build(),
    "debug_dump"            [Pure]   => debug_dump_frost().build(),

    // --- Mutable cell ---
    "mutable_cell"          [Impure] => mutable_cell_frost().build(),

    // --- Import ---
    "imported"              [Impure] => imported_frost().build(),
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
            debug_info: None,
        }),
    }))
}

/// A native named `name` that returns, as a value, the error a native with
/// `params` raises when its arguments have the wrong types. A hand-rolled
/// bytecode global calls it only once its own fast type check has failed, then
/// raises the result with `ProduceError`. Raised from the global's own frame,
/// the error names the global once in the backtrace; raised from this native,
/// it would name it twice, once for each.
fn type_error_native(name: &'static str, params: Params) -> Value {
    Value::native(name, params.arity(), move |ctx, args| {
        match ctx.check_args(args, &params) {
            Err(error) => Ok(error.into_value()),
            Ok(()) => unreachable!(
                "{} calls its type error native only when an argument has a wrong type",
                ctx.name()
            ),
        }
    })
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
