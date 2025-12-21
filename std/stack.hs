struct Stack {
    data;
    cap = 0;
    top = 0;           # index of next free slot #
    size ::= top;
    empty ::= top == 0;
}

func stack(capacity) {
    s := struct Stack;
    s.data = [capacity];
    s.cap = capacity;
    s.top = 0;
    return s;
}

func push(s, value) {
    if s.top >= s.cap {
        return 0;   # overflow #
    }

    s.data[s.top] = value;
    s.top = s.top + 1;
    return value;
}

func pop(s) {
    if s.top == 0 {
        return 0;   # underflow #
    }

    s.top = s.top - 1;
    return s.data[s.top];
}

func top(s) {
    if s.top == 0 {
        return 0;
    }

    return s.data[s.top - 1];
}
