# Hash

```frost
def hash = import('ext.hash')
```

Cryptographic and non-cryptographic hash functions, plus HMAC variants.

All hash functions take a single `String` argument and return a lowercase hexadecimal `String` digest. Frost strings are binary-safe, so the input may contain arbitrary bytes.

Build flag: `WITH_HASH` (default: `AUTO`). Requires OpenSSL and zlib.

To obtain the raw binary digest, use [`encoding.hex.decode`](encoding.md#hexdecode).

```frost
def enc = import('std.encoding')
def raw_digest = enc.hex.decode(hash.sha256('hello'))
```

The module also provides HMAC (Hash-based Message Authentication Code) variants of each algorithm via the `hmac` sub-map. Each HMAC function takes a key and a message, both `String`, and returns a lowercase hexadecimal `String`.

```frost
hash.hmac.sha256('my-secret-key', 'message to authenticate')
```

Some algorithms in this module (notably MD5 and SHA-1) are considered weak and should not be used for security-sensitive purposes. They are included for interoperability with existing systems. When security matters, research current best practices.

## Hash Algorithms

Each of these also has an HMAC variant.

|  Function | Algorithm | Digest size  |
| ---|---|--- |
|  `md5` | MD5 | 128-bit  |
|  `sha1` | SHA-1 | 160-bit  |
|  `sha224` | SHA-224 | 224-bit  |
|  `sha256` | SHA-256 | 256-bit  |
|  `sha384` | SHA-384 | 384-bit  |
|  `sha512` | SHA-512 | 512-bit  |
|  `sha3_224` | SHA3-224 | 224-bit  |
|  `sha3_256` | SHA3-256 | 256-bit  |
|  `sha3_384` | SHA3-384 | 384-bit  |
|  `sha3_512` | SHA3-512 | 512-bit  |
|  `sha512_224` | SHA-512/224 | 224-bit  |
|  `sha512_256` | SHA-512/256 | 256-bit  |
|  `blake2s256` | BLAKE2s | 256-bit  |
|  `blake2b512` | BLAKE2b | 512-bit  |
|  `ripemd160` | RIPEMD-160 | 160-bit  |
|  `sm3` | SM3 | 256-bit  |

## Checksums and Fast Hashes

|  Function | Algorithm | Digest size  |
| ---|---|--- |
|  `crc32` | CRC-32 | 32-bit  |
|  `xxh32` | xxHash32 | 32-bit  |
|  `xxh64` | xxHash64 | 64-bit  |
|  `xxh3_64` | XXH3 (64-bit) | 64-bit  |
|  `xxh3_128` | XXH3 (128-bit) | 128-bit  |

## `crc32`

`hash.crc32(input)`

Returns the CRC-32 checksum (32-bit) of `input` as an 8-character hexadecimal string.

## `md5`

`hash.md5(input)`

Returns the MD5 digest (128-bit) of `input`.

## `sha1`

`hash.sha1(input)`

Returns the SHA-1 digest (160-bit) of `input`.

## `sha224`

`hash.sha224(input)`

Returns the SHA-224 digest (224-bit) of `input`.

## `sha256`

`hash.sha256(input)`

Returns the SHA-256 digest (256-bit) of `input`.

## `sha384`

`hash.sha384(input)`

Returns the SHA-384 digest (384-bit) of `input`.

## `sha512`

`hash.sha512(input)`

Returns the SHA-512 digest (512-bit) of `input`.

## `sha3_224`

`hash.sha3_224(input)`

Returns the SHA3-224 digest (224-bit) of `input`.

## `sha3_256`

`hash.sha3_256(input)`

Returns the SHA3-256 digest (256-bit) of `input`.

## `sha3_384`

`hash.sha3_384(input)`

Returns the SHA3-384 digest (384-bit) of `input`.

## `sha3_512`

`hash.sha3_512(input)`

Returns the SHA3-512 digest (512-bit) of `input`.

## `sha512_224`

`hash.sha512_224(input)`

Returns the SHA-512/224 digest (224-bit) of `input`.

## `sha512_256`

`hash.sha512_256(input)`

Returns the SHA-512/256 digest (256-bit) of `input`.

## `blake2s256`

`hash.blake2s256(input)`

Returns the BLAKE2s digest (256-bit) of `input`.

## `blake2b512`

`hash.blake2b512(input)`

Returns the BLAKE2b digest (512-bit) of `input`.

## `ripemd160`

`hash.ripemd160(input)`

Returns the RIPEMD-160 digest (160-bit) of `input`.

## `sm3`

`hash.sm3(input)`

Returns the SM3 digest (256-bit) of `input`.

## `xxh32`

`hash.xxh32(input)`

Returns the xxHash32 digest (32-bit) of `input` as an 8-character hexadecimal string. xxHash is a non-cryptographic hash optimized for speed.

## `xxh64`

`hash.xxh64(input)`

Returns the xxHash64 digest (64-bit) of `input`.

## `xxh3_64`

`hash.xxh3_64(input)`

Returns the XXH3 digest (64-bit) of `input`. XXH3 is the latest xxHash algorithm and is faster than xxh32/xxh64 on most inputs.

## `xxh3_128`

`hash.xxh3_128(input)`

Returns the XXH3 digest (128-bit) of `input`.

## HMAC

Each hash algorithm has a corresponding HMAC function under `hmac`.

### `hmac.md5`

`hash.hmac.md5(key, message)`

Returns the HMAC-MD5 of `message` using `key`.

### `hmac.sha1`

`hash.hmac.sha1(key, message)`

Returns the HMAC-SHA-1 of `message` using `key`.

### `hmac.sha224`

`hash.hmac.sha224(key, message)`

Returns the HMAC-SHA-224 of `message` using `key`.

### `hmac.sha256`

`hash.hmac.sha256(key, message)`

Returns the HMAC-SHA-256 of `message` using `key`.

### `hmac.sha384`

`hash.hmac.sha384(key, message)`

Returns the HMAC-SHA-384 of `message` using `key`.

### `hmac.sha512`

`hash.hmac.sha512(key, message)`

Returns the HMAC-SHA-512 of `message` using `key`.

### `hmac.sha3_224`

`hash.hmac.sha3_224(key, message)`

Returns the HMAC-SHA3-224 of `message` using `key`.

### `hmac.sha3_256`

`hash.hmac.sha3_256(key, message)`

Returns the HMAC-SHA3-256 of `message` using `key`.

### `hmac.sha3_384`

`hash.hmac.sha3_384(key, message)`

Returns the HMAC-SHA3-384 of `message` using `key`.

### `hmac.sha3_512`

`hash.hmac.sha3_512(key, message)`

Returns the HMAC-SHA3-512 of `message` using `key`.

### `hmac.sha512_224`

`hash.hmac.sha512_224(key, message)`

Returns the HMAC-SHA-512/224 of `message` using `key`.

### `hmac.sha512_256`

`hash.hmac.sha512_256(key, message)`

Returns the HMAC-SHA-512/256 of `message` using `key`.

### `hmac.blake2s256`

`hash.hmac.blake2s256(key, message)`

Returns the HMAC-BLAKE2s of `message` using `key`.

### `hmac.blake2b512`

`hash.hmac.blake2b512(key, message)`

Returns the HMAC-BLAKE2b of `message` using `key`.

### `hmac.ripemd160`

`hash.hmac.ripemd160(key, message)`

Returns the HMAC-RIPEMD-160 of `message` using `key`.

### `hmac.sm3`

`hash.hmac.sm3(key, message)`

Returns the HMAC-SM3 of `message` using `key`.

