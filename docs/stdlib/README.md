# Standard Library

Frost's standard library consists of builtin functions (implemented in C++)
and prelude functions (implemented in Frost, loaded at startup).

Some builtins are namespaced (`re`, `fs`, `http`) and accessed via dot syntax.

An [alphabetical index of all functions](./index.md) is available.

## Parameter Name Conventions

Many functions in the standard library are sufficiently generic that parameter names beyond the type carry little additional meaning.
In these cases, short placeholder names are used in signatures rather than inventing names that would imply unwarranted specificity.

| Name       | Meaning                        |
|------------|--------------------------------|
| `n`        | Any numeric value              |
| `s`        | A `String`                     |
| `arr`      | An `Array`                     |
| `m`        | A `Map`                        |
| `f`        | A `Function`                   |
| `value`    | Any value                      |
| `path`     | A filesystem path (`String`)   |
| `pattern`  | A regex pattern (`String`)     |

## Modules

| Module                              | Description                                                                                |
|-------------------------------------|--------------------------------------------------------------------------------------------|
| [Collections](./collections.md)     | `keys`, `values`, `len`, `has`, `range`, `take`, `drop`, `zip`, `sorted`, `group_by`, ... |
| [Types](./types.md)                 | `is_int`, `is_string`, `type`, `to_string`, `read_value`, `to_int`, `to_float`, `pretty`  |
| [Math](./math.md)                   | `abs`, `sqrt`, `pow`, `min`, `max`, trig, ...                                              |
| [Strings](./strings.md)             | `split`, `to_upper`, `contains`, `starts_with`, base64, ...                                |
| [Regex](./regex.md)                 | `re.matches`, `re.replace`, `re.scan_matches`                                              |
| [Operators](./operators.md)         | `plus`, `minus`, `equal`, `deep_equal`, ...                                                |
| [Output](./output.md)               | `print`, `mformat`, `mprint`                                                               |
| [Streams](./streams.md)             | `open_read`, `open_trunc`, `stdin`, `stderr`                                                |
| [Functions](./functions.md)         | `pack_call`, `try_call`, `and_then`, `or_else`                                              |
| [Debug](./debug.md)                 | `assert`, `debug_dump`                                                                      |
| [Mutable Cell](./mutable-cell.md)   | `mutable_cell`                                                                              |
| [OS](./os.md)                       | `getenv`, `exit`, `sleep`                                                                   |
| [Filesystem](./filesystem.md)       | `fs.move`, `fs.exists`, `fs.list`, ...                                                      |
| [JSON](./json.md)                   | `parse_json`, `to_json`                                                                     |
| [HTTP](./http.md)                   | `http.request` (optional)                                                                   |
| [Prelude](./prelude.md)             | `curry`, `compose`, `tap`, `reject`, ...                                                    |
