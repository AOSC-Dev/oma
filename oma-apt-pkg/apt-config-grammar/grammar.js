/// <reference types="tree-sitter-cli/dist/grammar" />

module.exports = grammar({
  name: 'apt_config',

  extras: $ => [
    /\s/,
    $.line_comment,
    $.block_comment,
  ],

  rules: {
    source_file: $ => repeat($._statement),

    _statement: $ => choice(
      $.key_value,
      $.list_value,
      $.scope,
      $.include_directive,
      $.clear_directive,
      $.unknown_statement,
    ),

    include_directive: $ => seq(
      '#include',
      field('path', $.string),
    ),

    clear_directive: $ => seq(
      '#clear',
      field('key', $.path),
    ),

    unknown_statement: $ => seq(
      '#',
      /[^\n]*/,
    ),

    key_value: $ => seq(
      field('key', $.path),
      repeat(field('value', $.string)),
      ';',
    ),

    list_value: $ => seq(
      field('value', $.string),
      ';',
    ),

    scope: $ => seq(
      field('key', $.path),
      '{',
      repeat($._statement),
      '}',
      optional(';'),
    ),

    path: $ => prec.left(seq(
      $.identifier,
      repeat(seq('::', $.identifier)),
    )),

    identifier: $ => /[a-zA-Z0-9_\-.+]+/,

    string: $ => token(seq('"', repeat(choice(
      /[^"\\\n]/,
      /\\./,
    )), '"')),

    line_comment: $ => token(seq('//', /[^\n]*/)),

    block_comment: $ => token(seq(
      '/*',
      repeat(choice(
        /[^*]/,
        seq('*', /[^/]/),
      )),
      optional('*'),
      '*/',
    )),
  },
});
