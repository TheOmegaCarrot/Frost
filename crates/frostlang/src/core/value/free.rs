//! Freeing Values within a bounded native stack, however deeply they nest.
//!
//! Left to the compiler's drop glue, freeing a container frees each of its
//! children, which frees theirs in turn: one native stack frame per level, so a
//! deeply nested Value overflows the stack when it is freed. Each type whose
//! Values hold other Values frees them through [`free`] in its `Drop`, which lets
//! the glue recurse only so deep, then frees the rest in a loop.
//!
//! Every Value is still freed exactly once, by ownership: a Value is only ever
//! moved or dropped here, never forgotten.

use std::cell::Cell;
use std::sync::Arc;

use crate::core::Value;

/// How many containers deep the drop glue may recurse on a thread before [`free`]
/// frees what is below in a loop. The glue is faster, so the many shallow trees
/// a script makes are freed by it alone.
const MAX_RECURSION: usize = 32;

thread_local! {
    /// How many calls to [`free`] this thread is within that let the drop glue recurse.
    static DEPTH: Cell<usize> = const { Cell::new(0) };
}

/// Frees `contents`, the Values a container held, and everything they solely own.
/// `into_values` gives the Values in `contents`, for freeing them in a loop.
pub(crate) fn free<C, I>(contents: C, into_values: impl FnOnce(C) -> I)
where
    I: Iterator<Item = Value>,
{
    // Where the thread local is unavailable, the loop, which needs none, frees instead.
    let recursing = DEPTH.try_with(|depth| {
        let within = depth.get() < MAX_RECURSION;
        if within {
            depth.set(depth.get() + 1);
        }
        within
    });
    if recursing == Ok(true) {
        let _recursing = Recursing;
        drop(contents);
    } else {
        free_iteratively(into_values(contents));
    }
}

/// A level of [`DEPTH`], given back when dropped, even by a panic unwinding.
struct Recursing;

impl Drop for Recursing {
    fn drop(&mut self) {
        let _ = DEPTH.try_with(|depth| depth.set(depth.get() - 1));
    }
}

fn free_iteratively(contents: impl Iterator<Item = Value>) {
    let mut pending = Vec::new();
    for value in contents {
        take_apart(value, &mut pending);
        while let Some(value) = pending.pop() {
            take_apart(value, &mut pending);
        }
    }
}

/// Drops `value`, first moving its children onto `pending` if it was the last
/// owner of a container.
fn take_apart(value: Value, pending: &mut Vec<Value>) {
    // Whichever owner releases a container last frees it, whether here or through
    // the container's own `Drop`. `Arc::into_inner` claims and releases in one step;
    // a failed `Arc::try_unwrap` would hand back an `Arc` that other threads could
    // leave this the last owner of, to free through `Drop`.
    // A container claimed here is emptied before it drops, so its `Drop` frees nothing.
    match value {
        Value::Array(array) => {
            if let Some(mut elements) = Arc::into_inner(array.inner) {
                pending.append(&mut elements);
            }
        }
        Value::Map(map) => {
            if let Some(map) = Arc::into_inner(map.inner) {
                pending.extend(map.into_values());
            }
        }
        Value::Closure(closure) => {
            if let Some(mut closure) = Arc::into_inner(closure) {
                pending.append(&mut closure.captures);
            }
        }
        // Holds no Values, or holds them only inside native or Opaque state, which
        // this cannot see into.
        // TODO: A chain nesting Values through such state, alternating with
        // containers, still recurses once per link. Nothing a script can reach
        // builds one; only a host's own natives or Opaques can.
        Value::Null
        | Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::String(_)
        | Value::Bytes(_)
        | Value::NativeFunction(_)
        | Value::Opaque(_) => {}
    }
}
