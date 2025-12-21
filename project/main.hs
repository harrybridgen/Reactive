import std.hashmap;

struct Pair {
    i = 0;
    j = 0;
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
            p.i = get(m, want);
            p.j = idx;
            return p;
        }

        put(m, x, idx);
        idx = didx;
    }

    return struct Pair;
}

# ---- test ---- #
nums = [4];
nums[0] = 2;
nums[1] = 7;
nums[2] = 11;
nums[3] = 15;

result := struct Pair;

result.i ::= twosum(nums, 9).i;
result.j ::= twosum(nums, 9).j;

println result.i;
println result.j;

nums[1] = 8;
nums[2] = 1;

println result.i;
println result.j;