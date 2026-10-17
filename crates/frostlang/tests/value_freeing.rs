//! Freeing a Value, however deeply it nests, never overflows the native stack, and frees
//! each Value it solely owns exactly once, even as threads release a shared tree at once.
//!
//! Each deep case is freed on a thread with a small stack, which freeing one native frame
//! per level would overflow many times over. A stack overflow aborts the whole test
//! binary, so a failure here shows as a crash rather than a failed assertion.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;

use frostlang::{FrostOpaque, MapKey, OpaqueHandle, Value, ValueMap};

use crate::script::{self, Script};

/// Far less stack than freeing one frame per level of [`DEPTH`] would take.
const SMALL_STACK: usize = 512 * 1024;

/// How deeply the deep cases nest.
const DEPTH: usize = 100_000;

/// Runs `f` to completion on a thread with [`SMALL_STACK`].
fn on_a_small_stack<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    thread::scope(|scope| {
        thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn_scoped(scope, f)
            .expect("the thread spawns")
            .join()
            .expect("the thread completes")
    })
}

/// `inner` nested `depth` levels deep, each level built by `wrap` around the one below.
fn nested(depth: usize, inner: Value, wrap: impl Fn(Value) -> Value) -> Value {
    (0..depth).fold(inner, |below, _| wrap(below))
}

fn in_an_array(below: Value) -> Value {
    Value::from(vec![below])
}

fn in_a_map(below: Value) -> Value {
    Value::from(ValueMap::from([(MapKey::from("below"), below)]))
}

// -- Nesting shapes --

#[test]
fn a_deeply_nested_array_is_freed() {
    on_a_small_stack(|| drop(nested(DEPTH, Value::Null, in_an_array)));
}

#[test]
fn a_deeply_nested_map_is_freed() {
    on_a_small_stack(|| drop(nested(DEPTH, Value::Null, in_a_map)));
}

#[test]
fn arrays_and_maps_nested_in_turn_are_freed() {
    on_a_small_stack(|| {
        drop(nested(DEPTH / 2, Value::Null, |below| {
            in_a_map(in_an_array(below))
        }));
    });
}

#[test]
fn a_value_map_holding_a_deep_value_is_freed() {
    on_a_small_stack(|| {
        let deep = nested(DEPTH, Value::Null, in_a_map);
        drop(ValueMap::from([(MapKey::from("deep"), deep)]));
    });
}

#[test]
fn an_array_of_deep_values_is_freed() {
    on_a_small_stack(|| {
        let branches = (0..10)
            .map(|_| nested(DEPTH / 10, Value::Null, in_an_array))
            .collect::<Vec<_>>();
        drop(Value::from(branches));
    });
}

/// A thread's locals are freed as the thread exits, after the rest of its work.
#[test]
fn a_deep_value_in_a_thread_local_is_freed_as_the_thread_exits() {
    thread_local! {
        static HELD: std::cell::RefCell<Value> = const { std::cell::RefCell::new(Value::Null) };
    }
    let freed = Arc::new(AtomicUsize::new(0));
    on_a_small_stack(|| {
        let outermost = counted_levels(DEPTH, &freed)
            .pop()
            .expect("there are levels");
        HELD.with(|held| *held.borrow_mut() = outermost);
    });
    assert_eq!(
        freed.load(Ordering::Relaxed),
        DEPTH,
        "each level is freed once"
    );
}

// -- Built by scripts --

/// A script nesting `v` in [`DEPTH`] levels, each built by `level` around the `v` below,
/// runs under every optimization permutation and frees what it builds.
fn assert_script_frees(level: &str) {
    let source = format!(
        r"
defn nest(v, n) -> if n == 0: v else: nest({level}, n - 1)
def deep = nest(null, {DEPTH})
is_null(deep)
"
    );
    let tail = on_a_small_stack(|| Script::new(&source).run());
    assert_eq!(tail, Value::Bool(false), "{source}");
}

#[test]
fn a_script_frees_deeply_nested_arrays() {
    assert_script_frees("[v]");
}

#[test]
fn a_script_frees_deeply_nested_maps() {
    assert_script_frees("{v}");
}

#[test]
fn a_script_frees_closures_nested_by_capture() {
    assert_script_frees("fn -> v");
}

#[test]
fn a_script_frees_every_shape_nested_in_turn() {
    assert_script_frees("(if n % 3 == 0: [v] elif n % 3 == 1: {v} else: fn -> v)");
}

#[test]
fn a_script_frees_deep_values_it_joins() {
    let source = format!(
        r"
defn nest(v, n) -> if n == 0: v else: nest([v], n - 1)
def deep = nest(null, {DEPTH})
def joined = [deep] + [1]
def merged = {{deep}} + {{other: 1}}
len(joined) + len(merged)
"
    );
    let tail = on_a_small_stack(|| Script::new(&source).run());
    assert_eq!(tail, Value::Int(4), "{source}");
}

