# CLI

```frost
def cli = import('std.cli')
```

`std.cli` parses command-line arguments against a declarative spec. Describe your tool's flags, options, and positional arguments in a Map; `cli.parse` validates `args` against that spec and returns a structured result.

## `parse`

`cli.parse(args, spec)`

Parses `args` according to `spec`. Returns a result Map with `ok` (`Bool`), `help` (`String`), and either `value` (on success) or `error` (on failure). The `value` map contains `flags` (`Map` of `Bool`), `options` (`Map` of `String`, `Array`, or `null`), and `positional` (`Array` of `String`). Spec errors (malformed spec) produces an error; parse errors (invalid user input) are returned in the result.

`args` is typically the predefined [`args`](../language.md#the-args-variable) variable. `args[0]` is treated as the script path and is consumed internally (used as the default tool name in error messages and help output). Everything after `args[0]` is parsed against the spec.

```frost
def cli = import('std.cli')

def result = cli.parse(args, {
    name: 'deploy',
    description: 'Deploy a service to a target environment',
    flags: {
        verbose: { short: 'v', description: 'Enable verbose output' },
        dry_run: { short: 'd', description: 'Show what would happen' },
    },
    options: {
        env: { short: 'e', required: true, description: 'Target environment' },
        retries: { short: 'r', default: '3', description: 'Number of retries' },
        tag: { short: 't', repeatable: true, description: 'Image tags' },
    },
    positional: [
        { name: 'service', description: 'Service to deploy' },
    ],
})

if not result.ok: do {
    print(result.error)
    print(result.help)
    os.exit(1)
}

def parsed = result.value
if parsed.flags.verbose: print('verbose mode')
def retries = parsed.options.retries @ to_int() or 3
def service = parsed.positional[0]
```

### Spec Shape

The spec is a Map with the following keys. All are optional.

|  Key | Type | Description  |
| ---|---|--- |
|  `name` | `String` | Tool name for help output and error messages. Defaults to `args[0]`.  |
|  `description` | `String` | A short description shown in the auto-generated help.  |
|  `help` | `String` | Custom help text. When present, this replaces the auto-generated help entirely.  |
|  `flags` | `Map` | Boolean toggles. See Flags below.  |
|  `options` | `Map` | Value-taking options. See Options below.  |
|  `positional` | `Array` or `Bool` | Positional arguments. See Positional Arguments below.  |

### Flags

Flags are boolean toggles. The `flags` key is a Map where each key is the flag's long name and each value is a sub-spec Map:

|  Sub-spec key | Type | Description  |
| ---|---|--- |
|  `short` | `String` | Single-character short form (used with `-`).  |
|  `description` | `String` | Shown in the auto-generated help.  |

In the result, every declared flag appears under `result.flags` as a `Bool`: `true` if it was present in `args`, `false` otherwise.

```frost
flags: {
    verbose: { short: 'v', description: 'Verbose output' },
    ['dry-run']: { description: 'Dry-run mode' },
}
```

### Options

Options take a value from the following argument. The `options` key is a Map where each key is the option's long name and each value is a sub-spec Map:

|  Sub-spec key | Type | Description  |
| ---|---|--- |
|  `short` | `String` | Single-character short form.  |
|  `required` | `Bool` | When true, the option must be provided.  |
|  `default` | `String` | Default value when not provided. Mutually exclusive with `required`.  |
|  `repeatable` | `Bool` | When true, the option can be specified multiple times.  |
|  `description` | `String` | Shown in the auto-generated help.  |

In the result, every declared option appears under `result.options`: non-repeatable options are `String` if provided or defaulted, `null` otherwise. Repeatable options are always an `Array` of `String`, possibly empty.

Option values are always Strings. `cli.parse` does not perform any type coercion; for other types, apply a conversion like [`to_int`](types.md#to_int) on the parsed value.

```frost
options: {
    env:     { short: 'e', required: true },
    retries: { short: 'r', default: '3' },
    tag:     { short: 't', repeatable: true },
}
```

### Positional Arguments

The `positional` key has three forms:

- Absent, `[]`, or `false`: no positional arguments are allowed. Any positional produces an error.
- Array of specs: each entry declares a named positional. All are required. Exact count must match.
- `true`: raw-collect mode. All positional arguments are gathered into `result.positional` without validation.

Each positional spec entry is a Map with `name` (`String`, required, used in help output) and `description` (`String`, shown in help).

```frost
positional: [
    { name: 'source', description: 'Source file' },
    { name: 'dest', description: 'Destination file' },
]
```

Or for scripts that want to handle positionals themselves:

```frost
positional: true
```

Array destructuring pairs nicely with declared positionals when you want to bind them by name:

```frost
def [source, dest] = parsed.positional
```

### Parsing Rules

- `--name` for flags, `--name value` for options (long form).
- `-n` for flags, `-n value` for options (short form).
- `-abc` is equivalent to `-a -b -c` (short bundling). The last character in a bundle may be a value-taking option that consumes the next argument. A value-taking option that is not the last character in a bundle is an error.
- `--` terminates option parsing. Everything after a bare `--` is treated as positional.
- Bare `-` is treated as a positional argument (Unix convention for stdin/stdout).
- Specifying a non-repeatable option more than once is an error.
- Required options must be provided or the parse fails.
- Defaults are applied to non-repeatable, non-required options that were not provided.

### Help Text

`result.help` always contains a help string. If the spec provides a `help` key, that string is returned verbatim. Otherwise, help is auto-generated from `name`, `description`, and the declared flags, options, and positionals.

`cli.parse` does not handle `--help` or `-h` automatically. The typical pattern is to add a `help` flag to the spec and check it after unwrapping:

```frost
def result = cli.parse(args, {
    flags: {
        help: { short: 'h', description: 'Show help' },
    },
})

if not result.ok: do {
    print(result.error)
    print(result.help)
    os.exit(1)
}

if result.value.flags.help: do {
    print(result.help)
    os.exit(0)
}
```

Note that `result.help` (the auto-generated text) and `result.value.flags.help` (the user-declared flag) are distinct.

### Errors

`cli.parse` distinguishes two error categories:

- Spec errors are prefixed with `cli.parse:` and indicate a malformed spec. These produce an error (catchable via `try_call`) because they are bugs in the script, not the user's invocation.
- Parse errors are prefixed with the tool name and indicate a problem with the user's invocation (unknown flag, missing required option, etc.). These are returned in the result as `{ ok: false, error: "..." }` so the script can show help or take other action.

```text
# Spec error
cli.parse: option 'env': required and default are mutually exclusive

# Parse error
deploy: missing required option '--env'
```

## `prompt`

`cli.prompt(message)`

Prints `message` followed by a space to stderr (no newline) and reads a line from stdin. Returns the line as a `String`, or `null` if the user aborts with end-of-file. Only the trailing newline is stripped; any other whitespace the user typed is preserved.

```frost
def name = cli.prompt('What is your name?')
if name != null: print($'Hello, ${name}')
```

stderr is used so that the prompt is visible even when stdout is redirected (`frost script.frst > out.txt`).

### Building a Yes/No Confirmation

`std.cli` does not provide a dedicated confirmation helper because the right behavior on end-of-file or unrecognized input depends on the script. A few lines using `prompt` cover most needs:

```frost
def ans = cli.prompt('Delete all files? [y/N]')
def confirmed = ans and ( ans @ to_lower() @ trim() @ starts_with('y') )
if confirmed: do {
    # ...
}
```

This pattern treats end-of-file as "no" (the safe default), which is usually what you want for destructive actions. Invert the logic for prompts where the safe default is "yes".

