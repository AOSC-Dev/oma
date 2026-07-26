#include "tree_sitter/parser.h"

#if defined(__GNUC__) || defined(__clang__)
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#endif

#define LANGUAGE_VERSION 14
#define STATE_COUNT 54
#define LARGE_STATE_COUNT 9
#define SYMBOL_COUNT 25
#define ALIAS_COUNT 0
#define TOKEN_COUNT 13
#define EXTERNAL_TOKEN_COUNT 0
#define FIELD_COUNT 3
#define MAX_ALIAS_SEQUENCE_LENGTH 5
#define PRODUCTION_ID_COUNT 7

enum ts_symbol_identifiers {
  anon_sym_POUNDinclude = 1,
  anon_sym_POUNDclear = 2,
  anon_sym_POUND = 3,
  aux_sym_unknown_statement_token1 = 4,
  anon_sym_SEMI = 5,
  anon_sym_LBRACE = 6,
  anon_sym_RBRACE = 7,
  anon_sym_COLON_COLON = 8,
  sym_identifier = 9,
  sym_string = 10,
  sym_line_comment = 11,
  sym_block_comment = 12,
  sym_source_file = 13,
  sym__statement = 14,
  sym_include_directive = 15,
  sym_clear_directive = 16,
  sym_unknown_statement = 17,
  sym_key_value = 18,
  sym_list_value = 19,
  sym_scope = 20,
  sym_path = 21,
  aux_sym_source_file_repeat1 = 22,
  aux_sym_key_value_repeat1 = 23,
  aux_sym_path_repeat1 = 24,
};

static const char * const ts_symbol_names[] = {
  [ts_builtin_sym_end] = "end",
  [anon_sym_POUNDinclude] = "#include",
  [anon_sym_POUNDclear] = "#clear",
  [anon_sym_POUND] = "#",
  [aux_sym_unknown_statement_token1] = "unknown_statement_token1",
  [anon_sym_SEMI] = ";",
  [anon_sym_LBRACE] = "{",
  [anon_sym_RBRACE] = "}",
  [anon_sym_COLON_COLON] = "::",
  [sym_identifier] = "identifier",
  [sym_string] = "string",
  [sym_line_comment] = "line_comment",
  [sym_block_comment] = "block_comment",
  [sym_source_file] = "source_file",
  [sym__statement] = "_statement",
  [sym_include_directive] = "include_directive",
  [sym_clear_directive] = "clear_directive",
  [sym_unknown_statement] = "unknown_statement",
  [sym_key_value] = "key_value",
  [sym_list_value] = "list_value",
  [sym_scope] = "scope",
  [sym_path] = "path",
  [aux_sym_source_file_repeat1] = "source_file_repeat1",
  [aux_sym_key_value_repeat1] = "key_value_repeat1",
  [aux_sym_path_repeat1] = "path_repeat1",
};

static const TSSymbol ts_symbol_map[] = {
  [ts_builtin_sym_end] = ts_builtin_sym_end,
  [anon_sym_POUNDinclude] = anon_sym_POUNDinclude,
  [anon_sym_POUNDclear] = anon_sym_POUNDclear,
  [anon_sym_POUND] = anon_sym_POUND,
  [aux_sym_unknown_statement_token1] = aux_sym_unknown_statement_token1,
  [anon_sym_SEMI] = anon_sym_SEMI,
  [anon_sym_LBRACE] = anon_sym_LBRACE,
  [anon_sym_RBRACE] = anon_sym_RBRACE,
  [anon_sym_COLON_COLON] = anon_sym_COLON_COLON,
  [sym_identifier] = sym_identifier,
  [sym_string] = sym_string,
  [sym_line_comment] = sym_line_comment,
  [sym_block_comment] = sym_block_comment,
  [sym_source_file] = sym_source_file,
  [sym__statement] = sym__statement,
  [sym_include_directive] = sym_include_directive,
  [sym_clear_directive] = sym_clear_directive,
  [sym_unknown_statement] = sym_unknown_statement,
  [sym_key_value] = sym_key_value,
  [sym_list_value] = sym_list_value,
  [sym_scope] = sym_scope,
  [sym_path] = sym_path,
  [aux_sym_source_file_repeat1] = aux_sym_source_file_repeat1,
  [aux_sym_key_value_repeat1] = aux_sym_key_value_repeat1,
  [aux_sym_path_repeat1] = aux_sym_path_repeat1,
};

