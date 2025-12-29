# Language Guide

This is a tutorial-style guide to the Reactive language, covering values, control flow, reactivity, and common patterns.

## Values and Types

- Integers: 32-bit signed integers
- Characters: Unicode scalar values ('A', 'b', '\n')
- Strings: Mutable arrays of characters ("HELLO")
- Arrays: Fixed-size, zero-initialized arrays of values (integers, characters, structs, or arrays)
- Lazy values: Expressions stored as ASTs and evaluated on access
- Structs: Heap-allocated records with named fields
- Functions: Callable units that may return integers, characters, arrays, or structs

Arrays (including strings) evaluate to their length when used as integers.

## Expressions

- Arithmetic: `+ - * /`
- Modulo `%`
- Comparison: `> < >= <= == !=`
- Logic: `&& || !`
- No boolean type: `0` is false, non-zero is true
- Casts: `(int) expr`, `(char) expr`
- Ternary `x ? y : z;`

## Control Flow

- Program starts in the `main` function.
- `if { } else if { } else { }` conditional execution
- `return x;` returns a value from a function
- `loop { }` infinite loop
- `break` exits the nearest loop
- `continue` skip to the next iteration of loop

Each loop iteration creates a fresh immutable `:=` scope, while mutable and reactive locations persist.

## Variables and Assignment

The language has three assignment forms, each with a distinct meaning.

### `=` Mutable Assignment

`=` creates or mutates a mutable location.

```lua
func main(){
    x = 10;
    println x; # 10 #
}
```

Mutable variables created with `=` are local to the current function invocation, unless they refer to an existing global or heap location.

```lua
func foo(x, y){
    z = x + y;
    return z;
}

func main(){
    x = 1;
    y = 2;
    z = foo(x, y);
    println  z; # 3 #
}
```

Inside structs, `=` creates per-instance mutable fields.

```lua
struct A {
    x = 0;
}

func main(){
    a = struct A;
    b = struct A;

    a.x = 5;
    println b.x; # 0 #
}
```

Struct fields are not shared between instances.

When used inside arrays, `=` assigns the location of the index in the array to a value.

```lua
func main(){
    arr = [2]
    arr[0] = 1;

    println arr[0]; # 1 #
}
```

### `::=` Reactive Assignment (relationships)

`::=` defines a relationship between locations. It stores an expression and its dependencies, not a value.

```lua
func main(){
    x = 1;
    y ::= x + 1;

    println y; # 2 #
}
```

The expression is evaluated when read. If any dependency changes, the result updates automatically.

```lua
func main(){
    x = 1;
    y ::= x + 1;

    println  y; # 2 #

    x = y;

    print  y; # 3 #
}
```

Reactive assignments:

- capture dependencies, not snapshots
- are lazy evaluated
- attach to the location, not the name

They are commonly used to build progression variables in loops:

```lua
func main(){
    x = 0;
    dx ::= x + 1;

    loop {
        println x;
        if x >= 4 { break; }
        x = dx;
    }
}
```

Reactive assignments work uniformly for variables, struct fields, and array elements.

```lua
struct Counter {
    x = 1;
    step = 1;
    next;
}

func main(){
    c = struct Counter;
    c.next ::= c.x + c.step;

    println c.next; # 2 #
    c.x = c.next;
    println c.next; # 3 #
}
```

Reactive assignments may use ternary expressions on the right-hand side.

```lua
func main(){
    arr =[2]
    arr[1] ::= arr[0] +  2;
    x ::= arr[1] >  1 ? 10 : 20;

    println arr[0]; # 0 #
    print x; # 10 #
}
```

### `:=` Immutable Binding (capture / identity)

`:=` is value capture, not assignment. It does not create a location or participate in the reactive graph.

That name:

- is immutable
- is not reactive
- disappears when the scope ends
- cannot be reassigned
- cannot be observed reactively

If the `:=` is binding an array or struct, the contents are mutable.

#### Why `:=` exists

Reactive bindings `::=` store relationships, not values. This means:

```lua
arr[i] ::= arr[i - 1] + 1;
```

does not mean: use the current value of `i`. It means: use whatever `i` refers to when this expression is evaluated.

