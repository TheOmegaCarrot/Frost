//! The integration tests, built as one binary: each file here is a module.

// Shared harness.
mod helpers;

mod ast_equality;
mod destructure;
mod diagnostic_labels;
mod expressions_bytes;
mod expressions_control_flow;
mod expressions_format_strings;
mod expressions_iterative;
mod expressions_lambda;
mod expressions_literals;
mod expressions_match;
mod expressions_operators;
mod expressions_postfix;
mod expressions_strings;
mod expressions_structures;
mod habit_hints;
mod line_endings;
mod literal_errors;
mod parse_errors;
mod render_styles;
mod tokens;
