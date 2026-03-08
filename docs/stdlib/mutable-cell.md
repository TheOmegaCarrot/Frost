# Mutable Cell

A `mutable_cell` is a mutable reference cell. It is the only built-in mechanism
for mutable state in Frost. Functions (including functions nested inside arrays
or maps) cannot be stored in a cell.

## `mutable_cell`
`mutable_cell()`
`mutable_cell(initial)`

Creates a new mutable cell. Without `initial`, the cell starts as `null`.

### `.get`
`.get()`

Returns the current value of the cell.

### `.exchange`
`.exchange(value)`

Sets the cell to `value` and returns the previous value.
