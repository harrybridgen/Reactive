# Reactive Language

This is a small self hosted expression-oriented language compiled to bytecode and executed on a stack-based virtual machine.
The Rust compiler/VM live in `src/`, and a bootstrapped Reactive compiler lives in `project/bootstrap/`.

## Values and Types

- **Integers**: 32-bit signed integers
- **Characters**: Unicode scalar values ('A', 'b', '\n')
- **Strings**: Mutable arrays of characters ("HELLO")
- **Arrays**: Fixed-size, zero-initialized arrays of values (integers, characters, structs, or arrays).
- **Lazy values**: Expressions stored as ASTs and evaluated on access
- **Structs**: Heap-allocated records with named fields
- **Functions**: Callable units that may return integers, characters, arrays, or structs

Arrays (including strings) evaluate to their length when used as integers.

Whenever an array is used in a numeric context (arithmetic, comparison, loop conditions), it coerces to its length.

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

The language has **three assignment forms**, each with a distinct meaning.

### `=` Mutable Assignment

`=` creates or mutates a **mutable location**.

Mutable variables created with `=` are local to the current function invocation, unless they refer to an existing global or heap location.

```lua
func main(){
    x = 10;
    println x; # 10 #
}
```

Mutables can be passed into functions and returned, as usual in a C-like imperative language.

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

Inside structs, `=` creates a per-instance mutable field.
Each struct instance owns its own copy of the field.

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

`::=` defines a **relationship** between locations.  
It stores an expression and its dependencies, not a value.

```lua
func main(){
    x = 1;
    y ::= x + 1;

    println y; # 2 #
}
```

The expression is evaluated **when read**.

In the example above, when we call `println y;` we are essentially calculating `x + 1`, where `x` is the current value of `x`, _at this specific time_.

If any dependency changes, the result updates automatically.

```lua
func main(){
    x = 1;
    y ::= x + 1;

    println  y; # 2 #

    x = y;

    print  y; # 3 #
}
```

In the example above, when we assign `x = y;` we are assigning the current value produced by the relation y (which is x + 1) back into `x`. This causes `x == 2`, which then causes the relation `y` to be equal to `2 + 1` when evaluated.

Think of this as "advancing" the relation. In this case, it adds `1` onto `y`.

`::=` Reactive assignments:

- capture **dependencies**, not snapshots
- are lazy evaluated
- attach to the **location**, not the name

They are commonly used to build **progression variables** in loops:

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

Here, `dx` defines how `x` advances, while `=` controls when the update occurs.

Reactive assignments work uniformly for **variables, struct fields, array elements**

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

- Relationships attach to the underlying field or element, so all aliases observe the same behavior.
- Reactive assignments may depend on literals, other locations, and immutable bindings (`:=`).
- Reactive relationships remain fixed unless explicitly reassigned.

### `:=` Immutable Binding (capture / identity)

**`:=` is value capture, not assignment**

`:=` does not:

- create a location
- point to a variable
- participate in the reactive graph
- update when things change

`:=` takes a snapshot of a value and gives it a name

That name:

- is immutable
- is not reactive
- disappears when the scope ends
- cannot be reassigned
- cannot be observed reactively

_If the `:=` is binding an array or struct, the contents **are** mutable_

#### Why `:=` exists at all

Reactive bindings `::=` do not store values! They store relationships.
This means that:

```lua
arr[i] ::= arr[i - 1] + 1;
```

does not mean: use the current value of `i`
It means: use whatever `i` refers to when this expression is evaluated

So if `i` keeps changing, the dependency graph becomes self-referential, unstable, or incorrect.

#### The problem (without `:=`)

Take this code:

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

and **not**:

```
arr[0] = 0
arr[1] = 10
arr[2] = 20
```

Why?
Because `::=` doesn't store a value it stores whatever `i` is later.

So, you need to use the `:=` immutable bind to "capture" the value of `i`

```lua
func main(){
    arr = [3];
    i = 0;

    loop {
        j := i; # capture the current value #
        arr[j] ::= j * 10;
        i = i + 1;
        if i >= 3 { break; }
    }

    print arr[0];
    print arr[1];
    print arr[2];
}
```

Here, `j` freezes the value of `i` for each iteration.

