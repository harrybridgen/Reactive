# Standard Library

The standard library is implemented as ordinary source files under `project/std/`.
There is no special treatment for standard modules.

Example:

```lua
import std.maths;
```

## Filesystem (std.file)

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
