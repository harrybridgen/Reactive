struct Queue {
    data;
    cap = 0;

    head = 0;
    tail = 0;
    count = 0;

    size ::= count;
    empty ::= count == 0;
    full ::= count == cap;
}

func queue(capacity) {
    q := struct Queue;
    q.data = [capacity];
    q.cap = capacity;

    q.head = 0;
    q.tail = 0;
    q.count = 0;

    return q;
}

func enqueue(q, value) {
    if q.count >= q.cap {
        return 0;   # full #
    }

    q.data[q.tail] = value;
    q.tail = (q.tail + 1) % q.cap;
    q.count = q.count + 1;

    return value;
}

func dequeue(q) {
    if q.count == 0 {
        return 0;   # empty #
    }

    v := q.data[q.head];
    q.head = (q.head + 1) % q.cap;
    q.count = q.count - 1;

    return v;
}

func peek(q) {
    if q.count == 0 {
        return 0;
    }

    return q.data[q.head];
}
