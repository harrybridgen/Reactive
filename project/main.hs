import std.maths;

x = 0;
step = 1;

i = 0;
di ::= i + 1;

loop {
    if i >= 15 {
        break;
    }

    println x;

    nx := x + step;              
    bx := clamp(nx, 0, 10);      

    x = bx;

    if i == 5 {
        step = 3;
    }

    if i == 10 {
        step = -2;
    }

    i = di;
}
