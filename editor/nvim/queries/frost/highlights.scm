(comment) @comment

[
  (kw_if)
  (kw_else)
  (kw_elif)
  (kw_def)
  (kw_export)
  (kw_fn)
  (kw_reduce)
  (kw_map)
  (kw_foreach)
  (kw_filter)
  (kw_with)
  (kw_init)
  (kw_and)
  (kw_or)
  (kw_not)
] @keyword

(boolean) @constant.builtin
(null) @constant.builtin

(integer) @number
(float) @number

(
  [
    (double_string)
    (single_string)
    (raw_string_double)
    (raw_string_single)
  ] @string
)

(
  [
    (format_string_double)
    (format_string_single)
  ] @string.special
)

[
  (format_escape_sequence)
  (format_single_escape_sequence)
] @string.escape

[
  (format_invalid_escape_sequence)
  (format_single_invalid_escape_sequence)
  (format_invalid_placeholder)
] @error

(format_placeholder
  name: (identifier) @variable.parameter)

(identifier) @variable

(postfix_expression
  function: (postfix_expression
    (primary_expression
      (identifier) @function)))

(threaded_call_target
  function: (threaded_callee
    (threaded_callee_primary
      (identifier) @function)))

(postfix_expression
  function: (postfix_expression
    property: (identifier) @function))

(postfix_expression
  property: (identifier) @property)

(map_entry
  key: (identifier) @property)

(map_destructure_entry
  key: (identifier) @property)

[
  "..."
  "->"
  "@"
  "=="
  "!="
  "<="
  ">="
  "<"
  ">"
  "+"
  "-"
  "*"
  "/"
  "%"
  "="
] @operator

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  (no_nl_call_open)
  (no_nl_index_open)
] @punctuation.bracket

[
  ":"
  ","
  ";"
] @punctuation.delimiter

(no_nl_dot) @punctuation.delimiter
