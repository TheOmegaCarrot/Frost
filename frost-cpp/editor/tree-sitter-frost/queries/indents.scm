[
  (block)
  (if_expression)
  (parenthesized_expression)
  (no_nl_arguments)
  (array_literal)
  (map_literal)
  (array_destructure_pattern)
  (map_destructure_pattern)
  (map_key_expression)
  (lambda_parameters_parenthesized)
] @indent.begin

(postfix_expression
  value: (postfix_expression)
  "@"
  call: (threaded_call_target)) @indent.begin

(lambda_expression
  body: (or_expression)) @indent.begin

[
  (elif_clause)
  (else_clause)
] @indent.branch

[
  ")"
  "]"
  "}"
] @indent.end @indent.branch

(comment) @indent.ignore
