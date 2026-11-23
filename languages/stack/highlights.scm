(comment) @comment
(identifier) @variable
(number) @number
(string) @string
(boolean) @boolean
(null) @constant
(function_declaration name: (qualified_name) @function)
(method_declaration name: (identifier) @function.method)
(class_declaration name: (identifier) @type)
(call_expression function: (identifier) @function)
(member_expression property: (identifier) @property)
["(" ")" "[" "]" "{" "}"] @punctuation.bracket
[";" "," "."] @punctuation.delimiter
(pair ":" @punctuation.delimiter)
(switch_clause ":" @punctuation.delimiter)

(keyword) @keyword
[(date) (time)] @number

[
  "+" "-" "*" "/" "%"
  "=" "+=" "-=" "*=" "/=" "%="
  "==" "!=" "<" ">" "<=" ">="
  "&&" "||" "!" "&" "|"
  "++" "--" "?" "..."
] @operator

(conditional_expression ":" @operator)
(binary_expression operator: (keyword) @operator)
(unary_expression (keyword) @operator)
