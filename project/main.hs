import std.queue;

q = queue(10);

println q.empty;  # 1 #

enqueue(q, 5);
println q.empty;  # 0 #
println q.size;   # 1 #
