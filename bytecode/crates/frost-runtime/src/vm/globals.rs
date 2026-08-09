//! Predefined globals available in every Frost program without importing.
//!
//! The `define_globals!` invocation below is the single source of truth for the
//! global names, their slot order (used by `LoadGlobal`), and their constructors.
//! Each global is built by a `*_global()` constructor that lives in a topic
//! submodule; not-yet-implemented ones return [`stub`].

mod collections;
mod debug;
mod functions;
mod import;
mod mutable_cell;
mod operators;
mod output;
mod strings;
mod types;

use std::sync::{Arc, LazyLock};

use crate::core::FrostResult;
use crate::{Arity, Value};

use collections::*;
use debug::*;
use functions::*;
use import::*;
use mutable_cell::*;
use operators::*;
use output::*;
use strings::*;
use types::*;

// The fixed set of predefined globals, shared by every `Vm`; never host-configurable.
// The compiler's seam onto it is `GLOBAL_NAMES`.
#[derive(Debug, Clone)]
pub(crate) struct GlobalSet(Vec<Value>);

// Sync-only macro: names and slot initializers expand from the same `name => init`
// list in the same order, so `names[i]` and `slots[i]` cannot drift apart.
macro_rules! define_globals {
    ($($name:literal => $init:expr),* $(,)?) => {
        /// The predefined global names, in slot order:
        /// a name's index in this list is its `LoadGlobal` slot.
        /// This list is the compiler's only seam onto the globals,
        /// which are themselves a fixed, runtime-internal set.
        pub const GLOBAL_NAMES: &[&str] = &[ $($name),* ];

        impl GlobalSet {
            fn build_defaults() -> Self {
                GlobalSet ( vec![ $($init),* ] )
            }
        }
    };
}

define_globals! {
    // --- Types ---
    "is_null"               => is_null_global(),
    "is_int"                => is_int_global(),
    "is_float"              => is_float_global(),
    "is_bool"               => is_bool_global(),
    "is_string"             => is_string_global(),
    "is_bytes"              => is_bytes_global(),
    "is_array"              => is_array_global(),
    "is_map"                => is_map_global(),
    "is_function"           => is_function_global(),
    "is_nonnull"            => is_nonnull_global(),
    "is_numeric"            => is_numeric_global(),
    "is_primitive"          => is_primitive_global(),
    "is_structured"         => is_structured_global(),
    "is_flat"               => is_flat_global(),
    "type"                  => type_global(),
    "to_string"             => to_string_global(),
    "pretty"                => pretty_global(),
    "to_int"                => to_int_global(),
    "to_float"              => to_float_global(),
    "to_bytes"              => to_bytes_global(),
    "from_utf8"             => from_utf8_global(),

    // --- Strings ---
    "split"                 => split_global(),
    "lines"                 => lines_global(),
    "join"                  => join_global(),
    "replace"               => replace_global(),
    "trim"                  => trim_global(),
    "trim_left"             => trim_left_global(),
    "trim_right"            => trim_right_global(),
    "to_upper"              => to_upper_global(),
    "to_lower"              => to_lower_global(),
    "contains"              => contains_global(),
    "starts_with"           => starts_with_global(),
    "ends_with"             => ends_with_global(),

    // --- Operators ---
    "plus"                  => plus_global(),
    "minus"                 => minus_global(),
    "times"                 => times_global(),
    "divide"                => divide_global(),
    "mod"                   => mod_global(),
    "equal"                 => equal_global(),
    "not_equal"             => not_equal_global(),
    "less_than"             => less_than_global(),
    "less_than_or_equal"    => less_than_or_equal_global(),
    "greater_than"          => greater_than_global(),
    "greater_than_or_equal" => greater_than_or_equal_global(),

    // --- Collections ---
    "keys"                  => keys_global(),
    "values"                => values_global(),
    "map_keys"              => map_keys_global(),
    "map_values"            => map_values_global(),
    "len"                   => len_global(),
    "range"                 => range_global(),
    "nulls"                 => nulls_global(),
    "repeat"                => repeat_global(),
    "id"                    => id_global(),
    "has"                   => has_global(),
    "includes"              => includes_global(),
    "index"                 => index_global(),
    "dig"                   => dig_global(),
    "slice"                 => slice_global(),
    "stride"                => stride_global(),
    "take"                  => take_global(),
    "drop"                  => drop_global(),
    "tail"                  => tail_global(),
    "drop_tail"             => drop_tail_global(),
    "slide"                 => slide_global(),
    "chunk"                 => chunk_global(),
    "reverse"               => reverse_global(),
    "take_while"            => take_while_global(),
    "drop_while"            => drop_while_global(),
    "chunk_by"              => chunk_by_global(),
    "flatten"               => flatten_global(),
    "zip"                   => zip_global(),
    "zip_with"              => zip_with_global(),
    "xprod"                 => xprod_global(),
    "xprod_with"            => xprod_with_global(),
    "transform"             => transform_global(),
    "flat_map"              => flat_map_global(),
    "select"                => select_global(),
    "reject"                => reject_global(),
    "fold"                  => fold_global(),
    "sum"                   => sum_global(),
    "product"               => product_global(),
    "sorted"                => sorted_global(),
    "sort_by"               => sort_by_global(),
    "any"                   => any_global(),
    "all"                   => all_global(),
    "none"                  => none_global(),
    "find"                  => find_global(),
    "group_by"              => group_by_global(),
    "count_by"              => count_by_global(),
    "scan"                  => scan_global(),
    "partition"             => partition_global(),
    "map_into"              => map_into_global(),
    "to_entries"            => to_entries_global(),
    "from_entries"          => from_entries_global(),
    "dissoc"                => dissoc_global(),
    "each"                  => each_global(),

    // --- Output ---
    "print"                 => print_global(),
    "mformat"               => mformat_global(),
    "mprint"                => mprint_global(),

    // --- Functions / combinators ---
    "call"                  => call_global(),
    "try_call"              => try_call_global(),
    "error"                 => error_global(),
    "and_then"              => and_then_global(),
    "or_else"               => or_else_global(),
    "inv"                   => inv_global(),
    "curry"                 => curry_global(),
    "bcurry"                => bcurry_global(),
    "collect"               => collect_global(),
    "spread"                => spread_global(),
    "rev_args"              => rev_args_global(),
    "tap"                   => tap_global(),
    "const"                 => const_global(),
    "compose"               => compose_global(),

    // --- Debug ---
    "assert"                => assert_global(),
    "debug_dump"            => debug_dump_global(),

    // --- Mutable cell ---
    "mutable_cell"          => mutable_cell_global(),

    // --- Import ---
    "import"                => import_global(),
}

/// A not-yet-implemented global. The table still builds (so `GlobalSet::defaults()`
/// and every `Vm` keep working, and the test suite stays green), but invoking the
/// global panics with a clear message. Replace the matching `*_global()` body with
/// the real constructor when implementing it.
fn stub(name: &'static str) -> Value {
    Value::native(
        name,
        Arity::AtLeast(0),
        move |_ctx, _args: &mut [Value]| -> FrostResult {
            todo!("the `{name}` global is not yet implemented")
        },
    )
}

static DEFAULT_GLOBALS: LazyLock<Arc<GlobalSet>> =
    LazyLock::new(|| Arc::new(GlobalSet::build_defaults()));

impl GlobalSet {
    pub fn defaults() -> Arc<Self> {
        DEFAULT_GLOBALS.clone()
    }

    /// Get the value at a global slot index.
    pub fn get(&self, idx: usize) -> &Value {
        &self.0[idx]
    }
}
