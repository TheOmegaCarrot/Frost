use std::env;
use std::path::PathBuf;

// Compiles the generated tree-sitter Frost grammar (a single parser.c, no
// external scanner) so astviz can syntax-highlight the source pane. The grammar
// source lives in the repo-root `editor/` directory.
fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let grammar_src = manifest.join("../../editor/tree-sitter-frost/src");
    let parser_c = grammar_src.join("parser.c");

    cc::Build::new()
        .include(&grammar_src)
        .file(&parser_c)
        .flag_if_supported("-w")
        .compile("tree_sitter_frost");

    println!("cargo:rerun-if-changed={}", parser_c.display());
}
