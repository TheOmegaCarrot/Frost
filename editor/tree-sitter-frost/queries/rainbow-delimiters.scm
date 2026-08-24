;; Rainbow delimiters query for Frost.
;; Required captures: @container and @delimiter.

(parenthesized_expression
  "(" @delimiter
  ")" @delimiter @sentinel) @container

(lambda_parameters_parenthesized
  "(" @delimiter
  ")" @delimiter @sentinel) @container

(no_nl_arguments
  (no_nl_call_open) @delimiter
  ")" @delimiter @sentinel) @container

(array_literal
  "[" @delimiter
  "]" @delimiter @sentinel) @container

(map_literal
  "{" @delimiter
  "}" @delimiter @sentinel) @container

(array_destructure_pattern
  "[" @delimiter
  "]" @delimiter @sentinel) @container

(map_destructure_pattern
  "{" @delimiter
  "}" @delimiter @sentinel) @container

(map_key_expression
  "[" @delimiter
  "]" @delimiter @sentinel) @container

(block
  "{" @delimiter
  "}" @delimiter @sentinel) @container

;; Postfix index forms use a no-newline index opener token.
(postfix_expression
  (no_nl_index_open) @delimiter
  "]" @delimiter @sentinel) @container

(threaded_callee
  (no_nl_index_open) @delimiter
  "]" @delimiter @sentinel) @container
