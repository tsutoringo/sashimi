; Sashimi currently uses tree-sitter-rust as a bootstrap grammar because the
; language intentionally shares Rust-like declaration syntax. These queries add
; Sashimi-specific identifiers on top of the Rust grammar.

(type_identifier) @type
(primitive_type) @type.builtin
(field_identifier) @property

(function_item (identifier) @function)
(function_signature_item (identifier) @function)
(call_expression function: (identifier) @function)
(call_expression
  function: (field_expression
    field: (field_identifier) @function.method))

(parameter (identifier) @variable.parameter)

(line_comment) @comment
(block_comment) @comment

(char_literal) @string
(string_literal) @string
(raw_string_literal) @string
(escape_sequence) @string.escape

(boolean_literal) @boolean
(integer_literal) @number
(float_literal) @number

[
  "fn"
  "for"
  "impl"
  "let"
  "pub"
  "return"
  "trait"
] @keyword

((identifier) @keyword
 (#eq? @keyword "class"))
((identifier) @keyword
 (#eq? @keyword "new"))

((identifier) @type.builtin
 (#eq? @type.builtin "number"))
((identifier) @type.builtin
 (#eq? @type.builtin "string"))
((identifier) @type.builtin
 (#eq? @type.builtin "boolean"))
((identifier) @type.builtin
 (#eq? @type.builtin "unknown"))

((identifier) @type
 (#match? @type "^(Array|Map|Set|Iterator|SashimiIterator)$"))

(self) @variable.special

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  ":"
  "."
  ","
  ";"
  "::"
] @punctuation.delimiter

[
  "*"
  "&"
] @operator
