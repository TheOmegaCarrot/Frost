# Datetime

```frost
def dt = import('std.datetime')
```

Timestamps, formatting, parsing, and calendar components.

All timestamps are `Int` values representing Unix milliseconds (ms since 1970-01-01T00:00:00 UTC).
Integer arithmetic can be used for duration math with the help of the `ms` constants.

```frost
def tomorrow = dt.now() + dt.ms.day
def in_3_hours = dt.now() + 3 * dt.ms.hour
```

All functions operate in UTC. Timezone support is planned for a future version.

Format patterns follow the [`std::chrono::format`](https://en.cppreference.com/w/cpp/chrono/format) specification.

## `now`

`now()`

Returns the current time as Unix milliseconds.

## `format`

`format(millis, pattern)`

Formats a timestamp as a `String` using a chrono format pattern.

```frost
dt.format(dt.now(), '%Y-%m-%d %H:%M:%S')
# => "2026-04-19 14:30:00.000"
```

## `parse`

`parse(s, pattern)`

Parses a date/time string according to a pattern and returns Unix milliseconds.
Throws on invalid input or pattern mismatch.
Fields not specified by the pattern default to their epoch values (1970-01-01 00:00:00 UTC).

```frost
dt.parse('2024-03-16', '%Y-%m-%d')
dt.parse('2024-03-16 14:30:00', '%Y-%m-%d %H:%M:%S')
```

## `components`

`components(millis)`

Breaks a timestamp into its UTC calendar components.
Returns a `Map` with keys `year`, `month` (1-12), `day` (1-31), `hour`, `minute`, `second`, `ms`, and `weekday` (a `String` such as `"Monday"`).

```frost
dt.components(dt.now())
# => {year: 2026, month: 4, day: 19, hour: 21, minute: 55, second: 35, ms: 236, weekday: "Sunday"}
```

## `from_components`

`from_components(map)`

Constructs a timestamp from a components `Map`.
Missing fields default to their minimum: `year` to 1970, `month` and `day` to 1, time fields to 0.
Invalid dates (e.g., Feb 30) and out-of-range time fields (e.g., hour 25) are rejected.
Unknown keys (including `weekday`) are ignored, so the output of `components` can be passed directly.

```frost
dt.from_components({year: 2024, month: 3, day: 16})
dt.from_components({year: 2024, month: 3, day: 16, hour: 14, minute: 30})

# modify a timestamp via components
def c = dt.components(dt.now()) + {hour: 0, minute: 0, second: 0, ms: 0}
def midnight = dt.from_components(c)
```

## `epoch`

The constant `0` — the Unix epoch (1970-01-01T00:00:00 UTC).
Provided for readability: `dt.epoch` is clearer than a bare `0` when expressing intent.

## `ms`

A sub-map of duration constants in milliseconds.

| Constant | Value |
|---|---|
| `ms.second` | `1000` |
| `ms.minute` | `60000` |
| `ms.hour` | `3600000` |
| `ms.day` | `86400000` |
| `ms.week` | `604800000` |

Month and year constants are not provided because their durations vary.
