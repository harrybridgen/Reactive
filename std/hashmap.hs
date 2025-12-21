import std.maths;
struct HashMap {
    cap = 0;
    size = 0;
    keys;
    values;
    used;
}
func hashmap(capacity) {
    m := struct HashMap;

    m.cap = capacity;
    m.size = 0;

    m.keys   = [capacity];
    m.values = [capacity];
    m.used   = [capacity];

    return m;
}
func hash(key, cap) {
    return (key * 5761) % cap;
}
func put(m, key, value) {
    i := hash(key, m.cap);
    start := i;

    loop {
        if m.used[i] == 0 {
            m.used[i] = 1;
            m.keys[i] = key;
            m.values[i] = value;
            m.size = m.size + 1;
            return 1;
        }

        if m.keys[i] == key {
            m.values[i] = value;
            return 1;
        }

        i = (i + 1) % m.cap;

        if i == start {
            return 0;  
        }
    }
}
func get(m, key) {
    i := hash(key, m.cap);
    start := i;

    loop {
        if m.used[i] == 0 {
            return 0;
        }

        if m.keys[i] == key {
            return m.values[i];
        }

        i = (i + 1) % m.cap;

        if i == start {
            return 0;
        }
    }
}
func has(m, key) {
    i := hash(key, m.cap);
    start := i;

    loop {
        if m.used[i] == 0 {
            return 0;
        }

        if m.keys[i] == key {
            return 1;
        }

        i = (i + 1) % m.cap;

        if i == start {
            return 0;
        }
    }
}
func remove(m, key) {
    i := hash(key, m.cap);
    start := i;

    loop {
        if m.used[i] == 0 {
            return 0;
        }

        if m.keys[i] == key {
            m.used[i] = 0;
            m.size = m.size - 1;
            return 1;
        }

        i = (i + 1) % m.cap;

        if i == start {
            return 0;
        }
    }
}
func hashmap_get(m, key) {
    return get(m, key);
}