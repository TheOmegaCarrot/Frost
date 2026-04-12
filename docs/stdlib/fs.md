# Filesystem

```frost
def fs = import('std.fs')
```

## `absolute`
`absolute(path)`

Returns the absolute form of `path` without resolving symlinks.

## `canonical`
`canonical(path)`

Returns the canonical absolute path, resolving symlinks and `.`/`..` components.
Produces an error if `path` does not exist.

## `exists`
`exists(path)`

Returns `true` if `path` exists on the filesystem.

## `stat`
`stat(path)`

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

## `size`
`size(path)`

Returns the size of the file at `path` in bytes as an `Int`.
Produces an error if `path` does not exist or is not a regular file.

## `cwd`
`cwd()`

Returns the current working directory as a string.

## `cd`
`cd(path)`

Changes the current working directory to `path`.
Produces an error if `path` does not exist or is not accessible.

## `list`
`list(path)`

Returns an array of path strings for all entries in the directory at `path`.
Produces an error if `path` does not exist or is not a directory.

## `list_recursively`
`list_recursively(path)`

Recursively lists all entries under `path`.
Entries in directories where permission is denied are silently skipped.
Produces an error if `path` itself does not exist or is not a directory.

## `mkdir`
`mkdir(path)`

Creates `path` and any missing parent directories.
Returns `true` if the directory was created, `false` if it already existed.

## `move`
`move(src, dest)`

Moves or renames `src` to `dest`. Produces an error on failure.

## `copy`
`copy(src, dest)`

Recursively copies `src` to `dest`. Produces an error on failure.

## `symlink`
`symlink(to, link)`

Creates a symbolic link at `link` pointing to `to`. Produces an error on failure.

## `remove`
`remove(path)`

Removes the file or empty directory at `path`.
Returns `true` if something was removed, `false` if `path` did not exist.

## `remove_recursively`
`remove_recursively(path)`

Recursively removes `path` and all its contents.
Returns the number of entries removed as an `Int`.
Returns `0` if `path` did not exist.

## `concat`
`concat(base, path)`

Joins `base` and `path` using the platform path separator.
Equivalent to `base / path` in filesystem terms.

## `stem`
`stem(path)`

Returns the filename component of `path` with the last extension removed.

```frost
fs.stem('/a/b/file.txt')    # => 'file'
fs.stem('archive.tar.gz')   # => 'archive.tar'
fs.stem('/a/b/file')        # => 'file'
```

## `extension`
`extension(path)`

Returns the last extension of the filename component of `path`, including the leading dot.
Returns an empty string if there is no extension.

```frost
fs.extension('/a/b/file.txt')   # => '.txt'
fs.extension('archive.tar.gz')  # => '.gz'
fs.extension('/a/b/file')       # => ''
```

## `filename`
`filename(path)`

Returns the final component of `path` (the filename), including any extension.

```frost
fs.filename('/a/b/file.txt')  # => 'file.txt'
```

## `parent`
`parent(path)`

Returns the parent directory of `path`.
Returns an empty string if `path` contains no directory component.

```frost
fs.parent('/a/b/file.txt')  # => '/a/b'
fs.parent('/a/b')           # => '/a'
fs.parent('file.txt')       # => ''
```
