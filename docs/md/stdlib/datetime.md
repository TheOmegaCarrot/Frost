# Datetime

```frost
def dt = import('std.datetime')
```

Timestamps, formatting, parsing, and calendar components.

All timestamps are `Int` values representing Unix milliseconds (ms since 1970-01-01T00:00:00 UTC). Negative values represent dates before the epoch. Integer arithmetic can be used for duration math with the help of the `ms` constants.

```frost
def tomorrow = dt.now() + dt.ms.day
def in_3_hours = dt.now() + 3 * dt.ms.hour
```

All functions which take a timezone default to UTC. An optional timezone parameter accepts [IANA timezone names](https://en.wikipedia.org/wiki/List_of_tz_database_zones) (e.g., `"America/New_York"`, `"Asia/Tokyo"`).

Format patterns follow the [`std::chrono::format`](https://en.cppreference.com/w/cpp/chrono/format) specification.

## `now`

`dt.now()`

Returns the current time as Unix milliseconds.

## `format`

`dt.format(millis, pattern)`
`dt.format(millis, pattern, timezone)`

Formats a timestamp as a `String` using a chrono format pattern.

```frost
dt.format(dt.now(), '%Y-%m-%d %H:%M:%S')
# => "2026-04-19 14:30:00.000"

dt.format(dt.now(), '%Y-%m-%d %H:%M', 'America/New_York')
# => "2026-04-19 10:30"
```

## `parse`

`dt.parse(s, pattern)`
`dt.parse(s, pattern, timezone)`

Parses a date/time string according to a pattern and returns Unix milliseconds. Produces an error on invalid input or pattern mismatch. Fields not specified by the pattern default to their epoch values (1970-01-01 00:00:00 UTC).

When a timezone is provided, the input is interpreted as local time in that timezone.

```frost
dt.parse('2024-03-16', '%Y-%m-%d')
dt.parse('2024-03-15 20:00:00', '%Y-%m-%d %H:%M:%S', 'America/New_York')
```

## `components`

`dt.components(millis)`
`dt.components(millis, timezone)`

Breaks a timestamp into its calendar components. Returns a `Map` with keys `year`, `month` (1-12), `day` (1-31), `hour`, `minute`, `second`, `ms`, and `weekday` (a `String` such as `"Monday"`).

```frost
dt.components(dt.now())
# => {year: 2026, month: 4, day: 19, hour: 21, minute: 55, ...}

dt.components(dt.now(), 'Asia/Tokyo')
# => {year: 2026, month: 4, day: 20, hour: 6, minute: 55, ...}
```

See also:
[`from_components`](datetime.md#from_components)

## `from_components`

`dt.from_components(map)`
`dt.from_components(map, timezone)`

Constructs a timestamp from a components `Map`. Missing fields default to the epoch: `year` to 1970, `month` and `day` to 1, time fields to 0. Invalid dates (e.g., Feb 30) and out-of-range time fields (e.g., hour 25) are rejected. Unknown keys (including `weekday`) are ignored, so the output of `components` can be passed directly.

When a timezone is provided, the components are interpreted as local time in that timezone.

```frost
dt.from_components({year: 2024, month: 3, day: 16})
dt.from_components({year: 2024, month: 3, day: 16, hour: 14, minute: 30})

# "noon in Tokyo" as UTC millis
dt.from_components({year: 2024, month: 3, day: 16, hour: 12}, 'Asia/Tokyo')

# modify a timestamp via components
def c = dt.components(dt.now()) + {hour: 0, minute: 0, second: 0, ms: 0}
def midnight_today = dt.from_components(c)
```

See also:
[`components`](datetime.md#components)

## `epoch`

The constant `0`: the Unix epoch (1970-01-01T00:00:00 UTC). Provided for readability, as `dt.epoch` is clearer than a bare `0` when expressing intent.

## `ms`

A sub-map of duration constants in milliseconds, for performing arithmetic on timestamps.

```frost
def in_2_hours = dt.now() + 2 * dt.ms.hour
```

|  Constant | Value  |
| ---|--- |
|  `ms.second` | `1000`  |
|  `ms.minute` | `60000`  |
|  `ms.hour` | `3600000`  |
|  `ms.day` | `86400000`  |
|  `ms.week` | `604800000`  |

Month and year constants are not provided because their durations vary.