So if `i` keeps changing, the dependency graph becomes self-referential or incorrect.

#### The problem (without `:=`)

```lua
func main(){
    arr = [3];
    i = 0;

    loop {
        arr[i] ::= i * 10;
        i = i + 1;
        if i >=  3 { break; }
    }

    println  arr[0];
    println  arr[1];
    println  arr[2];
}
```

Becomes:

```
arr[0] = 30
arr[1] = 30
arr[2] = 30
```

and not:

```
arr[0] = 0
arr[1] = 10
arr[2] = 20
```

To fix this, capture `i` with `:=`:

```lua
func main(){
    arr = [3];
    i = 0;

    loop {
        j := i;
        arr[j] ::= j * 10;
        i = i + 1;
        if i >= 3 { break; }
    }
}
```

## Characters and Strings

### Characters

Character literals use single quotes:

```lua
func main(){
    c = 'A';
    println c;   # A #
}
```

Characters coerce to integers in numeric contexts. Casting is explicit:

```haskell
func main(){
    n := (int)'A';
    c := (char)(n + 1);

    println n;  # 65 #
    println c;  # B #
}
```

### Strings

Strings use double quotes and are compiled as arrays of characters:

```lua
func main(){
    s := "HELLO";
    println s;      # HELLO #
    println s[1];   # E #
    println (int) s;    # 5 #
}
```

Strings are:

- indexable
- mutable
- usable anywhere arrays are allowed

```lua
func main(){
    s = "HELLO";
    s[0] = 'X';
    println s;   # XELLO #
}
```

### Reactivity with Text

```lua
func main(){
    text := "HELLO";

    i = 0;
    di ::= i + 1;

    c ::= text[i];

    println c;   # H #
    i = di;
    println c;   # E #
}
```

### Strings in Structs and Functions

```lua
func main(){
    struct Label {
        text;
    }

    l = struct Label;
    l.text = "OK";
    l.text[1] = '!';
    println l.text;  # O! #
}
```

Returned strings are shared by reference:

```lua
func make() {
    return "HI";
}

func main(){
    a = make();
    b = a;
    b[0] = 'X';

    println a;  # XI #
}
```

### Printing Strings

- print / println automatically detect strings and characters
- strings print as text, not arrays
- characters print as characters, not numbers

```lua
func main(){
    println 'A';      # A #
    println "ABC";    # ABC #
    println (char)("A"[0]+1); # B #
}
```

## Structs

Structs define heap-allocated records with named fields.

### Field Kinds

- `=` mutable field
- `:=` immutable bind
- `::=` reactive field

Reactive fields may depend on other fields in the same struct.

### Creating Struct Instances

```lua
struct Counter {
    x = 0;
    step := 1;
    next ::= x + step;
    foo;
}

func main(){
    c = struct Counter;
    println c.next; # 1 #
    c.x = 10;
    println c.next; # 11 #
}
```

### Closed Structs

Fields in a struct must be declared in the struct definition.

```lua
struct Empty {}

func main(){
    e := struct Empty;
    e.foo = 1; # Error #
}
```

### Reactive Field Capture and Globals

Reactive fields defined with `::=` do not capture free variables from the surrounding environment. Instead, reactive fields inside structs are evaluated entirely in the context of the struct instance.

```haskell
x := 10;

struct Example {
    y;
    x;
    sum ::= x + y;
}

func main(){
    e = struct Example;
    e.y = 1;
    e.x = 1;
    println e.sum; # 2 #
}
```

If you want to use a global immutable within a struct reactive assignment:

```haskell
x := 10;

struct Example {
    y;
    xx := x;
    sum ::= xx + y;
}
```

## Arrays

Arrays are fixed-size, heap-allocated containers of values.

```lua
func main(){
    arr = [2];
    arr[1] = 10;
    println arr[1]; # 10 #
}
```

When used as integers, arrays evaluate to their length.

### Indexing and Assignment

Array elements are accessed with brackets:

```lua
func main(){
    arr = [2];
    arr[1] = 10;

    x = arr[1];
    print x; # 10 #
}
```

Array elements support both mutable (`=`) and reactive (`::=`) assignment.
Bounds are checked at runtime.

### Nested Arrays

