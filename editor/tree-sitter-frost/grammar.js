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
        commaSep1NoTrailing($.destructure_name),
        optional(seq(',', $.rest_pattern)),
      ),
    ),

    rest_pattern: $ => seq('...', field('name', $.destructure_name)),

    destructure_name: $ => $.identifier,

    map_destructure_pattern: $ => seq(
      '{',
      optional(field('entries', $.map_destructure_entry_list)),
      '}',
    ),

    map_destructure_entry_list: $ => commaSep1NoTrailing($.map_destructure_entry),

    map_destructure_entry: $ => choice(
      // Computed key: binding always required.
      seq(
        field('key', $.map_key_expression),
        ':',
        field('name', $.destructure_name),
      ),
      // Identifier key: explicit binding, or shorthand (key name used as binding).
      seq(
        field('key', $.identifier),
        optional(seq(':', field('name', $.destructure_name))),
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
      $.literal,
      $.array_literal,
      $.map_literal,
      $.parenthesized_expression,
      $.if_expression,
      $.lambda_expression,
      $.do_block,
      $.map_expression,
      $.filter_expression,
      $.reduce_expression,
      $.foreach_expression,
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
      $.do_block,
      $.map_expression,
      $.filter_expression,
      $.reduce_expression,
      $.foreach_expression,
      $.identifier,
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
      $.kw_with,
      field('operation', $.expression),
      optional(seq($.kw_init, ':', field('init', $.expression))),
    )),

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
      $.double_string,
      $.single_string,
      $.raw_string_double,
      $.raw_string_single,
      $.format_string_double,
      $.format_string_single,
    ),

    float: _ => token(/[0-9]+\.[0-9]+/),
    integer: _ => token(/[0-9]+/),

    boolean: $ => choice($.kw_true, $.kw_false),
    null: $ => $.kw_null,

    // Atomic tokens: no extras (including comments) can be injected inside.
    double_string: _ => token(seq('"', /[^"\\\n]*(?:\\.[^"\\\n]*)*/, '"')),
    single_string: _ => token(seq("'", /[^'\\\n]*(?:\\.[^'\\\n]*)*/u, "'")),

    raw_string_double: _ => token(/R"\([^\n]*\)"/),
    raw_string_single: _ => token(/R'\([^\n]*\)'/),

    format_string_double: $ => seq(
      token('$"'),
      repeat(choice(
        $.format_placeholder,
        $.format_invalid_placeholder,
        $.format_escape_sequence,
        $.format_invalid_escape_sequence,
        $.format_content_double,
        $.format_dollar,
      )),
      token.immediate('"'),
    ),

    format_string_single: $ => seq(
      token("$'"),
      repeat(choice(
        $.format_placeholder,
        $.format_invalid_placeholder,
        $.format_single_escape_sequence,
        $.format_single_invalid_escape_sequence,
        $.format_content_single,
        $.format_dollar,
      )),
      token.immediate("'"),
    ),

    format_content_double: _ => token.immediate(/[^"\\$\n]+/),
    format_content_single: _ => token.immediate(/[^'\\$\n]+/),

    format_dollar: _ => token.immediate('$'),

    format_escape_sequence: _ => token.immediate(/\\[ntr"\\0$]/),
    format_invalid_escape_sequence: _ => token.immediate(/\\./),
    format_single_escape_sequence: _ => token.immediate(/\\[ntr'\\0$]/),
    format_single_invalid_escape_sequence: _ => token.immediate(/\\./),

    format_placeholder: $ => prec(2, seq(
      token.immediate('${'),
      field('name', $.identifier),
      token.immediate('}'),
    )),

    format_invalid_placeholder: $ => prec(1, seq(
      token.immediate('${'),
      optional(field('content', $.format_placeholder_content)),
      token.immediate('}'),
    )),

    // Keep invalid placeholder content at lower lexical precedence so
    // `${ident}` matches `format_placeholder` first.
    format_placeholder_content: _ => token.immediate(prec(-1, /[^}\n]+/)),

    identifier: _ => token(/[A-Za-z_][A-Za-z0-9_]*/),

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
