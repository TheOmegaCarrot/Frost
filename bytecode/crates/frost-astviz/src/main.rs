//! astviz -- interactive AST visualizer for Frost (Rust parser).
//!
//! Parses a `.frst` file and emits a self-contained HTML file: source on the
//! left, AST tree on the right, with bidirectional highlighting and source
//! range validation. Developer tooling, not part of the distribution.

mod highlight;
mod walk;

use std::path::Path;
use std::process::ExitCode;

use serde::Serialize;
use walk::{Highlight, LineIndex, Node, Walker};

#[derive(Serialize)]
struct FrostData {
    filename: String,
    source: String,
    ast: Vec<Node>,
    highlights: Vec<Highlight>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: frost-astviz <file.frst>");
        return ExitCode::FAILURE;
    }
    let path = &args[1];

    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: could not read {path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let program = match frost_parse::parse_program(path, &source) {
        Ok(program) => program,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let index = LineIndex::new(&source);
    let mut walker = Walker::new(&index);
    let ast: Vec<Node> = program.statements.iter().map(|s| walker.stmt(s)).collect();

    let highlights = highlight::highlights(&source);

    let filename = Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.clone());

    let data = FrostData {
        filename: filename.clone(),
        source,
        ast,
        highlights,
    };

    // Escape `</` as `<\/` so embedded JSON can't close the <script> early.
    let json = serde_json::to_string(&data)
        .expect("AST serialization is infallible")
        .replace("</", "<\\/");

    let template = include_str!("../template.html");

    print!(
        "<!DOCTYPE html>\n\
         <html lang=\"en\">\n\
         <head>\n\
        \x20 <meta charset=\"utf-8\">\n\
        \x20 <title>AST: {filename}</title>\n\
         </head>\n\
         <body>\n\
         <script>\nwindow.FROST_DATA = {json};\n</script>\n\
         {template}\n\
         </body>\n\
         </html>\n"
    );

    ExitCode::SUCCESS
}
