# Grammar

```lua
program
    ::= statement (";" statement)* ";"?

statement
    ::= import_statement
     | struct_definition
     | function_definition
     | if_statement
     | loop_statement
     | break_statement
     | continue_statement
     | return_statement
     | print_statement
     | println_statement
     | assert_statement
     | error_statement
     | assignment
     | reactive_assignment
     | immutable_assignment
     | expression

import_statement
    ::= "import" import_path

import_path
    ::= identifier ("." identifier)*

assignment
    ::= lvalue "=" expression

reactive_assignment
    ::= lvalue "::=" expression

immutable_assignment
    ::= identifier ":=" expression

lvalue
    ::= identifier
     | lvalue "[" expression "]"
     | lvalue "." identifier

struct_definition
    ::= "struct" identifier "{" field* "}"

field
    ::= identifier
     | identifier ("=" | ":=" | "::=") expression ";"?

function_definition
    ::= "func" identifier "(" params? ")" block

params
    ::= identifier ("," identifier)*

if_statement
    ::= "if" expression block ("else" (if_statement | block))?

loop_statement
    ::= "loop" block

break_statement
    ::= "break"

continue_statement
    ::= "continue"

return_statement
    ::= "return"
     | "return" expression

block
    ::= "{" statement (";" statement)* ";"? "}"

print_statement
    ::= "print" expression

println_statement
    ::= "println" expression

assert_statement
    ::= "assert" expression

error_statement
    ::= "error" string

expression
    ::= ternary

ternary
    ::= or_expr ("?" expression ":" expression)?

or_expr
    ::= and_expr ("||" and_expr)*

and_expr
    ::= comparison ("&&" comparison)*

comparison
    ::= additive ((">" | "<" | ">=" | "<=" | "==" | "!=") additive)*

additive
    ::= multiplicative (("+" | "-") multiplicative)*

multiplicative
    ::= unary (("*" | "/" | "%") unary)*

unary
    ::= "-" unary
     | "!" unary
     | cast
     | postfix

postfix
    ::= factor postfix_op*

postfix_op
    ::= "." identifier
     | "[" expression "]"
     | "(" arguments? ")"

arguments
    ::= expression ("," expression)*

factor
    ::= number
     | string
     | char
     | identifier
     | "struct" identifier
     | "(" expression ")"
     | "[" expression "]"

cast
    ::= "(" ("int" | "char") ")" factor

identifier
    ::= [a-zA-Z][a-zA-Z0-9_]*

number
    ::= [0-9]+

char
    ::= "'" character "'"

string
    ::= '"' character* '"'

character
    ::= escaped_char
     | any_char_except_quote_or_backslash

escaped_char
    ::= "\\" ("n" | "t" | "r" | "0" | "'" | '"' | "\\")

comment
    ::= "#" .* "#"
```