```lua
func main(){
    matrix = [2];

    matrix[0] = [2];
    matrix[1] = [2];

    matrix[1][1] = 5;
    println matrix[1][1]; # 5 #
}
```

### Reactive Array Relationships

```lua
func main(){
    base = 0;
    arr = [2]

    arr[0] ::= base;
    arr[1] ::= arr[0] + 1;

    base = arr[1];
    println arr[1]; # 2 #
}
```

### Arrays and Structs

```lua
struct Cell {
    y = 0;
    yy ::= y * 2;
}

struct Container {
    m := [2];
}

func main(){
    c = struct Container;

    c.m[0] = [2];
    c.m[1] = [2];

    c.m[0][0] = struct Cell;
    c.m[0][1] = struct Cell;

    c.m[0][0].y = 5;
    println c.m[0][0].y;   # 5 #
    println c.m[0][0].yy;  # 10 #
}
```

## Functions

### Function Values and Calls

Functions encapsulate reusable logic and may return integers, characters, arrays, or structs.

```lua
func add(a, b) {
    return a + b;
}

println add(2, 3);  # 5 #
```

### Function Execution Model

Calling a function:

1) Creates a new immutable scope for parameters
2) Binds arguments to parameter names immutably
3) Executes the function body
4) Returns a value (or `0` if no return executes)

```lua
func f(x) {
    x = 10;   # error: x is immutable #
}
```

### Return Semantics

Returns are eager. Reactive relationships do not escape the function unless explicitly attached to a location outside.

```lua
func f(x) {
    y ::= x + 1;
    return y;
}

func main(){
    a = 10;
    b = f(a);
    a = 20;

    println b;  # 11 #
}
```

### Returned Heap Values Are Shared

Arrays and structs are heap-allocated and returned by reference.

```lua
struct Counter {
    x = 0;
    step := 1;
    next ::= x + step;
}

func make() {
    s := struct Counter;
    return s;
}

func main(){
    c1 = make();
    c2 = c1;

    c1.x = 10;
    println c2.x;  # 10 #
}
```

### Immutability Does Not Propagate Through Return

Returning an immutable binding yields a mutable value to the caller.

```lua
func f() {
    x := 5;
    return x;
}

func main(){
    y = f();
    y = 10;   # allowed #
}
```

### Reactive Bindings and Functions Returning Heap Objects

Reactive bindings may reference expressions that evaluate to heap-allocated values, including structs and arrays returned from functions.

```lua
struct Pair{
    x = 0;
    y = 0;
    xy ::= x + y;
}

func newpair(x,y){
    pair = struct Pair;
    pair.x = x;
    pair.y = y;
    return pair;
}

func main(){
    result ::= newpair(1, 2);
    println result.xy; # 3 #
}
```

Reactivity is expression-based, not identity-based:

```lua
struct Counter {
    x = 1;
    step = 1;
}

func buildcounter(start) {
    c := struct Counter;
    c.x = start;
    return c;
}

func main(){
    counter ::= buildcounter(10);
    counter.x = 20;
    println counter.x; # PRINTS 10, NOT 20 #
}
```

Use `:=` to capture the object instead:

```lua
func main(){
    counter := buildcounter(10);
    counter.x = 20;
    println counter.x; # PRINTS 20 #
}
```

## Imports and Modules

The language supports file-based imports using dot-separated paths.

```lua
import std.maths;
```

Imports load and execute another source file exactly once. Imports are not namespaced. Import order matters.

Imports are resolved relative to the program root by translating dots into folders:

```
game/entities/player.rx
```

## Standard Library (std)

The standard library is implemented as ordinary source files under `project/std/`.

Importing `std.file` registers native filesystem functions:

- `file_read(path)` -> string
- `file_write(path, contents)` -> number of chars written
- `file_exists(path)` -> 1 if exists, 0 otherwise
- `file_remove(path)` -> 1 on success

## Errors, Assert, and Stack Traces

`assert` and `error` stop execution and print a stack trace.

- `assert expr;` fails if `expr` evaluates to 0
- `error "message";` always fails (string literal only)

```lua
func div(a, b) {
    assert b != 0;
    return a / b;
}

func main(){
    div(10, 0);
}
```
