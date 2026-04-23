# OS

```frost
def os = import('std.os')
```

## `getenv`
`getenv(name)`

Returns the value of the environment variable `name` as a `String`, or `null`
if it is not set.

## `setenv`
`setenv(variable, value)`

Sets the environment variable `variable` to `value`, overwriting any existing value.
Returns `null`.
Produces an error if the operating system rejects the variable name (the exact conditions are platform-dependent).

## `unsetenv`
`unsetenv(variable)`

Removes the environment variable `variable`.
Unsetting a variable that is not set is not an error.
Returns `null`.
Produces an error if the operating system rejects the variable name (the exact conditions are platform-dependent).

## `pid`
`pid()`

Returns the process ID of the current process as an `Int`.

## `hostname`
`hostname()`

Returns the hostname of the current machine as a `String`.

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

`command` is the executable to run. It can be an absolute path or a program name, which is looked up in `PATH`.
`args` is an Array of Strings passed as command-line arguments.
Each element is passed as a single argument with no word splitting -- `os.run('foo', ['bar baz'])` is equivalent to `foo 'bar baz'`, not `foo bar baz`.

```frost
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

```frost
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

### Notes

`run` always waits for the child process to exit before returning.
Both stdout and stderr are captured in full; this is not a streaming interface.

If the child process is killed by a signal (e.g., SIGSEGV), `exit_code` will be non-zero.
Any output the child flushed before the signal is still captured.

If the command cannot be found or fails to launch, `run` produces an error.
