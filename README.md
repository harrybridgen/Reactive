# Simple Parser Language

This is a small expression-oriented language compiled to bytecode and executed on a stack-based virtual machine.

## Values and Types
- **Integers**: 32-bit signed integers
- **Arrays**: Fixed-size, zero-initialized arrays of integers
- **Lazy integers**: Expressions stored as ASTs and evaluated on access

Arrays evaluate to their length when used as integers.

## Variables and Assignment
- `=` mutable assignment
- `:=` immutable assignment (cannot be reassigned)
- `::=` reactive (lazy) assignment, evaluated when read

Immutable variables are scoped and cannot be overwritten. Reactive variables re-evaluate their expression each time they are accessed.

## Arrays
- Created with `someArr = [size];`
- Indexed with `array[index] = 5;`
- Support mutable (`=`) and reactive (`::=`) element assignment
- Bounds are checked at runtime

## Expressions
- Arithmetic: `+ - * /`
- Comparison: `> < >= <= == !=`
- Logic: `&& ||`
- No boolean type: `0` is false, non-zero is true

## Control Flow
- `if { } else { }` conditional execution
- `loop { }` infinite loop
- `break` exits the nearest loop

Each loop iteration creates a fresh immutable scope.

## Scoping Rules
- Mutable variables are global
- Immutable variables are block-scoped
- Immutable scopes are cleared on each loop iteration
- Inner immutable bindings shadow outer ones

## Examples

### Reactive variables
```haskell
x = 1;
y ::= x + 1;

println y;   # 2 #
x = 10;
println y;   # 11 #
```
### Immutable bindings
```haskell
a := 5;
a = 6;     # runtime error #
println a;
```

### Arrays and lazy elements
```haskell
arr = [5];
x = 2;

arr[0] ::= x * 10;
println arr[0];  # 20  #

x = 7;
println arr[0];  # 70 #
```

### Array Dependency Chain
```haskell
arr = [5];

base = 1;

arr[0] ::= base;
arr[1] ::= arr[0] + 1;
arr[2] ::= arr[1] + 1;
arr[3] ::= arr[2] + 1;
arr[4] ::= arr[3] + 1;

println arr[4];   # 5 #

base = 10;

println arr[4];   # 14 #
```

### Fibonacci-style dependency graph
```haskell
# allocate array #
fib = [10]; 

# set up base cases #
n0 = 0; 
n1 = 1;
fib[0] ::= n0;
fib[1] ::= n1;

## init loop
x = 0;
dx ::= x + 1;

# construct relational array #
loop {
    if x >= fib - 2 {
        break;
    }

    i := x;
    fib[i + 2] ::= fib[i] + fib[i + 1];
    x = dx;
}

# print relational array #
x = 0;
loop{
    if x >= fib {
        break;
    }
    println fib[x];
    x = dx;
}

## ! change the base values ! #
n0 = 89;
n1 = 144;

## print relational array (different than before)
x = 0;
loop{
    if x >= fib {
        break;
    }
    println fib[x];
    x = dx;
}

```

## Grammar
```haskell
program        ::= statement (";" statement)* ";"?

statement      ::= assignment
                 | array_assignment
                 | reactive_assignment
                 | immutable_assignment
                 | if_statement
                 | loop_statement
                 | break_statement
                 | print_statement
                 | println_statement
                 | expression

assignment     ::= identifier "=" expression

reactive_assignment
                ::= identifier "::=" expression

immutable_assignment
                ::= identifier ":=" expression

array_assignment
                ::= identifier "[" expression "]" "=" expression
                 | identifier "[" expression "]" "::=" expression

if_statement   ::= "if" expression block ("else" block)?

loop_statement ::= "loop" block

break_statement
                ::= "break"

block          ::= "{" statement (";" statement)* ";"? "}"

print_statement
                ::= "print" expression

println_statement
                ::= "println" expression

expression     ::= or_expr

or_expr        ::= and_expr ("||" and_expr)*

and_expr       ::= comparison ("&&" comparison)*

comparison     ::= additive ((">" | "<" | ">=" | "<=" | "==" | "!=") additive)*

additive       ::= multiplicative (("+" | "-") multiplicative)*

multiplicative ::= postfix (("*" | "/") postfix)*

postfix        ::= factor ("[" expression "]")*

factor         ::= number
                 | identifier
                 | "-" factor
                 | "(" expression ")"
                 | "[" expression "]"     // array creation

identifier     ::= [a-zA-Z][a-zA-Z0-9]*
number         ::= [0-9]+

comment        ::= "#" .* "#"
```
