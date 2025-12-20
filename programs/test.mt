datasize = 10;
data = [datasize];

i = 0;
di ::= i + 1;

loop {
    if i >= datasize {
        break;
    }

    data[i] = i;
    i = di;
}

winsize = 3;
win = [winsize];

offset = 0;
doffset ::= offset + 1;

k = 0;
dk ::= k + 1;

loop {
    if k >= winsize {
        break;
    }

    idx := k;                     
    win[idx] ::= data[offset + idx];

    k = dk;
}

loop {

    p = 0;
    dp ::= p + 1;

    loop {
        if p >= winsize {
            break;
        }

        print win[p];
        p = dp;
    }

    println 0;
    offset = doffset;

    if offset > datasize - winsize {
        break;
    }
}