static const TSSymbolMetadata ts_symbol_metadata[] = {
  [ts_builtin_sym_end] = {
    .visible = false,
    .named = true,
  },
  [anon_sym_POUNDinclude] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_POUNDclear] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_POUND] = {
    .visible = true,
    .named = false,
  },
  [aux_sym_unknown_statement_token1] = {
    .visible = false,
    .named = false,
  },
  [anon_sym_SEMI] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_LBRACE] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_RBRACE] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_COLON_COLON] = {
    .visible = true,
    .named = false,
  },
  [sym_identifier] = {
    .visible = true,
    .named = true,
  },
  [sym_string] = {
    .visible = true,
    .named = true,
  },
  [sym_line_comment] = {
    .visible = true,
    .named = true,
  },
  [sym_block_comment] = {
    .visible = true,
    .named = true,
  },
  [sym_source_file] = {
    .visible = true,
    .named = true,
  },
  [sym__statement] = {
    .visible = false,
    .named = true,
  },
  [sym_include_directive] = {
    .visible = true,
    .named = true,
  },
  [sym_clear_directive] = {
    .visible = true,
    .named = true,
  },
  [sym_unknown_statement] = {
    .visible = true,
    .named = true,
  },
  [sym_key_value] = {
    .visible = true,
    .named = true,
  },
  [sym_list_value] = {
    .visible = true,
    .named = true,
  },
  [sym_scope] = {
    .visible = true,
    .named = true,
  },
  [sym_path] = {
    .visible = true,
    .named = true,
  },
  [aux_sym_source_file_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_key_value_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_path_repeat1] = {
    .visible = false,
    .named = false,
  },
};

enum ts_field_identifiers {
  field_key = 1,
  field_path = 2,
  field_value = 3,
};

static const char * const ts_field_names[] = {
  [0] = NULL,
  [field_key] = "key",
  [field_path] = "path",
  [field_value] = "value",
};

static const TSFieldMapSlice ts_field_map_slices[PRODUCTION_ID_COUNT] = {
  [1] = {.index = 0, .length = 1},
  [2] = {.index = 1, .length = 1},
  [3] = {.index = 2, .length = 1},
  [4] = {.index = 3, .length = 1},
  [5] = {.index = 4, .length = 2},
  [6] = {.index = 6, .length = 2},
};

static const TSFieldMapEntry ts_field_map_entries[] = {
  [0] =
    {field_path, 1},
  [1] =
    {field_key, 1},
  [2] =
    {field_value, 0},
  [3] =
    {field_key, 0},
  [4] =
    {field_key, 0},
    {field_value, 1, .inherited = true},
  [6] =
    {field_value, 0, .inherited = true},
    {field_value, 1, .inherited = true},
};

static const TSSymbol ts_alias_sequences[PRODUCTION_ID_COUNT][MAX_ALIAS_SEQUENCE_LENGTH] = {
  [0] = {0},
};

static const uint16_t ts_non_terminal_alias_map[] = {
  0,
};

static const TSStateId ts_primary_state_ids[STATE_COUNT] = {
  [0] = 0,
  [1] = 1,
  [2] = 2,
  [3] = 3,
  [4] = 4,
  [5] = 5,
  [6] = 3,
  [7] = 4,
  [8] = 5,
  [9] = 9,
  [10] = 10,
  [11] = 11,
  [12] = 12,
  [13] = 10,
  [14] = 9,
  [15] = 11,
  [16] = 12,
  [17] = 17,
  [18] = 17,
  [19] = 19,
  [20] = 19,
  [21] = 21,
  [22] = 22,
  [23] = 23,
  [24] = 24,
  [25] = 25,
  [26] = 25,
  [27] = 24,
  [28] = 28,
  [29] = 23,
  [30] = 30,
  [31] = 22,
  [32] = 32,
  [33] = 21,
  [34] = 32,
  [35] = 30,
  [36] = 28,
  [37] = 37,
  [38] = 37,
  [39] = 39,
  [40] = 40,
  [41] = 40,
  [42] = 42,
  [43] = 43,
  [44] = 43,
  [45] = 45,
  [46] = 46,
  [47] = 47,
  [48] = 48,
  [49] = 46,
  [50] = 50,
  [51] = 48,
  [52] = 45,
  [53] = 50,
};

