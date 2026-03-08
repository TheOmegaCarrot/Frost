# Filesystem

## `fs.absolute`
`fs.absolute(path)`

Returns the absolute form of `path` without resolving symlinks.

## `fs.canonical`
`fs.canonical(path)`

Returns the canonical absolute path, resolving symlinks and `.`/`..` components.
Produces an error if `path` does not exist.

## `fs.exists`
`fs.exists(path)`

Returns `true` if `path` exists on the filesystem.

## `fs.stat`
`fs.stat(path)`

Returns a map describing the file at `path`:

```
{
    type:  String,   # "regular", "directory", "symlink", "block",
                     # "character", "fifo", "socket", "none",
                     # "not_found", or "unknown"
    perms: {
        owner:  { read: Bool, write: Bool, exec: Bool },
        group:  { read: Bool, write: Bool, exec: Bool },
        others: { read: Bool, write: Bool, exec: Bool },
    }
}
```

Produces an error if the stat call fails.

## `fs.size`
`fs.size(path)`

Returns the size of the file at `path` in bytes as an `Int`.
Produces an error if `path` does not exist or is not a regular file.

## `fs.cwd`
`fs.cwd()`

Returns the current working directory as a string.

## `fs.cd`
`fs.cd(path)`

Changes the current working directory to `path`.
Produces an error if `path` does not exist or is not accessible.

## `fs.list`
`fs.list(path)`

Returns an array of path strings for all entries in the directory at `path`.
Produces an error if `path` does not exist or is not a directory.

## `fs.list_recursively`
`fs.list_recursively(path)`

Recursively lists all entries under `path`.
Entries in directories where permission is denied are silently skipped.
Produces an error if `path` itself does not exist or is not a directory.

## `fs.mkdir`
`fs.mkdir(path)`

Creates `path` and any missing parent directories.
Returns `true` if the directory was created, `false` if it already existed.

## `fs.move`
`fs.move(src, dest)`

Moves or renames `src` to `dest`. Produces an error on failure.

## `fs.copy`
`fs.copy(src, dest)`

Recursively copies `src` to `dest`. Produces an error on failure.

## `fs.symlink`
`fs.symlink(to, link)`

Creates a symbolic link at `link` pointing to `to`. Produces an error on failure.

## `fs.remove`
`fs.remove(path)`

Removes the file or empty directory at `path`.
Returns `true` if something was removed, `false` if `path` did not exist.

## `fs.remove_recursively`
`fs.remove_recursively(path)`

Recursively removes `path` and all its contents.
Returns the number of entries removed as an `Int`.
Returns `0` if `path` did not exist.

## `fs.concat`
`fs.concat(base, path)`

Joins `base` and `path` using the platform path separator.
Equivalent to `base / path` in filesystem terms.

## `fs.stem`
`fs.stem(path)`

Returns the filename component of `path` with the last extension removed.

```
fs.stem('/a/b/file.txt')    # => 'file'
fs.stem('archive.tar.gz')   # => 'archive.tar'
fs.stem('/a/b/file')        # => 'file'
```

## `fs.extension`
`fs.extension(path)`

Returns the last extension of the filename component of `path`, including the leading dot.
Returns an empty string if there is no extension.

```
fs.extension('/a/b/file.txt')   # => '.txt'
fs.extension('archive.tar.gz')  # => '.gz'
fs.extension('/a/b/file')       # => ''
```

## `fs.filename`
`fs.filename(path)`

Returns the final component of `path` (the filename), including any extension.

```
fs.filename('/a/b/file.txt')  # => 'file.txt'
```

## `fs.parent`
`fs.parent(path)`

Returns the parent directory of `path`.
Returns an empty string if `path` contains no directory component.

```
fs.parent('/a/b/file.txt')  # => '/a/b'
fs.parent('/a/b')           # => '/a'
fs.parent('file.txt')       # => ''
```
