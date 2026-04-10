# Hash

```frost
def hash = import('ext.hash')
```

All hash functions take a single `String` argument and return a lowercase hexadecimal `String` digest.
Frost strings are binary-safe, so the input may contain arbitrary bytes.

To obtain the raw binary digest, use [`encoding.hex.decode`](encoding.md#hexdecode).

```frost
def enc = import('std.encoding')
def raw_digest = enc.hex.decode(hash.sha256('hello'))
```

The module also provides HMAC (Hash-based Message Authentication Code) variants of each algorithm via the `hmac` sub-map.
Each HMAC function takes a key and a message, both `String`, and returns a lowercase hexadecimal `String`.

```frost
hash.hmac.sha256('my-secret-key', 'message to authenticate')
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

## `crc32`
`crc32(input)`

Returns the CRC-32 checksum (32-bit) of `input` as an 8-character hexadecimal string.

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

## HMAC

Each hash algorithm has a corresponding HMAC function under `hmac`.

## `hmac.md5`
`hmac.md5(key, message)`

Returns the HMAC-MD5 of `message` using `key`.

## `hmac.sha1`
`hmac.sha1(key, message)`

Returns the HMAC-SHA-1 of `message` using `key`.

## `hmac.sha224`
`hmac.sha224(key, message)`

Returns the HMAC-SHA-224 of `message` using `key`.

## `hmac.sha256`
`hmac.sha256(key, message)`

Returns the HMAC-SHA-256 of `message` using `key`.

## `hmac.sha384`
`hmac.sha384(key, message)`

Returns the HMAC-SHA-384 of `message` using `key`.

## `hmac.sha512`
`hmac.sha512(key, message)`

Returns the HMAC-SHA-512 of `message` using `key`.

## `hmac.sha3_224`
`hmac.sha3_224(key, message)`

Returns the HMAC-SHA3-224 of `message` using `key`.

## `hmac.sha3_256`
`hmac.sha3_256(key, message)`

Returns the HMAC-SHA3-256 of `message` using `key`.

## `hmac.sha3_384`
`hmac.sha3_384(key, message)`

Returns the HMAC-SHA3-384 of `message` using `key`.

## `hmac.sha3_512`
`hmac.sha3_512(key, message)`

Returns the HMAC-SHA3-512 of `message` using `key`.

## `hmac.sha512_224`
`hmac.sha512_224(key, message)`

Returns the HMAC-SHA-512/224 of `message` using `key`.

## `hmac.sha512_256`
`hmac.sha512_256(key, message)`

Returns the HMAC-SHA-512/256 of `message` using `key`.

## `hmac.blake2s256`
`hmac.blake2s256(key, message)`

Returns the HMAC-BLAKE2s of `message` using `key`.

## `hmac.blake2b512`
`hmac.blake2b512(key, message)`

Returns the HMAC-BLAKE2b of `message` using `key`.

## `hmac.ripemd160`
`hmac.ripemd160(key, message)`

Returns the HMAC-RIPEMD-160 of `message` using `key`.

## `hmac.sm3`
`hmac.sm3(key, message)`

Returns the HMAC-SM3 of `message` using `key`.