static bool ts_lex(TSLexer *lexer, TSStateId state) {
  START_LEXER();
  eof = lexer->eof(lexer);
  switch (state) {
    case 0:
      if (eof) ADVANCE(18);
      if (lookahead == '"') ADVANCE(1);
      if (lookahead == '#') ADVANCE(21);
      if (lookahead == '/') ADVANCE(2);
      if (lookahead == ':') ADVANCE(6);
      if (lookahead == ';') ADVANCE(28);
      if (lookahead == '{') ADVANCE(29);
      if (lookahead == '}') ADVANCE(30);
      if (('\t' <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') SKIP(0);
      if (lookahead == '+' ||
          ('-' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(32);
      END_STATE();
    case 1:
      if (lookahead == '"') ADVANCE(33);
      if (lookahead == '\\') ADVANCE(17);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(1);
      END_STATE();
    case 2:
      if (lookahead == '*') ADVANCE(4);
      if (lookahead == '/') ADVANCE(34);
      END_STATE();
    case 3:
      if (lookahead == '*') ADVANCE(5);
      if (lookahead == '/') ADVANCE(36);
      if (lookahead != 0) ADVANCE(4);
      END_STATE();
    case 4:
      if (lookahead == '*') ADVANCE(5);
      if (lookahead != 0) ADVANCE(4);
      END_STATE();
    case 5:
      if (lookahead == '*') ADVANCE(3);
      if (lookahead == '/') ADVANCE(35);
      if (lookahead != 0) ADVANCE(4);
      END_STATE();
    case 6:
      if (lookahead == ':') ADVANCE(31);
      END_STATE();
    case 7:
      if (lookahead == 'a') ADVANCE(15);
      END_STATE();
    case 8:
      if (lookahead == 'c') ADVANCE(13);
      END_STATE();
    case 9:
      if (lookahead == 'd') ADVANCE(11);
      END_STATE();
    case 10:
      if (lookahead == 'e') ADVANCE(7);
      END_STATE();
    case 11:
      if (lookahead == 'e') ADVANCE(19);
      END_STATE();
    case 12:
      if (lookahead == 'l') ADVANCE(10);
      END_STATE();
    case 13:
      if (lookahead == 'l') ADVANCE(16);
      END_STATE();
    case 14:
      if (lookahead == 'n') ADVANCE(8);
      END_STATE();
    case 15:
      if (lookahead == 'r') ADVANCE(20);
      END_STATE();
    case 16:
      if (lookahead == 'u') ADVANCE(9);
      END_STATE();
    case 17:
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(1);
      END_STATE();
    case 18:
      ACCEPT_TOKEN(ts_builtin_sym_end);
      END_STATE();
    case 19:
      ACCEPT_TOKEN(anon_sym_POUNDinclude);
      END_STATE();
    case 20:
      ACCEPT_TOKEN(anon_sym_POUNDclear);
      END_STATE();
    case 21:
      ACCEPT_TOKEN(anon_sym_POUND);
      if (lookahead == 'c') ADVANCE(12);
      if (lookahead == 'i') ADVANCE(14);
      END_STATE();
    case 22:
      ACCEPT_TOKEN(aux_sym_unknown_statement_token1);
      if (lookahead == '\n') ADVANCE(4);
      if (lookahead == '*') ADVANCE(24);
      if (lookahead == '/') ADVANCE(23);
      if (lookahead != 0) ADVANCE(23);
      END_STATE();
    case 23:
      ACCEPT_TOKEN(aux_sym_unknown_statement_token1);
      if (lookahead == '\n') ADVANCE(4);
      if (lookahead == '*') ADVANCE(24);
      if (lookahead != 0) ADVANCE(23);
      END_STATE();
    case 24:
      ACCEPT_TOKEN(aux_sym_unknown_statement_token1);
      if (lookahead == '\n') ADVANCE(4);
      if (lookahead == '*') ADVANCE(22);
      if (lookahead == '/') ADVANCE(27);
      if (lookahead != 0) ADVANCE(23);
      END_STATE();
    case 25:
      ACCEPT_TOKEN(aux_sym_unknown_statement_token1);
      if (lookahead == '*') ADVANCE(23);
      if (lookahead == '/') ADVANCE(27);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(27);
      END_STATE();
    case 26:
      ACCEPT_TOKEN(aux_sym_unknown_statement_token1);
      if (lookahead == '/') ADVANCE(25);
      if (lookahead == '\t' ||
          (0x0b <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') ADVANCE(26);
      if (lookahead != 0 &&
          (lookahead < '\t' || '\r' < lookahead)) ADVANCE(27);
      END_STATE();
    case 27:
      ACCEPT_TOKEN(aux_sym_unknown_statement_token1);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(27);
      END_STATE();
    case 28:
      ACCEPT_TOKEN(anon_sym_SEMI);
      END_STATE();
    case 29:
      ACCEPT_TOKEN(anon_sym_LBRACE);
      END_STATE();
    case 30:
      ACCEPT_TOKEN(anon_sym_RBRACE);
      END_STATE();
    case 31:
      ACCEPT_TOKEN(anon_sym_COLON_COLON);
      END_STATE();
    case 32:
      ACCEPT_TOKEN(sym_identifier);
      if (lookahead == '+' ||
          lookahead == '-' ||
          lookahead == '.' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(32);
      END_STATE();
    case 33:
      ACCEPT_TOKEN(sym_string);
      END_STATE();
    case 34:
      ACCEPT_TOKEN(sym_line_comment);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(34);
      END_STATE();
    case 35:
      ACCEPT_TOKEN(sym_block_comment);
      END_STATE();
    case 36:
      ACCEPT_TOKEN(sym_block_comment);
      if (lookahead == '*') ADVANCE(5);
      if (lookahead != 0) ADVANCE(4);
      END_STATE();
    default:
      return false;
  }
}

static const TSLexMode ts_lex_modes[STATE_COUNT] = {
  [0] = {.lex_state = 0},
  [1] = {.lex_state = 0},
  [2] = {.lex_state = 0},
  [3] = {.lex_state = 0},
  [4] = {.lex_state = 0},
  [5] = {.lex_state = 0},
  [6] = {.lex_state = 0},
  [7] = {.lex_state = 0},
  [8] = {.lex_state = 0},
  [9] = {.lex_state = 0},
  [10] = {.lex_state = 0},
  [11] = {.lex_state = 0},
  [12] = {.lex_state = 0},
  [13] = {.lex_state = 0},
  [14] = {.lex_state = 0},
  [15] = {.lex_state = 0},
  [16] = {.lex_state = 0},
  [17] = {.lex_state = 0},
  [18] = {.lex_state = 0},
  [19] = {.lex_state = 0},
  [20] = {.lex_state = 0},
  [21] = {.lex_state = 0},
  [22] = {.lex_state = 0},
  [23] = {.lex_state = 0},
  [24] = {.lex_state = 0},
  [25] = {.lex_state = 0},
  [26] = {.lex_state = 0},
  [27] = {.lex_state = 0},
  [28] = {.lex_state = 0},
  [29] = {.lex_state = 0},
  [30] = {.lex_state = 0},
  [31] = {.lex_state = 0},
  [32] = {.lex_state = 0},
  [33] = {.lex_state = 0},
  [34] = {.lex_state = 0},
  [35] = {.lex_state = 0},
  [36] = {.lex_state = 0},
  [37] = {.lex_state = 0},
  [38] = {.lex_state = 0},
  [39] = {.lex_state = 0},
  [40] = {.lex_state = 0},
  [41] = {.lex_state = 0},
  [42] = {.lex_state = 0},
  [43] = {.lex_state = 0},
  [44] = {.lex_state = 0},
  [45] = {.lex_state = 0},
  [46] = {.lex_state = 0},
  [47] = {.lex_state = 0},
  [48] = {.lex_state = 0},
  [49] = {.lex_state = 0},
  [50] = {.lex_state = 26},
  [51] = {.lex_state = 0},
  [52] = {.lex_state = 0},
  [53] = {.lex_state = 26},
};

static const uint16_t ts_parse_table[LARGE_STATE_COUNT][SYMBOL_COUNT] = {
  [0] = {
    [ts_builtin_sym_end] = ACTIONS(1),
    [anon_sym_POUNDinclude] = ACTIONS(1),
    [anon_sym_POUNDclear] = ACTIONS(1),
    [anon_sym_POUND] = ACTIONS(1),
    [anon_sym_SEMI] = ACTIONS(1),
    [anon_sym_LBRACE] = ACTIONS(1),
    [anon_sym_RBRACE] = ACTIONS(1),
    [anon_sym_COLON_COLON] = ACTIONS(1),
    [sym_identifier] = ACTIONS(1),
    [sym_string] = ACTIONS(1),
    [sym_line_comment] = ACTIONS(3),
    [sym_block_comment] = ACTIONS(3),
  },
  [1] = {
    [sym_source_file] = STATE(47),
    [sym__statement] = STATE(2),
    [sym_include_directive] = STATE(2),
    [sym_clear_directive] = STATE(2),
    [sym_unknown_statement] = STATE(2),
    [sym_key_value] = STATE(2),
    [sym_list_value] = STATE(2),
    [sym_scope] = STATE(2),
    [sym_path] = STATE(37),
    [aux_sym_source_file_repeat1] = STATE(2),
    [ts_builtin_sym_end] = ACTIONS(5),
    [anon_sym_POUNDinclude] = ACTIONS(7),
    [anon_sym_POUNDclear] = ACTIONS(9),
    [anon_sym_POUND] = ACTIONS(11),
    [sym_identifier] = ACTIONS(13),
    [sym_string] = ACTIONS(15),
    [sym_line_comment] = ACTIONS(3),
    [sym_block_comment] = ACTIONS(3),
  },
  [2] = {
    [sym__statement] = STATE(5),
    [sym_include_directive] = STATE(5),
    [sym_clear_directive] = STATE(5),
    [sym_unknown_statement] = STATE(5),
    [sym_key_value] = STATE(5),
    [sym_list_value] = STATE(5),
    [sym_scope] = STATE(5),
    [sym_path] = STATE(37),
    [aux_sym_source_file_repeat1] = STATE(5),
    [ts_builtin_sym_end] = ACTIONS(17),
    [anon_sym_POUNDinclude] = ACTIONS(7),
    [anon_sym_POUNDclear] = ACTIONS(9),
    [anon_sym_POUND] = ACTIONS(11),
    [sym_identifier] = ACTIONS(13),
    [sym_string] = ACTIONS(15),
    [sym_line_comment] = ACTIONS(3),
    [sym_block_comment] = ACTIONS(3),
  },
  [3] = {
    [sym__statement] = STATE(8),
    [sym_include_directive] = STATE(8),
    [sym_clear_directive] = STATE(8),
    [sym_unknown_statement] = STATE(8),
    [sym_key_value] = STATE(8),
    [sym_list_value] = STATE(8),
    [sym_scope] = STATE(8),
    [sym_path] = STATE(38),
    [aux_sym_source_file_repeat1] = STATE(8),
    [anon_sym_POUNDinclude] = ACTIONS(19),
    [anon_sym_POUNDclear] = ACTIONS(21),
    [anon_sym_POUND] = ACTIONS(23),
    [anon_sym_RBRACE] = ACTIONS(25),
    [sym_identifier] = ACTIONS(13),
    [sym_string] = ACTIONS(27),
    [sym_line_comment] = ACTIONS(3),
    [sym_block_comment] = ACTIONS(3),
  },
  [4] = {
    [sym__statement] = STATE(3),
    [sym_include_directive] = STATE(3),
    [sym_clear_directive] = STATE(3),
    [sym_unknown_statement] = STATE(3),
    [sym_key_value] = STATE(3),
    [sym_list_value] = STATE(3),
    [sym_scope] = STATE(3),
    [sym_path] = STATE(38),
    [aux_sym_source_file_repeat1] = STATE(3),
    [anon_sym_POUNDinclude] = ACTIONS(19),
    [anon_sym_POUNDclear] = ACTIONS(21),
    [anon_sym_POUND] = ACTIONS(23),
    [anon_sym_RBRACE] = ACTIONS(29),
    [sym_identifier] = ACTIONS(13),
    [sym_string] = ACTIONS(27),
    [sym_line_comment] = ACTIONS(3),
    [sym_block_comment] = ACTIONS(3),
  },
  [5] = {
    [sym__statement] = STATE(5),
    [sym_include_directive] = STATE(5),
    [sym_clear_directive] = STATE(5),
    [sym_unknown_statement] = STATE(5),
    [sym_key_value] = STATE(5),
    [sym_list_value] = STATE(5),
    [sym_scope] = STATE(5),
    [sym_path] = STATE(37),
    [aux_sym_source_file_repeat1] = STATE(5),
    [ts_builtin_sym_end] = ACTIONS(31),
    [anon_sym_POUNDinclude] = ACTIONS(33),
    [anon_sym_POUNDclear] = ACTIONS(36),
    [anon_sym_POUND] = ACTIONS(39),
    [sym_identifier] = ACTIONS(42),
    [sym_string] = ACTIONS(45),
    [sym_line_comment] = ACTIONS(3),
    [sym_block_comment] = ACTIONS(3),
  },
  [6] = {
    [sym__statement] = STATE(8),
    [sym_include_directive] = STATE(8),
    [sym_clear_directive] = STATE(8),
    [sym_unknown_statement] = STATE(8),
    [sym_key_value] = STATE(8),
    [sym_list_value] = STATE(8),
    [sym_scope] = STATE(8),
    [sym_path] = STATE(38),
    [aux_sym_source_file_repeat1] = STATE(8),
    [anon_sym_POUNDinclude] = ACTIONS(19),
    [anon_sym_POUNDclear] = ACTIONS(21),
    [anon_sym_POUND] = ACTIONS(23),
    [anon_sym_RBRACE] = ACTIONS(48),
    [sym_identifier] = ACTIONS(13),
    [sym_string] = ACTIONS(27),
    [sym_line_comment] = ACTIONS(3),
    [sym_block_comment] = ACTIONS(3),
  },
  [7] = {
    [sym__statement] = STATE(6),
    [sym_include_directive] = STATE(6),
    [sym_clear_directive] = STATE(6),
    [sym_unknown_statement] = STATE(6),
    [sym_key_value] = STATE(6),
    [sym_list_value] = STATE(6),
    [sym_scope] = STATE(6),
    [sym_path] = STATE(38),
    [aux_sym_source_file_repeat1] = STATE(6),
    [anon_sym_POUNDinclude] = ACTIONS(19),
    [anon_sym_POUNDclear] = ACTIONS(21),
    [anon_sym_POUND] = ACTIONS(23),
    [anon_sym_RBRACE] = ACTIONS(50),
    [sym_identifier] = ACTIONS(13),
    [sym_string] = ACTIONS(27),
    [sym_line_comment] = ACTIONS(3),
    [sym_block_comment] = ACTIONS(3),
  },
  [8] = {
    [sym__statement] = STATE(8),
    [sym_include_directive] = STATE(8),
    [sym_clear_directive] = STATE(8),
    [sym_unknown_statement] = STATE(8),
    [sym_key_value] = STATE(8),
    [sym_list_value] = STATE(8),
    [sym_scope] = STATE(8),
    [sym_path] = STATE(38),
    [aux_sym_source_file_repeat1] = STATE(8),
    [anon_sym_POUNDinclude] = ACTIONS(52),
    [anon_sym_POUNDclear] = ACTIONS(55),
    [anon_sym_POUND] = ACTIONS(58),
    [anon_sym_RBRACE] = ACTIONS(31),
    [sym_identifier] = ACTIONS(42),
    [sym_string] = ACTIONS(61),
    [sym_line_comment] = ACTIONS(3),
    [sym_block_comment] = ACTIONS(3),
  },
};

static const uint16_t ts_small_parse_table[] = {
  [0] = 5,
    ACTIONS(66), 1,
      anon_sym_POUND,
    ACTIONS(68), 1,
      anon_sym_COLON_COLON,
    STATE(10), 1,
      aux_sym_path_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(64), 7,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_SEMI,
      anon_sym_LBRACE,
      sym_identifier,
      sym_string,
  [23] = 5,
    ACTIONS(68), 1,
      anon_sym_COLON_COLON,
    ACTIONS(72), 1,
      anon_sym_POUND,
    STATE(11), 1,
      aux_sym_path_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(70), 7,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_SEMI,
      anon_sym_LBRACE,
      sym_identifier,
      sym_string,
  [46] = 5,
    ACTIONS(76), 1,
      anon_sym_POUND,
    ACTIONS(78), 1,
      anon_sym_COLON_COLON,
    STATE(11), 1,
      aux_sym_path_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(74), 7,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_SEMI,
      anon_sym_LBRACE,
      sym_identifier,
      sym_string,
  [69] = 3,
    ACTIONS(76), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(74), 8,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_SEMI,
      anon_sym_LBRACE,
      anon_sym_COLON_COLON,
      sym_identifier,
      sym_string,
  [87] = 5,
    ACTIONS(72), 1,
      anon_sym_POUND,
    ACTIONS(81), 1,
      anon_sym_COLON_COLON,
    STATE(15), 1,
      aux_sym_path_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(70), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [108] = 5,
    ACTIONS(66), 1,
      anon_sym_POUND,
    ACTIONS(81), 1,
      anon_sym_COLON_COLON,
    STATE(13), 1,
      aux_sym_path_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(64), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [129] = 5,
    ACTIONS(76), 1,
      anon_sym_POUND,
    ACTIONS(83), 1,
      anon_sym_COLON_COLON,
    STATE(15), 1,
      aux_sym_path_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(74), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [150] = 3,
    ACTIONS(76), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(74), 6,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      anon_sym_COLON_COLON,
      sym_identifier,
      sym_string,
  [166] = 4,
    ACTIONS(88), 1,
      anon_sym_POUND,
    ACTIONS(90), 1,
      anon_sym_SEMI,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(86), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [184] = 4,
    ACTIONS(88), 1,
      anon_sym_POUND,
    ACTIONS(92), 1,
      anon_sym_SEMI,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(86), 5,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      sym_identifier,
      sym_string,
  [202] = 4,
    ACTIONS(96), 1,
      anon_sym_POUND,
    ACTIONS(98), 1,
      anon_sym_SEMI,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(94), 5,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      sym_identifier,
      sym_string,
  [220] = 4,
    ACTIONS(96), 1,
      anon_sym_POUND,
    ACTIONS(100), 1,
      anon_sym_SEMI,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(94), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [238] = 3,
    ACTIONS(104), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(102), 5,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      sym_identifier,
      sym_string,
  [253] = 3,
    ACTIONS(108), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(106), 5,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      sym_identifier,
      sym_string,
  [268] = 3,
    ACTIONS(112), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(110), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [283] = 3,
    ACTIONS(88), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(86), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [298] = 3,
    ACTIONS(116), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(114), 5,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      sym_identifier,
      sym_string,
  [313] = 3,
    ACTIONS(116), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(114), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [328] = 3,
    ACTIONS(88), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(86), 5,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      sym_identifier,
      sym_string,
  [343] = 3,
    ACTIONS(120), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(118), 5,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      sym_identifier,
      sym_string,
  [358] = 3,
    ACTIONS(112), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(110), 5,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      sym_identifier,
      sym_string,
  [373] = 3,
    ACTIONS(124), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(122), 5,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      sym_identifier,
      sym_string,
  [388] = 3,
    ACTIONS(108), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(106), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [403] = 3,
    ACTIONS(128), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(126), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [418] = 3,
    ACTIONS(104), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(102), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [433] = 3,
    ACTIONS(128), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(126), 5,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      sym_identifier,
      sym_string,
  [448] = 3,
    ACTIONS(124), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(122), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [463] = 3,
    ACTIONS(120), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(118), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [478] = 5,
    ACTIONS(130), 1,
      anon_sym_SEMI,
    ACTIONS(132), 1,
      anon_sym_LBRACE,
    ACTIONS(134), 1,
      sym_string,
    STATE(40), 1,
      aux_sym_key_value_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [495] = 5,
    ACTIONS(134), 1,
      sym_string,
    ACTIONS(136), 1,
      anon_sym_SEMI,
    ACTIONS(138), 1,
      anon_sym_LBRACE,
    STATE(41), 1,
      aux_sym_key_value_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [512] = 4,
    ACTIONS(140), 1,
      anon_sym_SEMI,
    ACTIONS(142), 1,
      sym_string,
    STATE(39), 1,
      aux_sym_key_value_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [526] = 4,
    ACTIONS(134), 1,
      sym_string,
    ACTIONS(145), 1,
      anon_sym_SEMI,
    STATE(39), 1,
      aux_sym_key_value_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [540] = 4,
    ACTIONS(134), 1,
      sym_string,
    ACTIONS(147), 1,
      anon_sym_SEMI,
    STATE(39), 1,
      aux_sym_key_value_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [554] = 2,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(149), 2,
      anon_sym_SEMI,
      sym_string,
  [563] = 3,
    ACTIONS(151), 1,
      sym_identifier,
    STATE(32), 1,
      sym_path,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [574] = 3,
    ACTIONS(13), 1,
      sym_identifier,
    STATE(34), 1,
      sym_path,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [585] = 2,
    ACTIONS(153), 1,
      sym_identifier,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [593] = 2,
    ACTIONS(155), 1,
      sym_string,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [601] = 2,
    ACTIONS(157), 1,
      ts_builtin_sym_end,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [609] = 2,
    ACTIONS(159), 1,
      anon_sym_SEMI,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [617] = 2,
    ACTIONS(161), 1,
      sym_string,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [625] = 2,
    ACTIONS(163), 1,
      aux_sym_unknown_statement_token1,
    ACTIONS(165), 2,
      sym_line_comment,
      sym_block_comment,
  [633] = 2,
    ACTIONS(167), 1,
      anon_sym_SEMI,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [641] = 2,
    ACTIONS(169), 1,
      sym_identifier,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [649] = 2,
    ACTIONS(171), 1,
      aux_sym_unknown_statement_token1,
    ACTIONS(165), 2,
      sym_line_comment,
      sym_block_comment,
};

static const uint32_t ts_small_parse_table_map[] = {
  [SMALL_STATE(9)] = 0,
  [SMALL_STATE(10)] = 23,
  [SMALL_STATE(11)] = 46,
  [SMALL_STATE(12)] = 69,
  [SMALL_STATE(13)] = 87,
  [SMALL_STATE(14)] = 108,
  [SMALL_STATE(15)] = 129,
  [SMALL_STATE(16)] = 150,
  [SMALL_STATE(17)] = 166,
  [SMALL_STATE(18)] = 184,
  [SMALL_STATE(19)] = 202,
  [SMALL_STATE(20)] = 220,
  [SMALL_STATE(21)] = 238,
  [SMALL_STATE(22)] = 253,
  [SMALL_STATE(23)] = 268,
  [SMALL_STATE(24)] = 283,
  [SMALL_STATE(25)] = 298,
  [SMALL_STATE(26)] = 313,
  [SMALL_STATE(27)] = 328,
  [SMALL_STATE(28)] = 343,
  [SMALL_STATE(29)] = 358,
  [SMALL_STATE(30)] = 373,
  [SMALL_STATE(31)] = 388,
  [SMALL_STATE(32)] = 403,
  [SMALL_STATE(33)] = 418,
  [SMALL_STATE(34)] = 433,
  [SMALL_STATE(35)] = 448,
  [SMALL_STATE(36)] = 463,
  [SMALL_STATE(37)] = 478,
  [SMALL_STATE(38)] = 495,
  [SMALL_STATE(39)] = 512,
  [SMALL_STATE(40)] = 526,
  [SMALL_STATE(41)] = 540,
  [SMALL_STATE(42)] = 554,
  [SMALL_STATE(43)] = 563,
  [SMALL_STATE(44)] = 574,
  [SMALL_STATE(45)] = 585,
  [SMALL_STATE(46)] = 593,
  [SMALL_STATE(47)] = 601,
  [SMALL_STATE(48)] = 609,
  [SMALL_STATE(49)] = 617,
  [SMALL_STATE(50)] = 625,
  [SMALL_STATE(51)] = 633,
  [SMALL_STATE(52)] = 641,
  [SMALL_STATE(53)] = 649,
};

static const TSParseActionEntry ts_parse_actions[] = {
  [0] = {.entry = {.count = 0, .reusable = false}},
  [1] = {.entry = {.count = 1, .reusable = false}}, RECOVER(),
  [3] = {.entry = {.count = 1, .reusable = true}}, SHIFT_EXTRA(),
  [5] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 0, 0, 0),
  [7] = {.entry = {.count = 1, .reusable = true}}, SHIFT(46),
  [9] = {.entry = {.count = 1, .reusable = true}}, SHIFT(44),
  [11] = {.entry = {.count = 1, .reusable = false}}, SHIFT(53),
  [13] = {.entry = {.count = 1, .reusable = true}}, SHIFT(9),
  [15] = {.entry = {.count = 1, .reusable = true}}, SHIFT(48),
  [17] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 1, 0, 0),
  [19] = {.entry = {.count = 1, .reusable = true}}, SHIFT(49),
  [21] = {.entry = {.count = 1, .reusable = true}}, SHIFT(43),
  [23] = {.entry = {.count = 1, .reusable = false}}, SHIFT(50),
  [25] = {.entry = {.count = 1, .reusable = true}}, SHIFT(17),
  [27] = {.entry = {.count = 1, .reusable = true}}, SHIFT(51),
  [29] = {.entry = {.count = 1, .reusable = true}}, SHIFT(20),
  [31] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0),
  [33] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(46),
  [36] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(44),
  [39] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(53),
  [42] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(9),
  [45] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(48),
  [48] = {.entry = {.count = 1, .reusable = true}}, SHIFT(18),
  [50] = {.entry = {.count = 1, .reusable = true}}, SHIFT(19),
  [52] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(49),
  [55] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(43),
  [58] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(50),
  [61] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(51),
  [64] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_path, 1, 0, 0),
  [66] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_path, 1, 0, 0),
  [68] = {.entry = {.count = 1, .reusable = true}}, SHIFT(45),
  [70] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_path, 2, 0, 0),
  [72] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_path, 2, 0, 0),
  [74] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_path_repeat1, 2, 0, 0),
  [76] = {.entry = {.count = 1, .reusable = false}}, REDUCE(aux_sym_path_repeat1, 2, 0, 0),
  [78] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_path_repeat1, 2, 0, 0), SHIFT_REPEAT(45),
  [81] = {.entry = {.count = 1, .reusable = true}}, SHIFT(52),
  [83] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_path_repeat1, 2, 0, 0), SHIFT_REPEAT(52),
  [86] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_scope, 4, 0, 4),
  [88] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_scope, 4, 0, 4),
  [90] = {.entry = {.count = 1, .reusable = true}}, SHIFT(23),
  [92] = {.entry = {.count = 1, .reusable = true}}, SHIFT(29),
  [94] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_scope, 3, 0, 4),
  [96] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_scope, 3, 0, 4),
  [98] = {.entry = {.count = 1, .reusable = true}}, SHIFT(27),
  [100] = {.entry = {.count = 1, .reusable = true}}, SHIFT(24),
  [102] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_unknown_statement, 2, 0, 0),
  [104] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_unknown_statement, 2, 0, 0),
  [106] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_include_directive, 2, 0, 1),
  [108] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_include_directive, 2, 0, 1),
  [110] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_scope, 5, 0, 4),
  [112] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_scope, 5, 0, 4),
  [114] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_key_value, 3, 0, 5),
  [116] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_key_value, 3, 0, 5),
  [118] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_key_value, 2, 0, 4),
  [120] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_key_value, 2, 0, 4),
  [122] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_list_value, 2, 0, 3),
  [124] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_list_value, 2, 0, 3),
  [126] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_clear_directive, 2, 0, 2),
  [128] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_clear_directive, 2, 0, 2),
  [130] = {.entry = {.count = 1, .reusable = true}}, SHIFT(28),
  [132] = {.entry = {.count = 1, .reusable = true}}, SHIFT(7),
  [134] = {.entry = {.count = 1, .reusable = true}}, SHIFT(42),
  [136] = {.entry = {.count = 1, .reusable = true}}, SHIFT(36),
  [138] = {.entry = {.count = 1, .reusable = true}}, SHIFT(4),
  [140] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_key_value_repeat1, 2, 0, 6),
  [142] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_key_value_repeat1, 2, 0, 6), SHIFT_REPEAT(42),
  [145] = {.entry = {.count = 1, .reusable = true}}, SHIFT(25),
  [147] = {.entry = {.count = 1, .reusable = true}}, SHIFT(26),
  [149] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_key_value_repeat1, 1, 0, 3),
  [151] = {.entry = {.count = 1, .reusable = true}}, SHIFT(14),
  [153] = {.entry = {.count = 1, .reusable = true}}, SHIFT(12),
  [155] = {.entry = {.count = 1, .reusable = true}}, SHIFT(22),
  [157] = {.entry = {.count = 1, .reusable = true}},  ACCEPT_INPUT(),
  [159] = {.entry = {.count = 1, .reusable = true}}, SHIFT(30),
  [161] = {.entry = {.count = 1, .reusable = true}}, SHIFT(31),
  [163] = {.entry = {.count = 1, .reusable = false}}, SHIFT(33),
  [165] = {.entry = {.count = 1, .reusable = false}}, SHIFT_EXTRA(),
  [167] = {.entry = {.count = 1, .reusable = true}}, SHIFT(35),
  [169] = {.entry = {.count = 1, .reusable = true}}, SHIFT(16),
  [171] = {.entry = {.count = 1, .reusable = false}}, SHIFT(21),
};

