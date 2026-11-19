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
[";" "," "." ":"] @punctuation.delimiter
(binary_expression operator: (_) @operator)

(keyword) @keyword
[(date) (time)] @number
["+" "-" "*" "/" "%" "=" "+=" "-=" "*=" "/=" "%=" "==" "!=" "<" ">" "<=" ">=" "&&" "||" "!" "++" "--" "?" "&" "|" "^" "~" "<<" ">>" "===" "!=="] @operator