/// A chain of closures, each capturing the one below, frees what its innermost capture
/// holds once the chain is freed, under every optimization permutation.
#[test]
fn closures_nested_by_capture_free_their_innermost_capture() {
    let source = format!(
        r"
defn nest(v, n) -> if n == 0: v else: nest(fn -> v, n - 1)
nest(innermost, {DEPTH})
"
    );
    on_a_small_stack(|| {
        for optimization in script::optimization_permutations() {
            let freed = Arc::new(AtomicUsize::new(0));
            let script = Script::new(&source).capture("innermost", counted(&freed));
            let chain = script.run_under(optimization);
            drop(script);
            assert_eq!(
                freed.load(Ordering::Relaxed),
                0,
                "{optimization:?}: the chain holds its innermost capture"
            );
            drop(chain);
            assert_eq!(
                freed.load(Ordering::Relaxed),
                1,
                "{optimization:?}: freeing the chain frees its innermost capture once"
            );
        }
    });
}

/// Continuation-passing recursion nests each continuation in the capture of the next.
#[test]
fn continuation_passing_recursion_runs() {
    let source = r"
defn sum(n, k) -> if n == 0: k(0) else: sum(n - 1, fn r -> k(r + n))
sum(100000, fn r -> r)
";
    let tail = on_a_small_stack(|| Script::new(source).run());
    assert_eq!(tail, Value::Int(5_000_050_000), "{source}");
}

// -- Each Value freed exactly once --

/// A payload that counts its own freeing.
#[derive(Debug)]
struct Counted(Arc<AtomicUsize>);

impl Drop for Counted {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

impl FrostOpaque for Counted {
    fn type_name() -> &'static str {
        "Counted"
    }

    fn try_to_string(&self) -> Option<String> {
        None
    }
}

fn counted(freed: &Arc<AtomicUsize>) -> Value {
    Value::Opaque(OpaqueHandle::new(Counted(freed.clone())))
}

/// Every level of a chain `depth` deep, innermost first, each holding a [`Counted`]
/// adding to `freed` and the level below. The levels are an Array, a small Map, and a
/// Map large enough to take its other form, in turn.
fn counted_levels(depth: usize, freed: &Arc<AtomicUsize>) -> Vec<Value> {
    let mut levels: Vec<Value> = Vec::with_capacity(depth);
    for level in 0..depth {
        let counted = counted(freed);
        let below = levels.last().cloned().unwrap_or(Value::Null);
        levels.push(match level % 3 {
            0 => Value::from(vec![counted, below]),
            shape => {
                let mut map = ValueMap::from([
                    (MapKey::from("counted"), counted),
                    (MapKey::from("below"), below),
                ]);
                if shape == 2 {
                    map.extend((0..20).map(|key| (MapKey::from(key), Value::Int(key))));
                }
                Value::from(map)
            }
        });
    }
    levels
}

/// Each of the depths around where freeing stops recursing and starts looping, and one
/// far past it, is freed exactly once.
#[test]
fn a_tree_of_any_depth_frees_each_value_in_it_once() {
    on_a_small_stack(|| {
        for depth in [1, 2, 31, 32, 33, 34, 63, 64, 65, 96, 97, DEPTH] {
            let freed = Arc::new(AtomicUsize::new(0));
            let outermost = counted_levels(depth, &freed)
                .pop()
                .expect("there are levels");
            assert_eq!(
                freed.load(Ordering::Relaxed),
                0,
                "{depth} levels deep: nothing is freed yet"
            );
            drop(outermost);
            assert_eq!(
                freed.load(Ordering::Relaxed),
                depth,
                "{depth} levels deep: each level is freed once"
            );
        }
    });
}

/// Every level holds the one below twice, so a tree 100 levels deep reaches its
/// innermost level by 2^100 paths. Freeing visits each shared level once.
#[test]
fn a_tree_sharing_its_levels_frees_each_once() {
    on_a_small_stack(|| {
        let freed = Arc::new(AtomicUsize::new(0));
        let outermost = (0..100).fold(Value::Null, |below, _| {
            Value::from(vec![counted(&freed), below.clone(), below])
        });
        drop(outermost);
        assert_eq!(
            freed.load(Ordering::Relaxed),
            100,
            "each level is freed once"
        );
    });
}

#[test]
fn a_deep_value_held_many_times_is_freed_once() {
    on_a_small_stack(|| {
        let freed = Arc::new(AtomicUsize::new(0));
        let deep = counted_levels(DEPTH, &freed)
            .pop()
            .expect("there are levels");
        let many = Value::from(vec![deep; 1000]);
        drop(many);
        assert_eq!(
            freed.load(Ordering::Relaxed),
            DEPTH,
            "each level is freed once"
        );
    });
}

