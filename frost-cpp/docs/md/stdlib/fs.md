# Filesystem

```frost
def fs = import('std.fs')
```

Filesystem operations: path manipulation, file metadata, directory listing, and file management.

## `absolute`

`fs.absolute(path)`

Returns the absolute form of `path` without resolving symlinks.

## `canonical`

`fs.canonical(path)`

Returns the canonical absolute path, resolving symlinks and `.`/`..` components. Produces an error if `path` does not exist.

## `exists`

`fs.exists(path)`

Returns `true` if `path` exists on the filesystem.

## `is_file`

`fs.is_file(path)`

Returns `true` if `path` exists and is a regular file.

## `is_directory`

`fs.is_directory(path)`

Returns `true` if `path` exists and is a directory.

## `is_symlink`

`fs.is_symlink(path)`

Returns `true` if `path` exists and is a symbolic link.

## `is_block`

`fs.is_block(path)`

Returns `true` if `path` exists and is a block device.

## `is_character`

`fs.is_character(path)`

Returns `true` if `path` exists and is a character device.

## `is_fifo`

`fs.is_fifo(path)`

Returns `true` if `path` exists and is a named pipe (FIFO).

## `is_socket`

`fs.is_socket(path)`

Returns `true` if `path` exists and is a Unix domain socket. All `is_*` predicates return `false` for nonexistent paths.

## `stat`

`fs.stat(path)`

Returns a map describing the file at `path`. Produces an error if the stat call fails.

```frost
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

## `size`

`fs.size(path)`

Returns the size of the file at `path` in bytes as an `Int`. Produces an error if `path` does not exist or is not a regular file.

## `cwd`

`fs.cwd()`

Returns the current working directory as a string.

## `cd`

`fs.cd(path)`

Changes the current working directory to `path`. Produces an error if `path` does not exist or is not accessible.

## `list`

`fs.list(path)`

Returns an array of path strings for all entries in the directory at `path`. Produces an error if `path` does not exist or is not a directory.

## `list_recursively`

`fs.list_recursively(path)`

Recursively lists all entries under `path`. Entries in directories where permission is denied are silently skipped. Produces an error if `path` itself does not exist or is not a directory.

## `mkdir`

`fs.mkdir(path)`

Creates `path` and any missing parent directories. Returns `true` if the directory was created, `false` if it already existed.

## `move`

`fs.move(src, dest)`

Moves or renames `src` to `dest`. Produces an error on failure.

## `copy`

`fs.copy(src, dest)`

Recursively copies `src` to `dest`. Produces an error on failure.

## `symlink`

`fs.symlink(to, link)`

Creates a symbolic link at `link` pointing to `to`. Produces an error on failure.

## `remove`

`fs.remove(path)`

Removes the file or empty directory at `path`. Returns `true` if something was removed, `false` if `path` did not exist.

## `remove_recursively`

`fs.remove_recursively(path)`

Recursively removes `path` and all its contents. Returns the number of entries removed as an `Int`. Returns `0` if `path` did not exist.

## `concat`

`fs.concat(base, path)`

Joins `base` and `path` using the platform path separator. Equivalent to `base / path` in filesystem terms.

## `stem`

`fs.stem(path)`

Returns the filename component of `path` with the last extension removed.

```frost
fs.stem('/a/b/file.txt')
# => file
```

```frost
fs.stem('archive.tar.gz')
# => archive.tar
```

```frost
fs.stem('/a/b/file')
# => file
```

## `extension`

`fs.extension(path)`

Returns the last extension of the filename component of `path`, including the leading dot. Returns an empty string if there is no extension.

```frost
fs.extension('/a/b/file.txt')
# => .txt
```

```frost
fs.extension('archive.tar.gz')
# => .gz
```

```frost
fs.extension('/a/b/file')
# => 
```

## `filename`

`fs.filename(path)`

Returns the final component of `path` (the filename), including any extension.

```frost
fs.filename('/a/b/file.txt')
# => file.txt
```

## `parent`

`fs.parent(path)`

Returns the parent directory of `path`. Returns an empty string if `path` contains no directory component.

```frost
fs.parent('/a/b/file.txt')
# => /a/b
```

```frost
fs.parent('/a/b')
# => /a
```

```frost
fs.parent('file.txt')
# => 
```

