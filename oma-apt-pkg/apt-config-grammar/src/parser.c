#include "tree_sitter/parser.h"

#if defined(__GNUC__) || defined(__clang__)
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#endif

#define LANGUAGE_VERSION 14
#define STATE_COUNT 58
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
  aux_sym_hash_comment_token1 = 4,
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
  sym_hash_comment = 17,
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
  [aux_sym_hash_comment_token1] = "hash_comment_token1",
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
  [sym_hash_comment] = "hash_comment",
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
  [aux_sym_hash_comment_token1] = aux_sym_hash_comment_token1,
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
  [sym_hash_comment] = sym_hash_comment,
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
  [aux_sym_hash_comment_token1] = {
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
  [sym_hash_comment] = {
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
  [6] = 2,
  [7] = 4,
  [8] = 3,
  [9] = 9,
  [10] = 10,
  [11] = 11,
  [12] = 12,
  [13] = 9,
  [14] = 11,
  [15] = 10,
  [16] = 16,
  [17] = 17,
  [18] = 12,
  [19] = 19,
  [20] = 20,
  [21] = 19,
  [22] = 20,
  [23] = 23,
  [24] = 16,
  [25] = 25,
  [26] = 26,
  [27] = 27,
  [28] = 28,
  [29] = 29,
  [30] = 28,
  [31] = 25,
  [32] = 27,
  [33] = 33,
  [34] = 29,
  [35] = 33,
  [36] = 36,
  [37] = 23,
  [38] = 26,
  [39] = 36,
  [40] = 17,
  [41] = 41,
  [42] = 41,
  [43] = 43,
  [44] = 44,
  [45] = 43,
  [46] = 46,
  [47] = 46,
  [48] = 48,
  [49] = 49,
  [50] = 50,
  [51] = 49,
  [52] = 52,
  [53] = 53,
  [54] = 50,
  [55] = 52,
  [56] = 53,
  [57] = 57,
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
      ACCEPT_TOKEN(aux_sym_hash_comment_token1);
      if (lookahead == '\n') ADVANCE(4);
      if (lookahead == '*') ADVANCE(24);
      if (lookahead == '/') ADVANCE(23);
      if (lookahead != 0) ADVANCE(23);
      END_STATE();
    case 23:
      ACCEPT_TOKEN(aux_sym_hash_comment_token1);
      if (lookahead == '\n') ADVANCE(4);
      if (lookahead == '*') ADVANCE(24);
      if (lookahead != 0) ADVANCE(23);
      END_STATE();
    case 24:
      ACCEPT_TOKEN(aux_sym_hash_comment_token1);
      if (lookahead == '\n') ADVANCE(4);
      if (lookahead == '*') ADVANCE(22);
      if (lookahead == '/') ADVANCE(27);
      if (lookahead != 0) ADVANCE(23);
      END_STATE();
    case 25:
      ACCEPT_TOKEN(aux_sym_hash_comment_token1);
      if (lookahead == '*') ADVANCE(23);
      if (lookahead == '/') ADVANCE(27);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(27);
      END_STATE();
    case 26:
      ACCEPT_TOKEN(aux_sym_hash_comment_token1);
      if (lookahead == '/') ADVANCE(25);
      if (lookahead == '\t' ||
          (0x0b <= lookahead && lookahead <= '\r') ||
          lookahead == ' ') ADVANCE(26);
      if (lookahead != 0 &&
          (lookahead < '\t' || '\r' < lookahead)) ADVANCE(27);
      END_STATE();
    case 27:
      ACCEPT_TOKEN(aux_sym_hash_comment_token1);
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
  [50] = {.lex_state = 0},
  [51] = {.lex_state = 0},
  [52] = {.lex_state = 26},
  [53] = {.lex_state = 0},
  [54] = {.lex_state = 0},
  [55] = {.lex_state = 26},
  [56] = {.lex_state = 0},
  [57] = {.lex_state = 0},
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
    [sym_source_file] = STATE(57),
    [sym__statement] = STATE(5),
    [sym_include_directive] = STATE(5),
    [sym_clear_directive] = STATE(5),
    [sym_hash_comment] = STATE(5),
    [sym_key_value] = STATE(5),
    [sym_list_value] = STATE(5),
    [sym_scope] = STATE(5),
    [sym_path] = STATE(41),
    [aux_sym_source_file_repeat1] = STATE(5),
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
    [sym__statement] = STATE(4),
    [sym_include_directive] = STATE(4),
    [sym_clear_directive] = STATE(4),
    [sym_hash_comment] = STATE(4),
    [sym_key_value] = STATE(4),
    [sym_list_value] = STATE(4),
    [sym_scope] = STATE(4),
    [sym_path] = STATE(42),
    [aux_sym_source_file_repeat1] = STATE(4),
    [anon_sym_POUNDinclude] = ACTIONS(17),
    [anon_sym_POUNDclear] = ACTIONS(19),
    [anon_sym_POUND] = ACTIONS(21),
    [anon_sym_RBRACE] = ACTIONS(23),
    [sym_identifier] = ACTIONS(13),
    [sym_string] = ACTIONS(25),
    [sym_line_comment] = ACTIONS(3),
    [sym_block_comment] = ACTIONS(3),
  },
  [3] = {
    [sym__statement] = STATE(2),
    [sym_include_directive] = STATE(2),
    [sym_clear_directive] = STATE(2),
    [sym_hash_comment] = STATE(2),
    [sym_key_value] = STATE(2),
    [sym_list_value] = STATE(2),
    [sym_scope] = STATE(2),
    [sym_path] = STATE(42),
    [aux_sym_source_file_repeat1] = STATE(2),
    [anon_sym_POUNDinclude] = ACTIONS(17),
    [anon_sym_POUNDclear] = ACTIONS(19),
    [anon_sym_POUND] = ACTIONS(21),
    [anon_sym_RBRACE] = ACTIONS(27),
    [sym_identifier] = ACTIONS(13),
    [sym_string] = ACTIONS(25),
    [sym_line_comment] = ACTIONS(3),
    [sym_block_comment] = ACTIONS(3),
  },
  [4] = {
    [sym__statement] = STATE(4),
    [sym_include_directive] = STATE(4),
    [sym_clear_directive] = STATE(4),
    [sym_hash_comment] = STATE(4),
    [sym_key_value] = STATE(4),
    [sym_list_value] = STATE(4),
    [sym_scope] = STATE(4),
    [sym_path] = STATE(42),
    [aux_sym_source_file_repeat1] = STATE(4),
    [anon_sym_POUNDinclude] = ACTIONS(29),
    [anon_sym_POUNDclear] = ACTIONS(32),
    [anon_sym_POUND] = ACTIONS(35),
    [anon_sym_RBRACE] = ACTIONS(38),
    [sym_identifier] = ACTIONS(40),
    [sym_string] = ACTIONS(43),
    [sym_line_comment] = ACTIONS(3),
    [sym_block_comment] = ACTIONS(3),
  },
  [5] = {
    [sym__statement] = STATE(7),
    [sym_include_directive] = STATE(7),
    [sym_clear_directive] = STATE(7),
    [sym_hash_comment] = STATE(7),
    [sym_key_value] = STATE(7),
    [sym_list_value] = STATE(7),
    [sym_scope] = STATE(7),
    [sym_path] = STATE(41),
    [aux_sym_source_file_repeat1] = STATE(7),
    [ts_builtin_sym_end] = ACTIONS(46),
    [anon_sym_POUNDinclude] = ACTIONS(7),
    [anon_sym_POUNDclear] = ACTIONS(9),
    [anon_sym_POUND] = ACTIONS(11),
    [sym_identifier] = ACTIONS(13),
    [sym_string] = ACTIONS(15),
    [sym_line_comment] = ACTIONS(3),
    [sym_block_comment] = ACTIONS(3),
  },
  [6] = {
    [sym__statement] = STATE(4),
    [sym_include_directive] = STATE(4),
    [sym_clear_directive] = STATE(4),
    [sym_hash_comment] = STATE(4),
    [sym_key_value] = STATE(4),
    [sym_list_value] = STATE(4),
    [sym_scope] = STATE(4),
    [sym_path] = STATE(42),
    [aux_sym_source_file_repeat1] = STATE(4),
    [anon_sym_POUNDinclude] = ACTIONS(17),
    [anon_sym_POUNDclear] = ACTIONS(19),
    [anon_sym_POUND] = ACTIONS(21),
    [anon_sym_RBRACE] = ACTIONS(48),
    [sym_identifier] = ACTIONS(13),
    [sym_string] = ACTIONS(25),
    [sym_line_comment] = ACTIONS(3),
    [sym_block_comment] = ACTIONS(3),
  },
  [7] = {
    [sym__statement] = STATE(7),
    [sym_include_directive] = STATE(7),
    [sym_clear_directive] = STATE(7),
    [sym_hash_comment] = STATE(7),
    [sym_key_value] = STATE(7),
    [sym_list_value] = STATE(7),
    [sym_scope] = STATE(7),
    [sym_path] = STATE(41),
    [aux_sym_source_file_repeat1] = STATE(7),
    [ts_builtin_sym_end] = ACTIONS(38),
    [anon_sym_POUNDinclude] = ACTIONS(50),
    [anon_sym_POUNDclear] = ACTIONS(53),
    [anon_sym_POUND] = ACTIONS(56),
    [sym_identifier] = ACTIONS(40),
    [sym_string] = ACTIONS(59),
    [sym_line_comment] = ACTIONS(3),
    [sym_block_comment] = ACTIONS(3),
  },
  [8] = {
    [sym__statement] = STATE(6),
    [sym_include_directive] = STATE(6),
    [sym_clear_directive] = STATE(6),
    [sym_hash_comment] = STATE(6),
    [sym_key_value] = STATE(6),
    [sym_list_value] = STATE(6),
    [sym_scope] = STATE(6),
    [sym_path] = STATE(42),
    [aux_sym_source_file_repeat1] = STATE(6),
    [anon_sym_POUNDinclude] = ACTIONS(17),
    [anon_sym_POUNDclear] = ACTIONS(19),
    [anon_sym_POUND] = ACTIONS(21),
    [anon_sym_RBRACE] = ACTIONS(62),
    [sym_identifier] = ACTIONS(13),
    [sym_string] = ACTIONS(25),
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
    STATE(9), 1,
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
    ACTIONS(73), 1,
      anon_sym_POUND,
    ACTIONS(75), 1,
      anon_sym_COLON_COLON,
    STATE(11), 1,
      aux_sym_path_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(71), 7,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_SEMI,
      anon_sym_LBRACE,
      sym_identifier,
      sym_string,
  [46] = 5,
    ACTIONS(79), 1,
      anon_sym_POUND,
    ACTIONS(81), 1,
      anon_sym_COLON_COLON,
    STATE(9), 1,
      aux_sym_path_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(77), 7,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_SEMI,
      anon_sym_LBRACE,
      sym_identifier,
      sym_string,
  [69] = 3,
    ACTIONS(66), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(64), 8,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_SEMI,
      anon_sym_LBRACE,
      anon_sym_COLON_COLON,
      sym_identifier,
      sym_string,
  [87] = 5,
    ACTIONS(66), 1,
      anon_sym_POUND,
    ACTIONS(83), 1,
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
  [108] = 5,
    ACTIONS(79), 1,
      anon_sym_POUND,
    ACTIONS(86), 1,
      anon_sym_COLON_COLON,
    STATE(13), 1,
      aux_sym_path_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(77), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [129] = 5,
    ACTIONS(73), 1,
      anon_sym_POUND,
    ACTIONS(88), 1,
      anon_sym_COLON_COLON,
    STATE(14), 1,
      aux_sym_path_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(71), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [150] = 4,
    ACTIONS(79), 1,
      anon_sym_POUND,
    ACTIONS(90), 1,
      sym_identifier,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(77), 6,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_SEMI,
      anon_sym_LBRACE,
      sym_string,
  [169] = 4,
    ACTIONS(90), 1,
      sym_identifier,
    ACTIONS(94), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(92), 6,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_SEMI,
      anon_sym_LBRACE,
      sym_string,
  [188] = 3,
    ACTIONS(66), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(64), 6,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      anon_sym_COLON_COLON,
      sym_identifier,
      sym_string,
  [204] = 4,
    ACTIONS(98), 1,
      anon_sym_POUND,
    ACTIONS(100), 1,
      anon_sym_SEMI,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(96), 5,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      sym_identifier,
      sym_string,
  [222] = 4,
    ACTIONS(104), 1,
      anon_sym_POUND,
    ACTIONS(106), 1,
      anon_sym_SEMI,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(102), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [240] = 4,
    ACTIONS(98), 1,
      anon_sym_POUND,
    ACTIONS(108), 1,
      anon_sym_SEMI,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(96), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [258] = 4,
    ACTIONS(104), 1,
      anon_sym_POUND,
    ACTIONS(110), 1,
      anon_sym_SEMI,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(102), 5,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      sym_identifier,
      sym_string,
  [276] = 3,
    ACTIONS(114), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(112), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [291] = 4,
    ACTIONS(79), 1,
      anon_sym_POUND,
    ACTIONS(116), 1,
      sym_identifier,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(77), 4,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_string,
  [308] = 3,
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
  [323] = 3,
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
  [338] = 3,
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
  [353] = 3,
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
  [368] = 3,
    ACTIONS(132), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(130), 5,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      sym_identifier,
      sym_string,
  [383] = 3,
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
  [398] = 3,
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
  [413] = 3,
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
  [428] = 3,
    ACTIONS(136), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(134), 5,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      sym_identifier,
      sym_string,
  [443] = 3,
    ACTIONS(132), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(130), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [458] = 3,
    ACTIONS(136), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(134), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [473] = 3,
    ACTIONS(140), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(138), 5,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      sym_identifier,
      sym_string,
  [488] = 3,
    ACTIONS(114), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(112), 5,
      ts_builtin_sym_end,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      sym_identifier,
      sym_string,
  [503] = 3,
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
  [518] = 3,
    ACTIONS(140), 1,
      anon_sym_POUND,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(138), 5,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_identifier,
      sym_string,
  [533] = 4,
    ACTIONS(94), 1,
      anon_sym_POUND,
    ACTIONS(116), 1,
      sym_identifier,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(92), 4,
      anon_sym_POUNDinclude,
      anon_sym_POUNDclear,
      anon_sym_RBRACE,
      sym_string,
  [550] = 5,
    ACTIONS(142), 1,
      anon_sym_SEMI,
    ACTIONS(144), 1,
      anon_sym_LBRACE,
    ACTIONS(146), 1,
      sym_string,
    STATE(45), 1,
      aux_sym_key_value_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [567] = 5,
    ACTIONS(146), 1,
      sym_string,
    ACTIONS(148), 1,
      anon_sym_SEMI,
    ACTIONS(150), 1,
      anon_sym_LBRACE,
    STATE(43), 1,
      aux_sym_key_value_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [584] = 4,
    ACTIONS(146), 1,
      sym_string,
    ACTIONS(152), 1,
      anon_sym_SEMI,
    STATE(44), 1,
      aux_sym_key_value_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [598] = 4,
    ACTIONS(154), 1,
      anon_sym_SEMI,
    ACTIONS(156), 1,
      sym_string,
    STATE(44), 1,
      aux_sym_key_value_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [612] = 4,
    ACTIONS(146), 1,
      sym_string,
    ACTIONS(159), 1,
      anon_sym_SEMI,
    STATE(44), 1,
      aux_sym_key_value_repeat1,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [626] = 3,
    ACTIONS(161), 1,
      sym_identifier,
    STATE(34), 1,
      sym_path,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [637] = 3,
    ACTIONS(13), 1,
      sym_identifier,
    STATE(29), 1,
      sym_path,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [648] = 2,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
    ACTIONS(163), 2,
      anon_sym_SEMI,
      sym_string,
  [657] = 2,
    ACTIONS(90), 1,
      sym_identifier,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [665] = 2,
    ACTIONS(165), 1,
      sym_string,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [673] = 2,
    ACTIONS(116), 1,
      sym_identifier,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [681] = 2,
    ACTIONS(167), 1,
      aux_sym_hash_comment_token1,
    ACTIONS(169), 2,
      sym_line_comment,
      sym_block_comment,
  [689] = 2,
    ACTIONS(171), 1,
      anon_sym_SEMI,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [697] = 2,
    ACTIONS(173), 1,
      sym_string,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [705] = 2,
    ACTIONS(175), 1,
      aux_sym_hash_comment_token1,
    ACTIONS(169), 2,
      sym_line_comment,
      sym_block_comment,
  [713] = 2,
    ACTIONS(177), 1,
      anon_sym_SEMI,
    ACTIONS(3), 2,
      sym_line_comment,
      sym_block_comment,
  [721] = 2,
    ACTIONS(179), 1,
      ts_builtin_sym_end,
    ACTIONS(3), 2,
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
  [SMALL_STATE(17)] = 169,
  [SMALL_STATE(18)] = 188,
  [SMALL_STATE(19)] = 204,
  [SMALL_STATE(20)] = 222,
  [SMALL_STATE(21)] = 240,
  [SMALL_STATE(22)] = 258,
  [SMALL_STATE(23)] = 276,
  [SMALL_STATE(24)] = 291,
  [SMALL_STATE(25)] = 308,
  [SMALL_STATE(26)] = 323,
  [SMALL_STATE(27)] = 338,
  [SMALL_STATE(28)] = 353,
  [SMALL_STATE(29)] = 368,
  [SMALL_STATE(30)] = 383,
  [SMALL_STATE(31)] = 398,
  [SMALL_STATE(32)] = 413,
  [SMALL_STATE(33)] = 428,
  [SMALL_STATE(34)] = 443,
  [SMALL_STATE(35)] = 458,
  [SMALL_STATE(36)] = 473,
  [SMALL_STATE(37)] = 488,
  [SMALL_STATE(38)] = 503,
  [SMALL_STATE(39)] = 518,
  [SMALL_STATE(40)] = 533,
  [SMALL_STATE(41)] = 550,
  [SMALL_STATE(42)] = 567,
  [SMALL_STATE(43)] = 584,
  [SMALL_STATE(44)] = 598,
  [SMALL_STATE(45)] = 612,
  [SMALL_STATE(46)] = 626,
  [SMALL_STATE(47)] = 637,
  [SMALL_STATE(48)] = 648,
  [SMALL_STATE(49)] = 657,
  [SMALL_STATE(50)] = 665,
  [SMALL_STATE(51)] = 673,
  [SMALL_STATE(52)] = 681,
  [SMALL_STATE(53)] = 689,
  [SMALL_STATE(54)] = 697,
  [SMALL_STATE(55)] = 705,
  [SMALL_STATE(56)] = 713,
  [SMALL_STATE(57)] = 721,
};

static const TSParseActionEntry ts_parse_actions[] = {
  [0] = {.entry = {.count = 0, .reusable = false}},
  [1] = {.entry = {.count = 1, .reusable = false}}, RECOVER(),
  [3] = {.entry = {.count = 1, .reusable = true}}, SHIFT_EXTRA(),
  [5] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 0, 0, 0),
  [7] = {.entry = {.count = 1, .reusable = true}}, SHIFT(54),
  [9] = {.entry = {.count = 1, .reusable = true}}, SHIFT(47),
  [11] = {.entry = {.count = 1, .reusable = false}}, SHIFT(55),
  [13] = {.entry = {.count = 1, .reusable = true}}, SHIFT(10),
  [15] = {.entry = {.count = 1, .reusable = true}}, SHIFT(56),
  [17] = {.entry = {.count = 1, .reusable = true}}, SHIFT(50),
  [19] = {.entry = {.count = 1, .reusable = true}}, SHIFT(46),
  [21] = {.entry = {.count = 1, .reusable = false}}, SHIFT(52),
  [23] = {.entry = {.count = 1, .reusable = true}}, SHIFT(20),
  [25] = {.entry = {.count = 1, .reusable = true}}, SHIFT(53),
  [27] = {.entry = {.count = 1, .reusable = true}}, SHIFT(21),
  [29] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(50),
  [32] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(46),
  [35] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(52),
  [38] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0),
  [40] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(10),
  [43] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(53),
  [46] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 1, 0, 0),
  [48] = {.entry = {.count = 1, .reusable = true}}, SHIFT(22),
  [50] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(54),
  [53] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(47),
  [56] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(55),
  [59] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2, 0, 0), SHIFT_REPEAT(56),
  [62] = {.entry = {.count = 1, .reusable = true}}, SHIFT(19),
  [64] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_path_repeat1, 2, 0, 0),
  [66] = {.entry = {.count = 1, .reusable = false}}, REDUCE(aux_sym_path_repeat1, 2, 0, 0),
  [68] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_path_repeat1, 2, 0, 0), SHIFT_REPEAT(49),
  [71] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_path, 1, 0, 0),
  [73] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_path, 1, 0, 0),
  [75] = {.entry = {.count = 1, .reusable = true}}, SHIFT(16),
  [77] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_path, 2, 0, 0),
  [79] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_path, 2, 0, 0),
  [81] = {.entry = {.count = 1, .reusable = true}}, SHIFT(17),
  [83] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_path_repeat1, 2, 0, 0), SHIFT_REPEAT(51),
  [86] = {.entry = {.count = 1, .reusable = true}}, SHIFT(40),
  [88] = {.entry = {.count = 1, .reusable = true}}, SHIFT(24),
  [90] = {.entry = {.count = 1, .reusable = true}}, SHIFT(12),
  [92] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_path, 3, 0, 0),
  [94] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_path, 3, 0, 0),
  [96] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_scope, 3, 0, 4),
  [98] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_scope, 3, 0, 4),
  [100] = {.entry = {.count = 1, .reusable = true}}, SHIFT(30),
  [102] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_scope, 4, 0, 4),
  [104] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_scope, 4, 0, 4),
  [106] = {.entry = {.count = 1, .reusable = true}}, SHIFT(25),
  [108] = {.entry = {.count = 1, .reusable = true}}, SHIFT(28),
  [110] = {.entry = {.count = 1, .reusable = true}}, SHIFT(31),
  [112] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_include_directive, 2, 0, 1),
  [114] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_include_directive, 2, 0, 1),
  [116] = {.entry = {.count = 1, .reusable = true}}, SHIFT(18),
  [118] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_scope, 5, 0, 4),
  [120] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_scope, 5, 0, 4),
  [122] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_list_value, 2, 0, 3),
  [124] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_list_value, 2, 0, 3),
  [126] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_key_value, 3, 0, 5),
  [128] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_key_value, 3, 0, 5),
  [130] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_clear_directive, 2, 0, 2),
  [132] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_clear_directive, 2, 0, 2),
  [134] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_hash_comment, 2, 0, 0),
  [136] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_hash_comment, 2, 0, 0),
  [138] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_key_value, 2, 0, 4),
  [140] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_key_value, 2, 0, 4),
  [142] = {.entry = {.count = 1, .reusable = true}}, SHIFT(36),
  [144] = {.entry = {.count = 1, .reusable = true}}, SHIFT(8),
  [146] = {.entry = {.count = 1, .reusable = true}}, SHIFT(48),
  [148] = {.entry = {.count = 1, .reusable = true}}, SHIFT(39),
  [150] = {.entry = {.count = 1, .reusable = true}}, SHIFT(3),
  [152] = {.entry = {.count = 1, .reusable = true}}, SHIFT(32),
  [154] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_key_value_repeat1, 2, 0, 6),
  [156] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_key_value_repeat1, 2, 0, 6), SHIFT_REPEAT(48),
  [159] = {.entry = {.count = 1, .reusable = true}}, SHIFT(27),
  [161] = {.entry = {.count = 1, .reusable = true}}, SHIFT(15),
  [163] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_key_value_repeat1, 1, 0, 3),
  [165] = {.entry = {.count = 1, .reusable = true}}, SHIFT(23),
  [167] = {.entry = {.count = 1, .reusable = false}}, SHIFT(35),
  [169] = {.entry = {.count = 1, .reusable = false}}, SHIFT_EXTRA(),
  [171] = {.entry = {.count = 1, .reusable = true}}, SHIFT(38),
  [173] = {.entry = {.count = 1, .reusable = true}}, SHIFT(37),
  [175] = {.entry = {.count = 1, .reusable = false}}, SHIFT(33),
  [177] = {.entry = {.count = 1, .reusable = true}}, SHIFT(26),
  [179] = {.entry = {.count = 1, .reusable = true}},  ACCEPT_INPUT(),
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
