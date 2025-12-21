# ========================================= #
#        Reactive Vector Mathematics        #
# ========================================= #
#                                           #
# This module provides a 2D integer vector  #
# type with reactive position updates.     #
#                                           #
# All operations are:                       #
# - integer-only                            #
# - pure and side-effect free               #
# - safe for reactive expressions (::=)     #
#                                           #
# Vectors are designed to work naturally   #
# with the reactive execution model.       #
#                                           #
# Import with:                              #
#     import std.vector;                    #
#                                           #
# ========================================= #

import std.maths;

# ----------------------------------------- #
# Vector2                                   #
# ----------------------------------------- #
# A simple 2D vector with velocity and      #
# reactive next-position fields.            #
#                                           #
# x, y   : position                         #
# vx, vy : velocity                         #
# dx, dy : next position (reactive)         #
# ----------------------------------------- #
struct Vector2 {
    x = 0;
    y = 0;

    vx = 0;
    vy = 0;

    dx ::= x + vx;
    dy ::= y + vy;

    mag2 ::= x*x + y*y;
    mag ::= sqrt(x*x + y*y);
}

func vec2(x, y) {
    v := struct Vector2;
    v.x = x;
    v.y = y;
    return v;
}

func vec2full(x, y, vx, vy) {
    v := struct Vector2;
    v.x = x;
    v.y = y;
    v.vx = vx;
    v.vy = vy;
    return v;
}

func update(v) {
    v.x = v.dx;
    v.y = v.dy;
    return v;
}

func add(a, b) {
    r := struct Vector2;
    r.x = a.x + b.x;
    r.y = a.y + b.y;
    return r;
}

func sub(a, b) {
    r := struct Vector2;
    r.x = a.x - b.x;
    r.y = a.y - b.y;
    return r;
}

func scale(v, s) {
    r := struct Vector2;
    r.x = v.x * s;
    r.y = v.y * s;
    return r;
}

func length2(v) {
    return v.x * v.x + v.y * v.y;
}

func length(v) {
    return sqrt(length2(v));
}

func distance2(a, b) {
    dx := a.x - b.x;
    dy := a.y - b.y;
    return dx * dx + dy * dy;
}

func distance(a, b) {
    return sqrt(distance2(a, b));
}

func setvelocity(v, vx, vy) {
    v.vx = vx;
    v.vy = vy;
    return v;
}

func addvelocity(v, ax, ay) {
    v.vx = v.vx + ax;
    v.vy = v.vy + ay;
    return v;
}

func zero(v) {
    v.x = 0;
    v.y = 0;
    v.vx = 0;
    v.vy = 0;
    return v;
}
