(comment) @comment

[
  (kw_if)
  (kw_else)
  (kw_elif)
  (kw_def)
  (kw_defn)
  (kw_export)
  (kw_fn)
  (kw_do)
  (kw_reduce)
  (kw_map)
  (kw_foreach)
  (kw_filter)
  (kw_with)
  (kw_init)
  (kw_match)
  (kw_is)
  (kw_as)
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
    (triple_string_double)
    (triple_string_single)
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
] @error

(identifier) @variable

(dollar_identifier) @variable.parameter

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

(match_map_entry
  key: (identifier) @property)

(match_type_constraint
  type: (identifier) @type)

[
  "..."
  "->"
  "=>"
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
  "|"
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

(abbreviated_lambda "$(" @punctuation.bracket)
(abbreviated_lambda ")" @punctuation.bracket)

[
  ":"
  ","
  ";"
] @punctuation.delimiter

(no_nl_dot) @punctuation.delimiter

(format_placeholder "${" @punctuation.special)
(format_placeholder "}" @punctuation.special)
