# Compression

```frost
def { deflate, gzip, zlib } = import('ext.compression')
```

Build flag: `WITH_COMPRESSION` (default: `ON`). Requires zlib.

Each algorithm provides `compress` and `decompress` functions operating on binary strings.
Since Frost strings are binary-safe, compressed output can be stored, concatenated, or passed to other functions without issue.

Algorithms can be swapped at runtime, as all algorithms support the same compress/decompress interface (if default compression parameters are accepted):

```frost
def algo = if fast: deflate else: gzip
algo.compress(data)
```

## `brotli.compress`
`brotli.compress(s)`
`brotli.compress(s, quality)`

Compresses `s` using Brotli (RFC 7932). Used by HTTP `Content-Encoding: br`.
`quality` is an optional `Int` from `0` to `11`. Default is `11` (maximum compression).

## `brotli.decompress`
`brotli.decompress(s)`

Decompresses a Brotli string. Produces an error on corrupt or truncated input.

## `deflate.compress`
`deflate.compress(s)`
`deflate.compress(s, level)`

Compresses `s` using raw DEFLATE (RFC 1951). No header or checksum.
`level` is an optional `Int`: `-1` (default, equivalent to `6`), `0` (no compression), or `1`--`9` (increasing effort).

## `deflate.decompress`
`deflate.decompress(s)`

Decompresses a raw DEFLATE string. Produces an error on corrupt or truncated input.

## `gzip.compress`
`gzip.compress(s)`
`gzip.compress(s, level)`

Compresses `s` in gzip format (RFC 1952). Includes a gzip header and CRC-32 checksum.
Used by HTTP `Content-Encoding: gzip`.
`level` is an optional `Int`: `-1` (default, equivalent to `6`), `0` (no compression), or `1`--`9` (increasing effort).

## `gzip.decompress`
`gzip.decompress(s)`

Decompresses a gzip string. Produces an error on corrupt or truncated input.

## `zlib.compress`
`zlib.compress(s)`
`zlib.compress(s, level)`

Compresses `s` in zlib-wrapped format (RFC 1950). Includes a 2-byte header and Adler-32 checksum.
`level` is an optional `Int`: `-1` (default, equivalent to `6`), `0` (no compression), or `1`--`9` (increasing effort).

## `zlib.decompress`
`zlib.decompress(s)`

Decompresses a zlib-wrapped string. Produces an error on corrupt or truncated input.