/// Values taken out of a Map, and those left in the iterator that took them, are each
/// freed once.
#[test]
fn a_map_partly_iterated_frees_each_value_in_it_once() {
    on_a_small_stack(|| {
        let freed = Arc::new(AtomicUsize::new(0));
        let map: ValueMap = (0..20)
            .map(|key| {
                let deep = counted_levels(DEPTH / 20, &freed)
                    .pop()
                    .expect("there are levels");
                (MapKey::from(key), deep)
            })
            .collect();
        let mut entries = map.into_iter();
        let taken: Vec<_> = entries.by_ref().take(3).collect();
        drop(entries);
        drop(taken);
        assert_eq!(
            freed.load(Ordering::Relaxed),
            DEPTH,
            "each level is freed once"
        );
    });
}

/// Elements taken out of an Array, whether it is shared or not, are freed once, with
/// whatever else holds them.
#[test]
fn elements_taken_out_of_an_array_are_freed_once() {
    on_a_small_stack(|| {
        let freed = Arc::new(AtomicUsize::new(0));
        let deep = counted_levels(DEPTH, &freed)
            .pop()
            .expect("there are levels");
        let array = Value::from(vec![deep]);
        let shared = array.as_array().cloned().expect("an Array");

        let copied = shared.into_vec();
        let shared = array.as_array().cloned().expect("an Array");
        let shared = shared
            .try_into_vec()
            .expect_err("a shared Array's elements stay in it");
        drop(array);
        let taken = shared
            .try_into_vec()
            .expect("the last owner takes the elements out");
        drop(taken);
        assert_eq!(
            freed.load(Ordering::Relaxed),
            0,
            "the copy still holds them all"
        );

        drop(copied);
        assert_eq!(
            freed.load(Ordering::Relaxed),
            DEPTH,
            "each level is freed once"
        );
    });
}

/// A payload that panics as it is freed.
#[derive(Debug)]
struct Panics;

impl Drop for Panics {
    fn drop(&mut self) {
        panic!("freeing a payload that panics");
    }
}

impl FrostOpaque for Panics {
    fn type_name() -> &'static str {
        "Panics"
    }

    fn try_to_string(&self) -> Option<String> {
        None
    }
}

/// A panic freeing one Value, whether near the top of a tree or deep within it, still
/// frees every other Value in the tree once, and leaves the thread freeing as before.
#[test]
fn a_panic_while_freeing_frees_the_rest_once() {
    on_a_small_stack(|| {
        for panics_at in [5, DEPTH / 2] {
            let freed = Arc::new(AtomicUsize::new(0));
            let below = counted_levels(DEPTH - panics_at, &freed)
                .pop()
                .expect("there are levels");
            let panicking = Value::from(vec![Value::Opaque(OpaqueHandle::new(Panics)), below]);
            let outermost = (1..panics_at).fold(panicking, |below, _| {
                Value::from(vec![counted(&freed), below])
            });

            let unwound =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(outermost)));
            assert!(unwound.is_err(), "the payload panics");
            assert_eq!(
                freed.load(Ordering::Relaxed),
                DEPTH - 1,
                "panicking {panics_at} levels deep: each other level is freed once"
            );
        }
        drop(nested(DEPTH, Value::Null, in_an_array));
    });
}

#[test]
fn a_level_held_elsewhere_outlives_the_tree_around_it() {
    on_a_small_stack(|| {
        let freed = Arc::new(AtomicUsize::new(0));
        let mut levels = counted_levels(DEPTH, &freed);
        let outermost = levels.pop().expect("there are levels");
        let held = levels.swap_remove(DEPTH / 2);
        drop(levels);
        assert_eq!(
            freed.load(Ordering::Relaxed),
            0,
            "the outermost level holds them all"
        );

        drop(outermost);
        assert_eq!(
            freed.load(Ordering::Relaxed),
            DEPTH - DEPTH / 2 - 1,
            "only the levels above the held one are freed"
        );

        drop(held);
        assert_eq!(
            freed.load(Ordering::Relaxed),
            DEPTH,
            "the rest go with the held level"
        );
    });
}

/// Threads releasing every level of the same tree at once, some outermost first and some
/// innermost first, contend over who frees each level. Whatever the interleaving, each is
/// freed exactly once.
#[test]
fn threads_releasing_a_shared_tree_free_each_value_once() {
    const THREADS: usize = 8;
    const LEVELS: usize = 10_000;
    const ROUNDS: usize = 20;

    for round in 0..ROUNDS {
        let freed = Arc::new(AtomicUsize::new(0));
        let levels = counted_levels(LEVELS, &freed);
        let barrier = Barrier::new(THREADS);
        thread::scope(|scope| {
            for thread in 0..THREADS {
                let mut held = levels.clone();
                if thread % 2 == 1 {
                    held.reverse();
                }
                let barrier = &barrier;
                thread::Builder::new()
                    .stack_size(SMALL_STACK)
                    .spawn_scoped(scope, move || {
                        barrier.wait();
                        drop(held);
                    })
                    .expect("the thread spawns");
            }
            drop(levels);
        });
        assert_eq!(
            freed.load(Ordering::Relaxed),
            LEVELS,
            "round {round}: each level is freed exactly once"
        );
    }
}
