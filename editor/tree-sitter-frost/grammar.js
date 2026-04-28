const PREC = {
  OR: 1,
  AND: 2,
  EQUALITY: 3,
  COMPARISON: 4,
  SUM: 5,
  PRODUCT: 6,
  UNARY: 7,
  POSTFIX: 8,
};

module.exports = grammar({
  name: 'frost',

  word: $ => $.identifier,

  reserved: {
    global: $ => [
      $.kw_if,
      $.kw_else,
      $.kw_elif,
      $.kw_def,
      $.kw_defn,
      $.kw_export,
      $.kw_fn,
      $.kw_reduce,
      $.kw_map,
      $.kw_foreach,
      $.kw_filter,
      $.kw_with,
      $.kw_init,
      $.kw_match,
      $.kw_is,
      $.kw_as,
      $.kw_true,
      $.kw_false,
      $.kw_and,
      $.kw_do,
      $.kw_or,
      $.kw_not,
      $.kw_null,
    ],
  },

  extras: $ => [
    /[\s\uFEFF\u2060\u200B\u00A0]/,
    $.comment,
  ],

  supertypes: $ => [
    $.statement,
    $.expression,
    $.literal,
    $.pattern,
  ],

  conflicts: $ => [
    [$.map_literal, $.block],
    [$.map_key_expression, $.array_elements],
  ],

  rules: {
    source_file: $ => repeat(choice($.statement, ';')),

    comment: _ => token(seq('#', /[^\n]*/)),

    statement: $ => choice(
      $.export_definition,
      $.export_defn_statement,
      $.definition,
      $.defn_statement,
      $.expression_statement,
    ),

    expression_statement: $ => $.expression,

    export_definition: $ => seq(
      $.kw_export,
      $.kw_def,
      field('pattern', $.pattern),
      '=',
      field('value', $.expression),
    ),

    export_defn_statement: $ => seq(
      $.kw_export,
      $.kw_defn,
      field('name', $.identifier),
      field('parameters', $.lambda_parameters_parenthesized),
      '->',
      field('body', choice($.block, $.expression)),
    ),

    definition: $ => seq(
      $.kw_def,
      field('pattern', $.pattern),
      '=',
      field('value', $.expression),
    ),

    defn_statement: $ => seq(
      $.kw_defn,
      field('name', $.identifier),
      field('parameters', $.lambda_parameters_parenthesized),
      '->',
      field('body', choice($.block, $.expression)),
    ),

    pattern: $ => choice(
      $.identifier,
      $.array_destructure_pattern,
      $.map_destructure_pattern,
    ),

    array_destructure_pattern: $ => seq(
      '[',
      optional(field('elements', $.array_destructure_elements)),
      ']',
    ),

    array_destructure_elements: $ => choice(
      $.rest_pattern,
      seq(
        commaSep1NoTrailing($.pattern),
        optional(seq(',', $.rest_pattern)),
      ),
    ),

    rest_pattern: $ => seq('...', field('name', $.identifier)),

    map_destructure_pattern: $ => seq(
      '{',
      optional(field('entries', $.map_destructure_entry_list)),
      '}',
      optional(seq($.kw_as, field('as', $.identifier))),
    ),

    map_destructure_entry_list: $ => commaSep1NoTrailing($.map_destructure_entry),

    map_destructure_entry: $ => choice(
      // Computed key: binding always required.
      seq(
        field('key', $.map_key_expression),
        ':',
        field('name', $.pattern),
      ),
      // Identifier key: explicit binding, or shorthand (key name used as binding).
      seq(
        field('key', $.identifier),
        optional(seq(':', field('name', $.pattern))),
      ),
    ),

    map_key_expression: $ => seq('[', field('expression', $.expression), ']'),

    expression: $ => $.or_expression,

    or_expression: $ => choice(
      prec.left(PREC.OR, seq(
        field('left', $.or_expression),
        field('operator', $.kw_or),
        field('right', $.and_expression),
      )),
      $.and_expression,
    ),

    and_expression: $ => choice(
      prec.left(PREC.AND, seq(
        field('left', $.and_expression),
        field('operator', $.kw_and),
        field('right', $.equality_expression),
      )),
      $.equality_expression,
    ),

    // Equality is intentionally non-chainable, matching the interpreter.
    equality_expression: $ => choice(
      prec.left(PREC.EQUALITY, seq(
        field('left', $.comparison_expression),
        field('operator', choice('==', '!=')),
        field('right', $.comparison_expression),
      )),
      $.comparison_expression,
    ),

    // Comparison is intentionally non-chainable, matching the interpreter.
    comparison_expression: $ => choice(
      prec.left(PREC.COMPARISON, seq(
        field('left', $.sum_expression),
        field('operator', choice('<', '<=', '>', '>=')),
        field('right', $.sum_expression),
      )),
      $.sum_expression,
    ),

    sum_expression: $ => choice(
      prec.left(PREC.SUM, seq(
        field('left', $.sum_expression),
        field('operator', choice('+', '-')),
        field('right', $.product_expression),
      )),
      $.product_expression,
    ),

    product_expression: $ => choice(
      prec.left(PREC.PRODUCT, seq(
        field('left', $.product_expression),
        field('operator', choice('*', '/', '%')),
        field('right', $.unary_expression),
      )),
      $.unary_expression,
    ),

    unary_expression: $ => choice(
      prec(PREC.UNARY, seq(
        field('operator', choice('-', $.kw_not)),
        field('operand', $.unary_expression),
      )),
      $.postfix_expression,
    ),

    postfix_expression: $ => choice(
      $.primary_expression,

      // Postfix operators cannot continue across newlines.
      prec.left(PREC.POSTFIX, seq(
        field('function', $.postfix_expression),
        field('arguments', $.no_nl_arguments),
      )),

      // Postfix operators cannot continue across newlines.
      prec.left(PREC.POSTFIX, seq(
        field('value', $.postfix_expression),
        $.no_nl_index_open,
        field('index', $.expression),
        ']',
      )),

      // Postfix operators cannot continue across newlines.
      prec.left(PREC.POSTFIX, seq(
        field('value', $.postfix_expression),
        $.no_nl_dot,
        field('property', $.identifier),
      )),

      // Threaded calls are newline-tolerant to support chained pipelines.
      prec.left(PREC.POSTFIX, seq(
        field('value', $.postfix_expression),
        '@',
        field('call', $.threaded_call_target),
      )),
    ),

    threaded_call_target: $ => seq(
      field('function', $.threaded_callee),
      field('arguments', $.no_nl_arguments),
    ),

    threaded_callee: $ => choice(
      $.threaded_callee_primary,

      prec.left(PREC.POSTFIX, seq(
        field('value', $.threaded_callee),
        $.no_nl_index_open,
        field('index', $.expression),
        ']',
      )),

      prec.left(PREC.POSTFIX, seq(
        field('value', $.threaded_callee),
        $.no_nl_dot,
        field('property', $.identifier),
      )),
    ),

    threaded_callee_primary: $ => choice(
      $.identifier,
      $.dollar_identifier,
      $.literal,
      $.array_literal,
      $.map_literal,
      $.parenthesized_expression,
      $.if_expression,
      $.lambda_expression,
      $.abbreviated_lambda,
      $.do_block,
      $.map_expression,
      $.filter_expression,
      $.reduce_expression,
      $.foreach_expression,
      $.match_expression,
    ),

    no_nl_arguments: $ => seq(
      $.no_nl_call_open,
      optional(field('items', $.argument_list)),
      ')',
    ),

    argument_list: $ => commaSep1NoTrailing($.expression),

    primary_expression: $ => choice(
      $.if_expression,
      $.lambda_expression,
      $.abbreviated_lambda,
      $.do_block,
      $.map_expression,
      $.filter_expression,
      $.reduce_expression,
      $.foreach_expression,
      $.match_expression,
      $.identifier,
      $.dollar_identifier,
      $.literal,
      $.array_literal,
      $.map_literal,
      $.parenthesized_expression,
    ),

    parenthesized_expression: $ => seq('(', $.expression, ')'),

    if_expression: $ => prec.right(seq(
      $.kw_if,
      field('condition', $.expression),
      ':',
      field('consequence', $.expression),
      repeat(field('elif', $.elif_clause)),
      optional(field('else', $.else_clause)),
    )),

    elif_clause: $ => seq(
      $.kw_elif,
      field('condition', $.expression),
      ':',
      field('consequence', $.expression),
    ),

    else_clause: $ => seq(
      $.kw_else,
      ':',
      field('alternative', $.expression),
    ),

    lambda_expression: $ => seq(
      $.kw_fn,
      choice(
        seq(
          field('parameters', $.lambda_parameters_parenthesized),
          '->',
          field('body', choice($.block, $.expression)),
        ),
        seq(
          field('parameters', $.lambda_parameters_elided),
          '->',
          field('body', choice($.block, $.expression)),
        ),
        seq(
          '->',
          field('body', choice($.block, $.expression)),
        ),
      ),
    ),

    lambda_parameters_parenthesized: $ => seq(
      '(',
      optional(field('payload', $.lambda_parameter_payload)),
      ')',
    ),

    lambda_parameters_elided: $ => field('payload', $.lambda_parameter_payload),

    lambda_parameter_payload: $ => choice(
      $.lambda_rest_parameter,
      seq(
        commaSep1NoTrailing($.identifier),
        optional(seq(',', $.lambda_rest_parameter)),
      ),
    ),

    lambda_rest_parameter: $ => seq('...', field('name', $.identifier)),

    block: $ => seq(
      '{',
      repeat(choice($.statement, ';')),
      '}',
    ),

    do_block: $ => seq(
      $.kw_do,
      field('body', $.block),
    ),

    map_expression: $ => seq(
      $.kw_map,
      field('structure', $.expression),
      $.kw_with,
      field('operation', $.expression),
    ),

    filter_expression: $ => seq(
      $.kw_filter,
      field('structure', $.expression),
      $.kw_with,
      field('operation', $.expression),
    ),

    foreach_expression: $ => seq(
      $.kw_foreach,
      field('structure', $.expression),
      $.kw_with,
      field('operation', $.expression),
    ),

    reduce_expression: $ => prec.right(seq(
      $.kw_reduce,
      field('structure', $.expression),
      optional(seq($.kw_init, ':', field('init', $.expression))),
      $.kw_with,
      field('operation', $.expression),
    )),

    // `match TARGET { PATTERN (if: GUARD)? => RESULT, ... }`
    //
    // Arms are required to be comma-separated. A trailing comma is allowed.
    // The arm list is optional (an empty `match v {}` parses).
    match_expression: $ => prec.right(seq(
      $.kw_match,
      field('target', $.expression),
      '{',
      optional(field('arms', $.match_arm_list)),
      '}',
    )),

    match_arm_list: $ => seq(
      $.match_arm,
      repeat(seq(',', $.match_arm)),
      optional(','),
    ),

    match_arm: $ => seq(
      field('pattern', $.match_pattern),
      optional(field('guard', $.match_guard)),
      '=>',
      field('result', $.expression),
    ),

    // `if: EXPRESSION` -- the colon distinguishes this from a normal `if`
    // expression and matches Frost's other if-positions.
    match_guard: $ => seq(
      $.kw_if,
      ':',
      field('condition', $.expression),
    ),

    match_pattern: $ => seq(
      $.match_pattern_atom,
      repeat(seq('|', $.match_pattern_atom)),
    ),

    match_pattern_atom: $ => choice(
      $.match_binding_pattern,
      $.match_value_pattern,
      $.match_array_pattern,
      $.match_map_pattern,
    ),

    // Identifier (or `_` discard) optionally followed by `is TYPE`. Type
    // names are contextual keywords -- they're parsed as identifiers here
    // and validated by the AST builder.
    match_binding_pattern: $ => seq(
      field('name', $.identifier),
      optional(field('constraint', $.match_type_constraint)),
    ),

    match_type_constraint: $ => seq(
      $.kw_is,
      field('type', $.identifier),
    ),

    // Primitive literal OR a parenthesized expression. The parens
    // explicitly mark "this is a value to compare against, not a binding".
    match_value_pattern: $ => choice(
      $.literal,
      seq('(', field('expression', $.expression), ')'),
    ),

    match_array_pattern: $ => seq(
      '[',
      optional(field('elements', $.match_array_elements)),
      ']',
    ),

    match_array_elements: $ => choice(
      $.match_rest_pattern,
      seq(
        $.match_pattern,
        repeat(seq(',', $.match_pattern)),
        optional(seq(',', optional($.match_rest_pattern))),
      ),
    ),

    // `...name` (named rest) or `..._` (discard rest).
    match_rest_pattern: $ => seq('...', field('name', $.identifier)),

    match_map_pattern: $ => seq(
      '{',
      optional(field('entries', $.match_map_entry_list)),
      '}',
      optional(seq($.kw_as, field('as', $.identifier))),
    ),

    match_map_entry_list: $ => seq(
      $.match_map_entry,
      repeat(seq(',', $.match_map_entry)),
      optional(','),
    ),

    // Four forms:
    //   [expr]: pattern         -- computed key
    //   identifier: pattern     -- named key with explicit pattern
    //   identifier              -- shorthand: key name doubles as binding
    //   identifier is TYPE      -- shorthand with type constraint
    match_map_entry: $ => choice(
      seq(
        field('key', $.map_key_expression),
        ':',
        field('pattern', $.match_pattern),
      ),
      seq(
        field('key', $.identifier),
        ':',
        field('pattern', $.match_pattern),
      ),
      seq(
        field('key', $.identifier),
        field('constraint', $.match_type_constraint),
      ),
      field('key', $.identifier),
    ),

    array_literal: $ => seq('[', optional(field('elements', $.array_elements)), ']'),

    array_elements: $ => commaSep1($.expression),

    map_literal: $ => seq('{', optional(field('entries', $.map_entries)), '}'),

    map_entries: $ => commaSep1($.map_entry),

    map_entry: $ => seq(
      field('key', choice($.identifier, $.map_key_expression)),
      ':',
      field('value', $.expression),
    ),

    literal: $ => choice(
      $.float,
      $.integer,
      $.boolean,
      $.null,
      $.triple_string_double,
      $.triple_string_single,
      $.double_string,
      $.single_string,
      $.raw_string_double,
      $.raw_string_single,
      $.format_string_double,
      $.format_string_single,
    ),

    float: _ => token(/[0-9]+\.[0-9]+(e[+-]?[0-9]+)?|[0-9]+e[+-]?[0-9]+/),
    integer: _ => token(/[0-9]+/),

    boolean: $ => choice($.kw_true, $.kw_false),
    null: $ => $.kw_null,

    // Atomic tokens: no extras (including comments) can be injected inside.
    triple_string_double: _ => token(seq('"""', /([^"\\]|\\.|"[^"]|""[^"])*/, '"""')),
    triple_string_single: _ => token(seq("'''", /([^'\\]|\\.|'[^']|''[^'])*/, "'''")),
    double_string: _ => token(seq('"', /[^"\\\n]*(?:\\.[^"\\\n]*)*/, '"')),
    single_string: _ => token(seq("'", /[^'\\\n]*(?:\\.[^'\\\n]*)*/u, "'")),

    raw_string_double: _ => token(/R"\([^\n]*\)"/),
    raw_string_single: _ => token(/R'\([^\n]*\)'/),

    format_string_double: $ => seq(
      token(prec(2, '$"')),
      repeat(choice(
        $.format_placeholder,
        $.format_escape_sequence,
        $.format_invalid_escape_sequence,
        $.format_content_double,
        $.format_dollar,
      )),
      token.immediate('"'),
    ),

    format_string_single: $ => seq(
      token(prec(2, "$'")),
      repeat(choice(
        $.format_placeholder,
        $.format_single_escape_sequence,
        $.format_single_invalid_escape_sequence,
        $.format_content_single,
        $.format_dollar,
      )),
      token.immediate("'"),
    ),

    format_content_double: _ => token.immediate(prec(1, /[^"\\$\n]+/)),
    format_content_single: _ => token.immediate(prec(1, /[^'\\$\n]+/)),

    format_dollar: _ => token.immediate('$'),

    format_escape_sequence: _ => token.immediate(/\\[ntr"\\0$]/),
    format_invalid_escape_sequence: _ => token.immediate(/\\./),
    format_single_escape_sequence: _ => token.immediate(/\\[ntr'\\0$]/),
    format_single_invalid_escape_sequence: _ => token.immediate(/\\./),

    // `${expr}` -- expression interpolation. The expression is a full
    // Frost expression; `}` terminates it naturally.
    format_placeholder: $ => seq(
      token.immediate('${'),
      field('expression', $.expression),
      '}',
    ),

    identifier: _ => token(/[A-Za-z_][A-Za-z0-9_]*/),

    // `$`, `$N` (single digit), or `$$`. Placeholder parameter names for
    // abbreviated lambdas. Higher precedence than format strings so `$1`
    // is not confused with `$` + integer `1`.
    dollar_identifier: _ => token(prec(1, /\$\$|\$[0-9]|\$/)),

    // `$(expr)` -- abbreviated lambda. The `$` and `(` must be adjacent.
    abbreviated_lambda: $ => seq(
      token(prec(2, '$(')),
      field('body', $.expression),
      ')',
    ),

    no_nl_call_open: _ => token.immediate(prec(1, /[ \t\r\f\v]*\(/)),
    no_nl_index_open: _ => token.immediate(prec(1, /[ \t\r\f\v]*\[/)),
    no_nl_dot: _ => token.immediate(prec(1, /[ \t\r\f\v]*\./)),

    kw_do: _ => token('do'),
    kw_if: _ => token('if'),
    kw_else: _ => token('else'),
    kw_elif: _ => token('elif'),
    kw_def: _ => token('def'),
    kw_defn: _ => token('defn'),
    kw_export: _ => token('export'),
    kw_fn: _ => token('fn'),
    kw_reduce: _ => token('reduce'),
    kw_map: _ => token('map'),
    kw_foreach: _ => token('foreach'),
    kw_filter: _ => token('filter'),
    kw_with: _ => token('with'),
    kw_init: _ => token('init'),
    kw_match: _ => token('match'),
    kw_is: _ => token('is'),
    kw_as: _ => token('as'),
    kw_true: _ => token('true'),
    kw_false: _ => token('false'),
    kw_and: _ => token('and'),
    kw_or: _ => token('or'),
    kw_not: _ => token('not'),
    kw_null: _ => token('null'),
  },
});

function commaSep(rule) {
  return optional(commaSep1(rule));
}

function commaSep1(rule) {
  return seq(rule, repeat(seq(',', rule)), optional(','));
}

function commaSepNoTrailing(rule) {
  return optional(commaSep1NoTrailing(rule));
}

function commaSep1NoTrailing(rule) {
  return seq(rule, repeat(seq(',', rule)));
}