Each reactive assignment becomes:

- independent
- anchored to a fixed index
- safe to evaluate later

Without `:=`, all reactive assignments would refer to the same moving variable, and the graph would be invalid.

## Characters and Strings

### Characters

Character literals use single quotes:

```lua
func main(){
    c = 'A';
    println c;   # A #
}
```

Characters are a distinct value kind that:

- print as characters
- can be indexed from strings
- can be explicitly cast to and from integers

However, all arithmetic is numeric.

### Characters in Expressions

Characters do not preserve character semantics through arithmetic.

```lua
func main(){
    x = 'A';
    y ::= x + 1;

    println y;   # 66 #
    x = 'Z';
    println y;   # 91 #
    println (char) y;   # [ #
}
```

In the example above:

- 'A' coerces to 65
- 'Z' coerces to 90
- arithmetic produces integers
- casting is required to interpret results as characters

### Explicit Casting

Casting is the only way to convert between characters and integers:

```haskell
func main(){
    n := (int)'A';
    c := (char)(n + 1);

    println n;  # 65 #
    println c;  # B #
}
```

Rules

- Arithmetic operators (+ - \* / %) always produce integers
- char values coerce to integers when used in numeric contexts
- (char) must be used explicitly to produce a character
- char prints as a character
- int prints as a number

### Strings

Strings use double quotes and are compiled as arrays of characters:

```lua
func main(){
    s := "HELLO";
    println s;      # HELLO #
    println s[1];   # E #
    println (int) s;    # 5 (coerce s into len int)#
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

Reactive bindings work naturally with characters and strings:

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

Strings are normal heap values:

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
- arithmetic always produces integers
- casting is required to print arithmetic results as characters

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

- = mutable field
- := immutable bind
- ::= reactive field

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

In the example above, we are defining:

- a `Counter` struct with:
- a mutable variable `x`
- an immutable variable `step`
- a relational variable `next` which depends on `x + step`
- an uninitialized variable `foo`

In `main` we construct the struct and assign it to `c`.
Then, mutate the `x` variable within `c`, which causes `next` to update automatically on read.

### Closed Structs

Fields in a struct must be declared in the struct definition.
You can initialize the fields with a value, or assign values to them later.
All unassigned fields in a struct have the type Uninitialized until they receive a value.

```lua
struct Empty {}

func main(){
    e := struct Empty;
    e.foo = 1; # Error ! #
}
```

### Reactive Field Capture and Globals

Reactive fields defined with `::=` do not capture free variables from the surrounding environment at definition time. Instead, reactive fields inside structs are evaluated entirely in the context of the struct instance.

Names referenced inside a reactive struct field always resolve to struct fields first, even if a global immutable with the same name exists.

For example:

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

In this example, sum evaluates to 2, not 11.

Although a global immutable `x := 10` exists, it does not participate in the reactive evaluation of sum. The identifier `x` inside `sum ::= x + y` resolves to the struct field `e.x`, and `y` resolves to the struct field `e.y`.

If you wanted to use a global immutable within a struct reactive assignment, you could do the following:

```haskell
x := 10;

struct Example {
    y;
    xx := x;
    sum ::= xx + y;
}

