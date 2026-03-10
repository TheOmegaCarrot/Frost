# OS

## `args`

A predefined `Array` of `String` values containing the command-line arguments passed to the program.
When running a file, `args[0]` is the script path and subsequent elements are any additional arguments.

```
frost script.frst foo bar   # args == ["script.frst", "foo", "bar"]
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
