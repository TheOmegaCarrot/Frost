# Hash

```frost
def hash = import('std.hash')
```

All hash functions take a single `String` argument and return a lowercase hexadecimal `String` digest.
Frost strings are binary-safe, so the input may contain arbitrary bytes.

To obtain the raw binary digest, use [`encoding.hex.decode`](encoding.md#hexdecode).

```frost
def enc = import('std.encoding')
def raw_digest = enc.hex.decode(hash.sha256('hello'))
```

## Available algorithms

| Function | Algorithm | Digest size |
|---|---|---|
| `md5` | MD5 | 128-bit |
| `sha1` | SHA-1 | 160-bit |
| `sha224` | SHA-224 | 224-bit |
| `sha256` | SHA-256 | 256-bit |
| `sha384` | SHA-384 | 384-bit |
| `sha512` | SHA-512 | 512-bit |
| `sha3_224` | SHA3-224 | 224-bit |
| `sha3_256` | SHA3-256 | 256-bit |
| `sha3_384` | SHA3-384 | 384-bit |
| `sha3_512` | SHA3-512 | 512-bit |
| `sha512_224` | SHA-512/224 | 224-bit |
| `sha512_256` | SHA-512/256 | 256-bit |
| `blake2s256` | BLAKE2s | 256-bit |
| `blake2b512` | BLAKE2b | 512-bit |
| `ripemd160` | RIPEMD-160 | 160-bit |
| `sm3` | SM3 | 256-bit |

## `md5`
`md5(input)`

Returns the MD5 digest (128-bit) of `input`.

## `sha1`
`sha1(input)`

Returns the SHA-1 digest (160-bit) of `input`.

## `sha224`
`sha224(input)`

Returns the SHA-224 digest (224-bit) of `input`.

## `sha256`
`sha256(input)`

Returns the SHA-256 digest (256-bit) of `input`.

## `sha384`
`sha384(input)`

Returns the SHA-384 digest (384-bit) of `input`.

## `sha512`
`sha512(input)`

Returns the SHA-512 digest (512-bit) of `input`.

## `sha3_224`
`sha3_224(input)`

Returns the SHA3-224 digest (224-bit) of `input`.

## `sha3_256`
`sha3_256(input)`

Returns the SHA3-256 digest (256-bit) of `input`.

## `sha3_384`
`sha3_384(input)`

Returns the SHA3-384 digest (384-bit) of `input`.

## `sha3_512`
`sha3_512(input)`

Returns the SHA3-512 digest (512-bit) of `input`.

## `sha512_224`
`sha512_224(input)`

Returns the SHA-512/224 digest (224-bit) of `input`.

## `sha512_256`
`sha512_256(input)`

Returns the SHA-512/256 digest (256-bit) of `input`.

## `blake2s256`
`blake2s256(input)`

Returns the BLAKE2s digest (256-bit) of `input`.

## `blake2b512`
`blake2b512(input)`

Returns the BLAKE2b digest (512-bit) of `input`.

## `ripemd160`
`ripemd160(input)`

Returns the RIPEMD-160 digest (160-bit) of `input`.

## `sm3`
`sm3(input)`

Returns the SM3 digest (256-bit) of `input`.