func main(){
    e = struct Example;
    e.y = 1;
    println e.sum;  # 11 #
}
```

## Arrays

Arrays are fixed-size, heap-allocated containers of values.
They may store integers, characters, structs, or other arrays.

Arrays are created using a size expression.

```lua
func main(){
    arr = [2]; # arr is an array of 2 elements #
    arr[1] = 10; # assign index 1 to int 10 #
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
Array values can be retrieved by both `::=` and `=` variables.
Bounds are checked at runtime.

### Nested Arrays

Arrays may contain other arrays, allowing arbitrary nesting.

```lua
# 2x2 Matrix #
func main(){
    matrix = [2];

    matrix[0] = [2];
    matrix[1] = [2];

    matrix[1][1] = 5;

    println matrix[1][1]; # 5 #
}
```

### Reactive Array Relationships

Reactive assignments to array elements capture relationships between values.

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

Changing any dependency automatically updates dependent elements.

### Arrays and Structs

Arrays may contain structs, and struct fields may contain arrays.
Field access (`.`) and indexing (`[]`) can be freely combined.

```lua
# A container holding a 2D array of cells #
struct Cell {
    y = 0;
    yy ::= y * 2;
}

struct Container {
    m := [2];
}

func main(){
    c = struct Container;

    # allocate 2x2 array of Cells #
    c.m[0] = [2];
    c.m[1] = [2];

    c.m[0][0] = struct Cell;
    c.m[0][1] = struct Cell;

    c.m[0][0].y = 5;

    println c.m[0][0].y;   # 5 #
    println c.m[0][0].yy;  # 10 #

    c.m[0][0].y = 7;
    println c.m[0][0].yy;  # 14 #
}
```

## Functions

### Function Values and Calls

Functions encapsulate reusable logic and may return **integers, characters, arrays, or structs**.

Functions are named callable values stored in the global environment.

```lua
func add(a, b) {
    return a + b;
}

println add(2, 3);  # 5 #
```

### Function Execution Model

Calling a function:

1.  Creates a **new immutable scope** for parameters
2.  Binds arguments to parameter names immutably
3.  Executes the function body
4.  Returns a value (or `0` if no return is executed)

```lua
func f(x) {
    x = 10;   # error: x is immutable #
}
```

Parameters behave like `:=` bindings.

### Return Semantics

Returns are **eager**

Returned expressions are evaluated **immediately**, not reactively.

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

Reactive relationships do **not escape** the function unless explicitly attached to a location outside.

### Returned Heap Values Are Shared

Arrays and structs are heap-allocated and returned **by reference**.

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

This sharing is intentional and allows mutation and reactivity across aliases.

### Immutability Does Not Propagate Through Return

Returning an immutable binding yields a **mutable value** to the caller.

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

Reactive bindings (::=) may reference expressions that evaluate to heap-allocated values, including structs and arrays returned from functions.

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
    result ::= newpair(1, 2); # returns pair struct #
    println result.xy; # 3 #
}
```

This is valid.

Reactive bindings store an expression (AST), not a snapshot.
When the binding is read, the expression is re-evaluated.

If the expression returns a heap object:

- the returned object is accessed normally
- field reads reflect the latest computed result
- mutations to dependencies trigger recomputation

Reactivity does not track object identity changes.
Instead, it re-evaluates the expression that produces the object.

### Reactivity Is Expression-Based, Not Identity-Based

Reactive bindings observe expressions, not object identity.

This means:

- the result of a function may change
- the heap object returned may change
- but reactivity is driven by expression re-evaluation, not pointer tracking

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

Each read of counter re-evaluates buildcounter(10) and discards any previous result.

If you wanted to make counter NOT re-evaluate, use the `:=` immutable binding:

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
    counter := buildcounter(10); # swapped ::= for := #
    counter.x = 20;
    println counter.x; # PRINTS 20 #
}
```

### Reactive Struct Fields vs Reactive Struct-Producing Expressions

You may bind:

- reactive fields inside structs
- reactive expressions that return structs
- Both are valid and supported.

Reactive fields attach to heap locations.
Reactive expressions attach to evaluation context.

Both coexist cleanly in the language.

## Imports and Modules

The language supports file-based imports using dot-separated paths.

```lua
import std.maths;
```

### Import Semantics

- Imports load and execute another source file exactly once
- Imported symbols (functions, structs, globals) become globally available
- Imports are not namespaced
- Import order matters
- Re-importing the same module is ignored

Imports are resolved relative to the program root by translating dots into folders.

```lua
import game.entities.player;
```

Resolves to:

```
game/entities/player.rx
```

### Nested Folders

Arbitrarily deep folder structures are supported.

Example project layout:

```
project/
  main.rx
  std/
    maths.rx
  game/
    entities/
      player.rx
```

### Example

game/entities/player.rx:

```lua
struct Player {
    x = 0;
    y = 0;
    xy ::= x + y;
}

func makeplayer(x, y) {
    p := struct Player;
    p.x = x;
    p.y = y;
    return p;
}
```

main.rx:

```lua
import game.entities.player;

func main(){
    player := makeplayer(10, 5);
    player.x = 5;
    println player.xy; # 10 #
}
```

### Standard Library (std)

The standard library is implemented as ordinary source files under the std/ folder.
There is no special treatment for standard modules.

