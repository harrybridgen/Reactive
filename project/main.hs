import std.hashmap;

m = hashmap(16);

put(m, 10, 100);
put(m, 20, 200);

println get(m, 10);   # 100 #
println get(m, 20);   # 200 #
println get(m, 30);   # 0 #

x = 10;
y ::= hashmap_get(m, x);

println y;   # 100 #

x = 20;
println y;   # 200 #

put(m, 20, 999);
println y;   # 999 #
