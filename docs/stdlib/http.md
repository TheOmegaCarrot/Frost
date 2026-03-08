# HTTP

HTTP support is optional, and must be enabled at build time with `-DWITH_HTTP=YES`.
This is enabled by default.

## `http.request`
`http.request(config)`

Performs an asynchronous HTTP request.
Returns immediately with a handle map; use `.is_ready()` to poll and `.get()` to retrieve the result when ready.

**`config` fields:**

| Field           | Type   | Required | Default | Description |
|-----------------|--------|----------|---------|-------------|
| `uri`           | Map    | Yes      | —       | See URI fields below |
| `method`        | String | No       | `"GET"` | HTTP method (`"GET"`, `"POST"`, etc.) |
| `headers`       | Map    | No       | `{}`    | Request headers. Values may be `String` or `Array` of `String` for repeated headers. `host`, `content-length`, and `transfer-encoding` are managed automatically. |
| `body`          | String | No       | none    | Request body |
| `timeout_ms`    | Int    | No       | `10000` | Request timeout in milliseconds |
| `verify_tls`    | Bool   | No       | `true`  | Whether to verify TLS certificates |
| `ca_file`       | String | No       | —       | Path to a CA certificate file |
| `ca_path`       | String | No       | —       | Path to a directory of CA certificates |
| `use_system_ca` | Bool   | No       | `true`  | Whether to load the system CA store |

**`uri` fields:**

| Field   | Type   | Required | Default   | Description |
|---------|--------|----------|-----------|-------------|
| `host`  | String | Yes      | —         | Hostname or IP address |
| `path`  | String | No       | `"/"`     | Request path |
| `tls`   | Bool   | No       | `true`    | Whether to use HTTPS |
| `port`  | Int    | No       | 80 or 443 | Port number (80 if `tls` is `false`, 443 otherwise) |
| `query` | Map    | No       | `{}`      | Query parameters. Values may be `String`, `null` (value-less param), or `Array` of `String` for repeated params. |

**Return value:**

```
{
    is_ready: fn -> Bool,
    get:      fn -> result
}
```

`.get()` blocks until the request either completes or errors, then returns.
The result is cached internally; subsequent calls to `.get()` return the same value immediately.

`ok` reflects network-level success — whether a response was received — not the HTTP status code.
A server returning a 500 yields `ok: true`.
Check `response.code` to determine application-level success.

```
# On network success (any HTTP status code):
{
    ok:       true,
    response: {
        code:    Int,
        body:    String,
        headers: Map   # header names are lowercased; values are String
                       # or Array of String for repeated headers
    }
}

# On network failure (connection error, timeout, etc.):
{
    ok:    false,
    error: { category: String, phase: String, message: String }
}
```