```
std/
  maths.rx
  vector.rx
  foo.rx
  bar.rx
```

Modules are imported like any other file:

```
import std.maths;
```

### Filesystem (std.file)

Importing `std.file` registers native filesystem functions:

- `file_read(path)` -> string
- `file_write(path, contents)` -> number of chars written
- `file_exists(path)` -> 1 if exists, 0 otherwise
- `file_remove(path)` -> 1 on success

```lua
import std.file;

func main(){
    path := "notes.txt";
    file_write(path, "hello");

    if file_exists(path) {
        println file_read(path);
    }

    file_remove(path);
}
```

## Errors, Assert, and Stack Traces

`assert` and `error` stop execution and print a stack trace.

- `assert expr;` fails if `expr` evaluates to 0
- `error "message";` always fails (string literal only)
- stack traces include function names from most recent call to oldest

```lua
func div(a, b) {
    assert b != 0;
    return a / b;
}

func main(){
    div(10, 0);
}
```

## Examples

### Reactive variables

```lua
func main(){
    x = 1;
    y ::= x + 1;

    println y;   # 2 #
    x = 10;
    println y;   # 11 #
}
```

### Struct with Reactive Fields

```lua
struct Counter {
    x = 0;
    step := 1;
    next ::= x + step;
}

func main(){
    c = struct Counter;

    println c.next; # 1 #
    c.x = 10;
    println c.next; # 11 #
}
```

### Factorial via Dependency Graph

```lua
func main(){
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
}
```

### Functions Returning Structs

```lua
struct Counter {
    x = 0;
    step := 1;
    next ::= x + step;
}

func makecounter(start) {
    c = struct Counter;
    c.x = start;
    return c;
}

func advance(c) {
    c.x = c.next;
    return c.x;
}

func main(){
    counter = makecounter(10);

    println advance(counter); # 11 #
    println counter.next;     # 12 #
}

```

### Arrays and lazy elements

```lua
func main(){
    arr = [5];
    x = 2;

    arr[0] ::= x * 10;
    println arr[0];  # 20  #

    x = 7;
    println arr[0];  # 70 #
}
```

### Simple Counting Loop

```lua
func main(){
    x = 0;
    dx ::= x + 1;

    loop {
        println x;

        if x >= 4 {
            break;
        }

        x = dx; # advances x by +1 #
    }
}
```

### Array Dependency Chain

```lua
func main(){
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
}
```

### Nested Relational Arrays

```lua
func main(){
    x = [1];
    y = [1];
    z = [1];

    x[0] = y;
    y[0] ::= z[0] + 1;

    z[0] = 5;
    println x[0][0]; # 6 #
}
```

### 3D Matrix Relations

```lua
func main(){
    # create a 2x2x2 array #
    arr = [2];
    arr[0] = [2];
    arr[1] = [2];

    arr[0][0] = [2];
    arr[0][1] = [2];
    arr[1][0] = [2];
    arr[1][1] = [2];

    # establish a 3D dependency #
    arr[0][0][0] ::= arr[1][1][1];

    # set source value #
    arr[1][1][1] = 7;
    println arr[0][0][0];   # 7 #

    # change source again #
    arr[1][1][1] = 42;
    println arr[0][0][0];   # 42 #
}

```

### Matrix Multiplication with Relations

```lua
struct Mat2 {
    m := [2];
}


func mat2(a00, a01, a10, a11) {
    A := struct Mat2; # immutable binding is crucial #
    A.m[0] = [2];
    A.m[1] = [2];

    A.m[0][0] = a00;
    A.m[0][1] = a01;
    A.m[1][0] = a10;
    A.m[1][1] = a11;

    return A;
}

func mat2mul(A, B) {
    C := struct Mat2;
    C.m[0] = [2];
    C.m[1] = [2];

    # reactive matrix multiplication #
    C.m[0][0] ::= A.m[0][0]*B.m[0][0] + A.m[0][1]*B.m[1][0];
    C.m[0][1] ::= A.m[0][0]*B.m[0][1] + A.m[0][1]*B.m[1][1];

    C.m[1][0] ::= A.m[1][0]*B.m[0][0] + A.m[1][1]*B.m[1][0];
    C.m[1][1] ::= A.m[1][0]*B.m[0][1] + A.m[1][1]*B.m[1][1];

    return C;
}

func main(){
    A = mat2(1, 2,
            3, 4);

    B = mat2(5, 6,
            7, 8);

    C = mat2mul(A, B);

    # ---- initial product ---- #
    println C.m[0][0];  # 19 #
    println C.m[0][1];  # 22 #
    println C.m[1][0];  # 43 #
    println C.m[1][1];  # 50 #

    println ' ';
    # ---- mutate input matrix ---- #
    A.m[0][0] = 10;

    # ---- product updates automatically ---- #
    println C.m[0][0];  # 10*5 + 2*7 = 64 #
    println C.m[0][1];  # 10*6 + 2*8 = 76 #
    println C.m[1][0];  # unchanged: 43 #
    println C.m[1][1];  # unchanged: 50 #
}
```

