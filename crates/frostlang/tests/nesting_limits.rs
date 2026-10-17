//! The default [`ParseLimits`] keep compiling within the stack: source nested as deeply as
//! they accept, in every shape, compiles and is freed on a thread with Rust's default stack
//! size, under every optimization permutation. So does each shape nested as deeply as
//! accepted around the longest `@` chain accepted within it: the costliest mix, since the
//! recursion limit bounds how many costly levels there are, and the AST depth limit lets
//! a chain fill the rest.
//!
//! A stack overflow aborts the whole test binary, so a failure here shows as a crash
//! rather than a failed assertion; the last case printed is the one before the crash.

use std::thread;

use frostlang::compile::{CompilerOptions, OptimizationOptions, ParseLimits, compile_program};

use crate::script::optimization_permutations;

/// Rust's default stack size for a spawned thread.
const DEFAULT_STACK: usize = 2 * 1024 * 1024;

/// The bindings the shapes use, ahead of each.
const PRELUDE: &str = r"
defn f(x) -> x
defn g() -> g
def c = false
def m = {}
def xs = []
";

/// An expression nested as many levels as it is given, with the expression it is given
/// innermost.
type Shape = fn(usize, &str) -> String;

/// Each nesting shape.
const SHAPES: [(&str, Shape); 27] = [
    ("parentheses", |n, inner| nest("(", inner, ")", n)),
    ("Array literal", |n, inner| nest("[", inner, "]", n)),
    ("Map literal", |n, inner| nest("{a: ", inner, "}", n)),
    ("call argument", |n, inner| nest("f(", inner, ")", n)),
    ("prefix `-`", |n, inner| format!("{}{inner}", "-".repeat(n))),
    ("prefix `not`", |n, inner| {
        format!("{}{inner}", "not ".repeat(n))
    }),
    ("right operand", |n, inner| nest("1 + (", inner, ")", n)),
    ("`+` chain", |n, inner| {
        format!("{inner}{}", " + 1".repeat(n))
    }),
    ("`and` chain", |n, inner| {
        format!("{inner}{}", " and true".repeat(n))
    }),
    ("`.` chain", |n, inner| format!("{inner}{}", ".a".repeat(n))),
    ("index chain", |n, inner| {
        format!("{inner}{}", "[0]".repeat(n))
    }),
    ("call chain", |n, inner| {
        format!("{inner}{}", "()".repeat(n))
    }),
    ("`@` chain", |n, inner| {
        format!("{inner}{}", " @ f()".repeat(n))
    }),
    ("`elif`", |n, inner| {
        format!("if c: 0 {}else: {inner}", "elif c: 0 ".repeat(n))
    }),
    ("`if` branch", |n, inner| {
        format!("{}{inner}", "if c: ".repeat(n))
    }),
    ("`else` branch", |n, inner| {
        format!("{}{inner}", "if c: 0 else: ".repeat(n))
    }),
    ("lambda body", |n, inner| {
        format!("{}{inner}", "fn -> ".repeat(n))
    }),
    ("called lambda", |n, inner| nest("(fn -> ", inner, ")()", n)),
    ("`do` block", |n, inner| nest("do { ", inner, " }", n)),
    ("`match` arm", |n, inner| {
        nest("match 1 { _ => ", inner, " }", n)
    }),
    ("Array pattern", |n, inner| {
        format!(
            "match xs {{ {} => {inner}, _ => 1 }}",
            nest("[", "a", "]", n)
        )
    }),
    ("Map pattern", |n, inner| {
        format!(
            "match m {{ {} => {inner}, _ => 1 }}",
            nest("{a: ", "b", "}", n)
        )
    }),
    ("alternatives", |n, inner| {
        format!(
            "match xs {{ {} => {inner}, _ => 1 }}",
            nest("[a | ", "a", "]", n)
        )
    }),
    ("`def` pattern", |n, inner| {
        format!("do {{\ndef {} = xs\n{inner}\n}}", nest("[", "a", "]", n))
    }),
    ("format String", |n, inner| nest("$'${", inner, "}'", n)),
    // Indexed, so that each level formats a String rather than an Array, whose quotes
    // escaped at every level would make the result exponentially long.
    ("interpolated Array", |n, inner| {
        nest("$'${[", inner, "][0]}'", n)
    }),
    ("abbreviated lambda", |n, inner| {
        format!("$({})", nest("[$1, ", inner, "]", n))
    }),
];

fn nest(open: &str, inner: &str, close: &str, levels: usize) -> String {
    format!("{}{inner}{}", open.repeat(levels), close.repeat(levels))
}

/// An `@` chain of `links` links, the costliest chain to compile.
fn chain(links: usize) -> String {
    format!("1{}", " @ f()".repeat(links))
}

/// Whether `expression` compiles after the prelude under `optimization` and the default
/// limits, on a thread with Rust's default stack size. Whether it compiles or not,
/// everything is freed there.
fn compiles_on_a_default_stack(expression: String, optimization: OptimizationOptions) -> bool {
    thread::Builder::new()
        .stack_size(DEFAULT_STACK)
        .spawn(move || {
            let options = CompilerOptions::new().with_optimization(optimization);
            let source = format!("{PRELUDE}{expression}");
            compile_program("deep.frst", &source, options).is_ok()
        })
        .expect("the thread starts")
        .join()
        .expect("compiling does not panic")
}

/// The greatest `n` for which `expression(n)` compiles, given that `expression(0)` does.
fn greatest_accepted(name: &str, expression: impl Fn(usize) -> String) -> usize {
    let compiles = |n| compiles_on_a_default_stack(expression(n), OptimizationOptions::NONE);
    assert!(compiles(0), "{name}: the least compiles");
    // Past the AST depth limit, any shape is rejected.
    let mut rejected = ParseLimits::new()
        .max_ast_depth
        .expect("the default limits AST depth")
        .get() as usize
        + 1;
    assert!(!compiles(rejected), "{name}: rejected past the limit");
    let mut accepted = 0;
    while rejected - accepted > 1 {
        let middle = (accepted + rejected) / 2;
        if compiles(middle) {
            accepted = middle;
        } else {
            rejected = middle;
        }
    }
    accepted
}

/// Asserts that `expression` compiles under every optimization permutation.
fn assert_compiles_under_every_permutation(name: &str, expression: &str) {
    for optimization in optimization_permutations() {
        assert!(
            compiles_on_a_default_stack(expression.to_owned(), optimization),
            "{name} compiles under {optimization:?}"
        );
    }
}

#[test]
fn the_deepest_source_the_default_limits_accept_compiles_on_a_default_stack() {
    for (name, shape) in SHAPES {
        let deepest = greatest_accepted(name, |n| shape(n, "1"));
        println!("{name}: {deepest} levels");
        assert_compiles_under_every_permutation(name, &shape(deepest, "1"));

        // A level fewer, which leaves the recursion an `@` callee takes.
        let levels = deepest - 1;
        let links = greatest_accepted(name, |links| shape(levels, &chain(links)));
        println!("{name}: {levels} levels around {links} links");
        assert_compiles_under_every_permutation(name, &shape(levels, &chain(links)));
    }
}
