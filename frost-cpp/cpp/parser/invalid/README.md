# Invalid Frost Programs

This directory contains a growing corpus of *invalid* Frost programs used to
track parser error-message quality over time.

## Layout

- `cases/` contains one invalid Frost program per file.
- `run.sh` runs the parser over all cases and records the output.

## Usage

From the repo root:

```sh
./cpp/parser/invalid/run.sh [path/to/cases]
```

If no argument is provided, it defaults to `cases/` next to the script.

This writes one output file per case into `cpp/parser/invalid/outputs/`.
Re-run after grammar changes to compare error messages.