### Bank Account with reactive fields

```lua
# Account struct with reactive fields #
struct Account {
    balance = 0;
    rate := 5;              # interest rate in percent #
    interest ::= balance * rate / 100;
    projected ::= balance + interest;
}

# create a new account #
func makeaccount(start) {
    a := struct Account;
    a.balance = start;
    return a;
}

# deposit money #
func deposit(a, amount) {
    a.balance = a.balance + amount;
    return a.balance;
}

# withdraw money #
func withdraw(a, amount) {
    if amount > a.balance {
        return a.balance;
    }

    a.balance = a.balance - amount;
    return a.balance;
}

# apply interest #
func applyinterest(a) {
    a.balance = a.projected;
    return a.balance;
}

# ---- demo ---- #
func main(){
    acct = makeaccount(1000);

    println acct.balance;    # 1000 #
    println acct.interest;   # 50 #
    println acct.projected;  # 1050 #

    deposit(acct, 500);
    println acct.projected;  # 1575 #

    applyinterest(acct);
    println acct.balance;    # 1575 #

    withdraw(acct, 200);
    println acct.projected;  # 1443 #
}
```

### Reactive Two Sum

```lua
import std.hashmap;

struct Pair {
    p1 = 0;
    p2 = 0;
}
func twosum(arr, target) {
    m := hashmap(arr);
    p := struct Pair;

    idx = 0;
    didx ::= idx + 1;

    loop {
        if idx >= arr {
            break;
        }

        x := arr[idx];
        want := target - x;

        if has(m, want) {
            p.p1 = get(m, want);
            p.p2 = idx;
            return p;
        }

        put(m, x, idx);
        idx = didx;
    }

    return p;
}

# ---- test ---- #
func main(){
    nums = [4];
    nums[0] = 2;
    nums[1] = 7;
    nums[2] = 11;
    nums[3] = 15;

    result ::= twosum(nums, 9);

    println result.p1; # 0 #
    println result.p2; # 1 #

    nums[0] = 12;
    nums[2] = 1;
    nums[3] = 8

    println result.p1; # 2 #
    println result.p2; # 3 #
}
```

### Reactive Fib in a Struct

```lua
struct Fibonacci {
    size := 10;

    n0 = 0;
    n1 = 1;

    seq = [10];
}

func initfib(f) {
    s := f.seq;

    s[0] ::= f.n0;
    s[1] ::= f.n1;

    x = 0;
    dx ::= x + 1;

    loop {
        if x >= f.size - 2 {
            break;
        }

        i := x;
        s[i + 2] ::= s[i] + s[i + 1];
        x = dx;
    }

    return f;
}

func printfib(f) {
    s := f.seq;

    x = 0;
    dx ::= x + 1;

    loop {
        if x >= f.size {
            break;
        }

        println s[x];
        x = dx;
    }
}

func main(){
    fib = struct Fibonacci;
    initfib(fib);

    printfib(fib);

    fib.n0 = 89;
    fib.n1 = 144;

    printfib(fib);
}

```

### Reactive Dot-Product Matrix

