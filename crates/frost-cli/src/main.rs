use std::env;
use std::fs;
use std::process;

use frost_parse::parse_program;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("usage: frost <file.frst>");
        process::exit(1);
    }

    let filename = &args[1];
    let source = match fs::read_to_string(filename) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading {filename}: {e}");
            process::exit(1);
        }
    };

    match parse_program(filename, &source) {
        Ok(program) => {
            let json = serde_json::to_string_pretty(&program).unwrap();
            println!("{json}");
        }
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    }
}
