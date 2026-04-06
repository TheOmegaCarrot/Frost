# OS

```frost
def os = import('std.os')
```

## `getenv`
`getenv(name)`

Returns the value of the environment variable `name` as a `String`, or `null`
if it is not set.

## `exit`
`exit(code)`

Terminates the process with the given integer exit code. Does not return.

## `sleep`
`sleep(ms)`

Pauses execution for `ms` milliseconds.

## `run`
`run(command, args)`
`run(command, args, options)`

Runs an executable and waits for it to complete.
Returns a Map with three keys:

- `stdout` (`String`) -- the captured standard output
- `stderr` (`String`) -- the captured standard error
- `exit_code` (`Int`) -- the process exit code

`command` is the path to the executable.
`args` is an Array of Strings passed as command-line arguments.

```
def r = os.run('/bin/echo', ['hello', 'world'])
# r.stdout == "hello world\n"
# r.exit_code == 0
```

### Options

The optional third argument is a Map that may contain any of the following keys:

| Key | Type | Description |
|-----|------|-------------|
| `stdin` | `String` | Data piped to the process's standard input |
| `cwd` | `String` | Working directory for the process |
| `env` | `Map` | Environment variables to add to or override in the inherited environment |
| `replace_env` | `Map` | Environment variables that completely replace the inherited environment |

`env` and `replace_env` are mutually exclusive.
Both expect a Map with String keys and String values.

```
# Pipe data to stdin
def r = os.run('/usr/bin/cat', [], {stdin: 'hello'})
# r.stdout == "hello"

# Set working directory
def r = os.run('/bin/pwd', [], {cwd: '/tmp'})
# r.stdout == "/tmp\n"

# Inherit environment and add a variable
def r = os.run('/usr/bin/env', [], {env: {MY_VAR: 'value'}})

# Replace environment entirely
def r = os.run('/usr/bin/env', [], {replace_env: {PATH: '/usr/bin'}})
```