```lua
struct Vec2 {
    x = 0;
    y = 0;
}

func new_vec_array(n) {
    V := [n];

    i = 0;
    di ::= i + 1;

    loop {
        if i >= n { break; }
        V[i] = struct Vec2;
        i = di;
    }

    return V;
}

func new_matrix(rows, cols) {
    M := [rows];

    i = 0;
    di ::= i + 1;

    loop {
        if i >= rows { break; }
        M[i] = [cols];
        i = di;
    }

    return M;
}

func init_vec_values(V, x0, y0, dx, dy) {
    i = 0;
    di ::= i + 1;

    loop {
        if i >= V { break; }

        V[i].x = x0 + i * dx;
        V[i].y = y0 + i * dy;

        i = di;
    }
}



func bind_dot_products(M, A, B) {
    i = 0;
    di ::= i + 1;

    loop {
        if i >= A { break; }

        j = 0;
        dj ::= j + 1;

        loop {
            if j >= B { break; }

            ii := i;
            jj := j;

            M[ii][jj] ::=
                A[ii].x * B[jj].x +
                A[ii].y * B[jj].y;

            j = dj;
        }

        i = di;
    }
}

func print_matrix(M) {
    i = 0;
    di ::= i + 1;

    loop {
        if i >= M { break; }

        j = 0;
        dj ::= j + 1;

        loop {
            if j >= M[i] { break; }
            println M[i][j];
            j = dj;
        }

        i = di;
    }
}

func main(){
    A = new_vec_array(3);
    B = new_vec_array(3);

    # A = [(1,2), (3,4), (5,6)] #
    init_vec_values(A, 1, 2, 2, 2);

    # B = [(7,8), (9,10), (11,12)] #
    init_vec_values(B, 7, 8, 2, 2);

    M = new_matrix(A, B);
    bind_dot_products(M, A, B);

    # ---- initial matrix ---- #
    print_matrix(M);

    # ---- mutate vectors ---- #
    A[1].x = 100;
    B[2].y = 1;
    println ' ';

    # ---- matrix updates automatically ---- #
    print_matrix(M);
}

```

### Bouncing String via a Constraint-Driven Reactive Framebuffer

```lua
struct Screen {
    width;
    height;
    buf;
}

struct Text {
    str;
    len;

    x = 0;
    y = 0;
    vx = 1;
    vy = 1;

    dx ::= x + vx;
    dy ::= y + vy;
}

func make_text(str){
    text := struct Text;
    text.str := str;
    text.len ::= text.str;
    return text;
}

func make_screen(width, height) {
    screen := struct Screen;
    screen.width := width;
    screen.height := height;
    screen.buf := [screen.height];

    y = 0;
    dy ::= y + 1;

    loop {
        if y >= screen.height { break; }

        screen.buf[y] = [screen.width];

        y = dy;
    }
    return screen;
}

func framebuffer(screen, text) {

    y = 0;
    dy ::= y + 1;

    loop {
        if y >= screen.height { break; }

        x = 0;
        dx ::= x + 1;

        loop {
            if x >= screen.width { break; }

            yy := y;
            xx := x;

            screen.buf[yy][xx] ::=
                (yy == text.y &&
                 xx >= text.x &&
                 xx < text.x + text.len)
                    ? text.str[xx - text.x]
                    : (' ');

            x = dx;
        }

        y = dy;
    }
}
func render(screen) {
    print "\033[2J";
    print "\033[H";

    y = 0;
    dy ::= y + 1;

    loop {
        if y >= screen.height { break; }
        println screen.buf[y];
        y = dy;
    }
}
func delay(n) {
    d = 0;
    dd ::= d + 1;

    loop {
        if d >= n { break; }
        d = dd;
    }
}

func main(){
    text := make_text("HELLO REACTIVE");
    screen := make_screen(31,5);

    framebuffer(screen, text);

    loop {
        render(screen);
        delay(20000);

        text.x = text.dx;
        text.y = text.dy;

        if text.x < 0 {
            text.x = -text.x;
            text.vx = -text.vx;
        }

        if (text.x + text.len) > screen.width {
            text.x = (screen.width - text.len) - ((text.x + text.len) - screen.width);
            text.vx = -text.vx;
        }

        if text.y < 0 {
            text.y = -text.y;
            text.vy = -text.vy;
        }

        if text.y > (screen.height - 1) {
            text.y = (screen.height - 1) - (text.y - (screen.height - 1));
            text.vy = -text.vy;
        }
    }
}
```

## Grammar

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
