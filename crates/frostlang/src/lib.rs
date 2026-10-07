//! Frost: an embeddable, dynamically-typed functional programming language.
//!
//! This crate gathers what a host application needs to run Frost: everything in
//! [`frostlang_runtime`] and, with the `compile` feature, [`frostlang_compile`].
//!
//! # Features
//!
//! - `compile` (default): compiling Frost source. Without it, a host runs only
//!   bytecode compiled elsewhere; see [`CompiledFunction::assert_trusted`].
//! - `graphical-diagnostics`: draw compile errors as source snippets with their
//!   labels, rather than narrating them as plain text. If you show Frost
//!   compiler errors to people, you probably want it. Implies `compile`.
//!
//! A library that extends Frost, such as one providing native functions, should
//! depend on [`frostlang_runtime`] instead. Its users then choose for themselves
//! whether to include the compiler.

#[cfg(feature = "compile")]
pub use frostlang_compile::*;
pub use frostlang_runtime::*;
