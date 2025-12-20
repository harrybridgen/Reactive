# Simple Parser Language

This is a small expression-oriented language compiled to bytecode and executed on a stack-based virtual machine.

## Values and Types
- **Integers**: 32-bit signed integers
- **Arrays**: Fixed-size, zero-initialized arrays of integers
- **Lazy integers**: Expressions stored as ASTs and evaluated on access
- **Structs**: Heap-allocated records with named fields
- **Functions**: Callable units that may return integers, arrays, or structs

Arrays evaluate to their length when used as integers.

## Variables and Assignment
- `=` mutable assignment
- `:=` immutable assignment (cannot be reassigned)
- `::=` reactive (lazy) assignment, evaluated when read

Immutable variables are scoped and cannot be overwritten. Reactive variables re-evaluate their expression each time they are accessed.

## Arrays
- Created with `someArr = [size];`
- Indexed with `someArr[index] = 5;`
- Support mutable (`=`) and reactive (`::=`) element assignment
- Bounds are checked at runtime

## Structs

Structs define heap-allocated records with named fields.

### Struct Definition
```haskell
struct Counter {
    x = 0;
    step := 1;
    next ::= x + step;
}
```
### Field Kinds
- = mutable field
- := immutable field (cannot be modified)
- ::= reactive field (re-evaluated on access)

Reactive fields may depend on other fields in the same struct.

### Creating Struct Instances
```haskell
c = struct Counter;
```

### Field Access and Assignment
```haskell
println c.x;
c.x = 10;
println c.next;
```

## Functions

Functions encapsulate reusable logic and may return any value type.

### Function Definition
```haskell
func makecounter(start) {
    c = struct Counter;
    c.x = start;
    return c;
}
```
### Function Calls
```haskell
counter = makecounter(10);
println counter.x;
```

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
### Struct with Reactive Fields
```haskell
struct Counter {
    x = 0;
    step := 1;
    next ::= x + step;
}

c = struct Counter;

println c.next; # 1 #
c.x = 10;
println c.next; # 11 #
```
### Factorial via Dependency Graph
```haskell
fact = [6];   # we want factorials up to 5 #

fact[0] ::= 1;
fact[1] ::= 1;

x = 1;
dx ::= x + 1;

loop {
    if x >= fact - 1 {
        break;
    }

    i := x;
    fact[i + 1] ::= fact[i] * (i + 1);
    x = dx;
}

println fact[5];  # 120 #
```

### Functions Returning Structs
```haskell
func advance(c) {
    c.x = c.next;
    return c.x;
}

counter = makecounter(10);

println advance(counter); # 11 #
println counter.next;     # 12 #
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

### Simple Counting Loop
```haskell
x = 0;
dx ::= x + 1;

loop {
    println x;

    if x >= 4 {
        break;
    }

    x = dx; # advances x by +1 #
}
```
### Array Dependency Chain
```haskell
arr = [5];

base = 1;

# relation between current and previous index is +1 #
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

# base cases #
n0 = 0; 
n1 = 1;

# relation for base cases #
fib[0] ::= n0; 
fib[1] ::= n1;

# loop set up #
x = 0;
dx ::= x + 1;

# set up relations between array indexes #
loop {
    if x >= fib - 2 {
        break;
    }

    i := x;
    fib[i + 2] ::= fib[i] + fib[i + 1]; 
    x = dx;
}

# print the array #
x = 0;
loop{
    if x >= fib {
        break;
    }
    println fib[x]; 
    x = dx;
}

# change base values #
n0 = 89; 
n1 = 144;

# array prints differently due to relational indexes #
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
                 | struct_definition
                 | function_definition
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

struct_definition ::= "struct" identifier "{" field* "}"

field              ::= identifier ("=" | ":=" | "::=") expression ";"

function_definition ::= "func" identifier "(" params ")" block

params             ::= identifier ("," identifier)*

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
                 | "[" expression "]"

identifier     ::= [a-zA-Z][a-zA-Z0-9]*
number         ::= [0-9]+

comment        ::= "#" .* "#"

```
