arr = [10];

x = 0;
dx ::= x + 1;

arr[0] ::= dx;

loop{
    if x >= arr-1 {
        break;
    }

    i := x;
    arr[i+1] ::= arr[i] * 2;

    x = dx;
}

x = 20; 
println arr[9];
x = 10;
println arr[9];
