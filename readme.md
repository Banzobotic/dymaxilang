# Dymaxilang

## Inspiration

One of javascript's many unusual features is the ability to access the field table of any object with convenient syntax:

```js
let regex = new RegExp();
regex.hi = "hello world";
console.log(regex["hi"]); // hello world
```

But what if this applied to every value and not just objects? And what if any equal value, in *any scope*, in *any function* was associated with the same hashmap? Named after the Dymaxion map projection, we present dymaxilang. 

## The Language

We think it should be obvious to users that the map set on line 2 refers to the value 3 rather than the variable `foo` itself.  

```rust
let foo = 3;
foo["hi"] = "hello world";
print((2 + 1)["hi"]); // hello world
```

Naturally revolutionary features like this one require sacrificing unimportant features like string indexing.

```rust
"hello world"[0] = "hi";
print("hello world"[0]); // hi
```

Instead, you can use the `chars` function to dump the string into the global hashmap.

```rust
let bar = "hello world";
let n = chars(hi);
print("chars"[0] + "chars"[n - 1]); // hd
```

The global hashmap shows its true power in more involved string processing tasks that take advantage of its 2d nature:

```rust
// input.txt:
// abcde
// fghij
// klmno
// pqrst
// uvwxy

let input = read("input.txt");
let n = split(input, "\n");
for i in 0>n {
	chars_into("split"[i], i);
}

let str = "";
for i in 0>n {
	str = str + i[i];
}
print(str); // agmsy
```

Users should be aware that values have the same associated hashmap in every scope, and it is up to them to ensure that functions they call don't unintentionally overwrite hashmap entries they are still using. 

```rust
let baz = fn () {
	0[0] = "world";
};

0[0] = "hello";
print(0[0]); // hello
baz();
print(0[0]); // world
```

## Performance

In a perfect world, dymaxilang wouldn't have if statements, and instead, all branching would be handled with dynamic dispatch as follows:

```rust
let fib = fn (n) {
	0[true] = fn (n) {
		return n;
	};
	0[false] = fn (n) {
		return fib(n - 2) + fib(n - 1);
	};

	return 0[n < 2](n);
};
```

Sadly the performance cost was too great for the committee to justify, so begrudgingly if statements were added.

```rust
let fib = fn (n) {
	if n < 2 { return n; }
	return fib(n - 2) + fib(n - 1);
};
```

This took the runtime for `fib(35)` from 5.0 seconds to 0.53 seconds, faster than a comparable python program which took 1.2 seconds. 

Similar performance gains are observed when computing the ackermann function, with `ackermann(3, 10)` taking 5.5 seconds in python and 0.85 seconds in dymaxilang. 

Sadly, the pursuit of perfection comes at a cost. It turns out that requiring two hashmap accesses to get a single value is really inefficient if that data could have been stored in a list. Computing how many of the first 4 million numbers are prime took 1.4 seconds in python but 2.9 seconds in dymaxilang. 

According to the committee, a more representative example is 2024 Advent of Code Day 1 (because it offered a more favourable result). With the input repeated 10 times to get a more measurable runtime, the python program took 1.6 seconds, while the dymaxilang program took 2.3 seconds. The committee considers this performance tradeoff acceptable in the pursuit of perfection. 

All programs mentioned, including their python counterparts, can be found in [examples](examples).