#ifdef __cplusplus
extern "C" {
#endif
#ifdef TREE_SITTER_HIDE_SYMBOLS
#define TS_PUBLIC
#elif defined(_WIN32)
#define TS_PUBLIC __declspec(dllexport)
#else
#define TS_PUBLIC __attribute__((visibility("default")))
#endif

TS_PUBLIC const TSLanguage *tree_sitter_apt_config(void) {
  static const TSLanguage language = {
    .version = LANGUAGE_VERSION,
    .symbol_count = SYMBOL_COUNT,
    .alias_count = ALIAS_COUNT,
    .token_count = TOKEN_COUNT,
    .external_token_count = EXTERNAL_TOKEN_COUNT,
    .state_count = STATE_COUNT,
    .large_state_count = LARGE_STATE_COUNT,
    .production_id_count = PRODUCTION_ID_COUNT,
    .field_count = FIELD_COUNT,
    .max_alias_sequence_length = MAX_ALIAS_SEQUENCE_LENGTH,
    .parse_table = &ts_parse_table[0][0],
    .small_parse_table = ts_small_parse_table,
    .small_parse_table_map = ts_small_parse_table_map,
    .parse_actions = ts_parse_actions,
    .symbol_names = ts_symbol_names,
    .field_names = ts_field_names,
    .field_map_slices = ts_field_map_slices,
    .field_map_entries = ts_field_map_entries,
    .symbol_metadata = ts_symbol_metadata,
    .public_symbol_map = ts_symbol_map,
    .alias_map = ts_non_terminal_alias_map,
    .alias_sequences = &ts_alias_sequences[0][0],
    .lex_modes = ts_lex_modes,
    .lex_fn = ts_lex,
    .primary_state_ids = ts_primary_state_ids,
  };
  return &language;
}
#ifdef __cplusplus
}
#endif
