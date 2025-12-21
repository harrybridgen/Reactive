# ========================================= #
#        Reactive Hash Map (Integer)        #
# ========================================= #
#                                           #
# This module provides a simple integer     #
# hash map implementation suitable for      #
# reactive and non-reactive programs.       #
#                                           #
# The map uses open addressing with         #
# linear probing and fixed capacity.        #
#                                           #
# All operations are:                       #
# - integer-only                            #
# - deterministic                           #
# - safe for reactive expressions (::=)     #
#                                           #
# Keys and values are 32-bit signed ints.   #
#                                           #
# Import with:                              #
#     import std.hashmap;                   #
#                                           #
# ========================================= #


import std.maths;


# ----------------------------------------- #
# HashMap                                   #
# ----------------------------------------- #
# A fixed-capacity integer hash map.        #
#                                           #
# Fields:                                   #
#   cap    : capacity of the table          #
#   size   : number of stored entries       #
#   keys   : array of keys                  #
#   values : array of values                #
#   used   : occupancy flags (0 or 1)       #
#                                           #
# Notes:                                    #
# - capacity never grows                    #
# - collisions resolved via linear probing  #
# - removal leaves tombstones               #
# ----------------------------------------- #
struct HashMap {
    cap = 0;
    size = 0;
    keys;
    values;
    used;
}


# ----------------------------------------- #
# hashmap                                   #
# ----------------------------------------- #
# Create a new empty hash map with the      #
# given capacity.                           #
#                                           #
# All slots are initially unused.           #
#                                           #
# Args:                                     #
#   capacity : number of buckets            #
#                                           #
# Returns:                                  #
#   new HashMap instance                    #
# ----------------------------------------- #
func hashmap(capacity) {
    m := struct HashMap;

    m.cap = capacity;
    m.size = 0;

    m.keys   = [capacity];
    m.values = [capacity];
    m.used   = [capacity];

    return m;
}


# ----------------------------------------- #
# hash                                      #
# ----------------------------------------- #
# Compute the hash index for a key.         #
#                                           #
# This function is pure and deterministic.  #
#                                           #
# Args:                                     #
#   key : integer key                       #
#   cap : table capacity                    #
#                                           #
# Returns:                                  #
#   index in range [0, cap)                 #
# ----------------------------------------- #
func hash(key, cap) {
    return (key * 5761) % cap;
}


# ----------------------------------------- #
# put                                       #
# ----------------------------------------- #
# Insert or update a key-value pair.        #
#                                           #
# If the key already exists, its value is   #
# updated. Otherwise, a new entry is added. #
#                                           #
# Uses linear probing to resolve collisions #
#                                           #
# Args:                                     #
#   m     : HashMap                         #
#   key   : integer key                     #
#   value : integer value                   #
#                                           #
# Returns:                                  #
#   1 on success                            #
#   0 if the table is full                  #
# ----------------------------------------- #
func put(m, key, value) {
    i = hash(key, m.cap);
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


# ----------------------------------------- #
# get                                       #
# ----------------------------------------- #
# Retrieve the value associated with a key. #
#                                           #
# Uses linear probing to search.            #
#                                           #
# Args:                                     #
#   m   : HashMap                           #
#   key : integer key                       #
#                                           #
# Returns:                                  #
#   value if found                          #
#   0 if key is not present                 #
# ----------------------------------------- #
func get(m, key) {
    i = hash(key, m.cap);
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


# ----------------------------------------- #
# has                                       #
# ----------------------------------------- #
# Check whether a key exists in the map.    #
#                                           #
# Args:                                     #
#   m   : HashMap                           #
#   key : integer key                       #
#                                           #
# Returns:                                  #
#   1 if key exists                         #
#   0 otherwise                             #
# ----------------------------------------- #
func has(m, key) {
    i = hash(key, m.cap);
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


# ----------------------------------------- #
# remove                                    #
# ----------------------------------------- #
# Remove a key from the map if present.     #
#                                           #
# Marks the slot as unused and decrements   #
# the size counter.                         #
#                                           #
# Args:                                     #
#   m   : HashMap                           #
#   key : integer key                       #
#                                           #
# Returns:                                  #
#   1 if removed                            #
#   0 if key was not found                  #
# ----------------------------------------- #
func remove(m, key) {
    i = hash(key, m.cap);
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


# ----------------------------------------- #
# hashmap_get                               #
# ----------------------------------------- #
# Convenience wrapper around get().         #
#                                           #
# Intended for use in reactive expressions  #
# (e.g. y ::= hashmap_get(m, x))            #
#                                           #
# Args:                                     #
#   m   : HashMap                           #
#   key : integer key                       #
#                                           #
# Returns:                                  #
#   same as get(m, key)                     #
# ----------------------------------------- #
func hashmap_get(m, key) {
    return get(m, key);
}
