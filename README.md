<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br>
    <b>value-lang</b>
    <br>
    <sub><sup>RUNTIME VALUES</sup></sub>
</h1>

<div align="center">
    <a href="https://crates.io/crates/value-lang"><img alt="Crates.io" src="https://img.shields.io/crates/v/value-lang"></a>
    <a href="https://crates.io/crates/value-lang"><img alt="Downloads" src="https://img.shields.io/crates/d/value-lang?color=%230099ff"></a>
    <a href="https://docs.rs/value-lang"><img alt="docs.rs" src="https://img.shields.io/docsrs/value-lang"></a>
    <a href="https://github.com/jamesgober/value-lang/actions"><img alt="CI" src="https://github.com/jamesgober/value-lang/actions/workflows/ci.yml/badge.svg"></a>
    <a href="https://github.com/rust-lang/rfcs/blob/master/text/2495-min-rust-version.md"><img alt="MSRV" src="https://img.shields.io/badge/MSRV-1.85%2B-blue"></a>
</div>

<br>

<div align="left">
    <p>
        value-lang is the LXRT-tier crate: Compact runtime value representation: tagged unions and NaN-boxing for interpreted languages. Part of the -lang language-construction family; see _strategy/LANG_COLLECTION.md for the master plan.
    </p>
    <br>
    <hr>
    <p>
        <strong>MSRV is 1.85+</strong> (Rust 2024 edition).
    </p>
    <blockquote>
        <strong>Status: stable.</strong> As of <code>1.0.0</code> the public API is frozen under Semantic Versioning; see <a href="./docs/API.md#stability"><code>docs/API.md</code></a> for the promise and <a href="./CHANGELOG.md"><code>CHANGELOG.md</code></a> for the history.
    </blockquote>
</div>

<hr>
<br>

<div align="left">
    <p>
        <strong>value-lang</strong> is a compact runtime value type for interpreted languages. A dynamically-typed interpreter needs one type to stand for every runtime value — <code>nil</code>, a boolean, a number, an interned name — and it copies that type on nearly every instruction. value-lang packs all of them into a single 64-bit <a href="./docs/API.md#value"><code>Value</code></a>: <b>eight bytes</b>, <b><code>Copy</code></b>, and free of any discriminant word, because the kind is folded into the bit pattern itself.
    </p>
    <p>
        The technique is <em>NaN-boxing</em>. IEEE-754 leaves a large block of <code>f64</code> bit patterns unused — every quiet-NaN encoding names the same abstract "not a number". A <code>Value</code> stores a real float as itself and hides every other kind inside those spare NaN payloads, which no genuine float ever produces. Reading a value back is a mask and a compare.
    </p>
    <p>
        The whole representation is <b>safe Rust</b>: it is built from <code>f64::to_bits</code>, <code>f64::from_bits</code>, and integer arithmetic, with no pointers and no <code>unsafe</code> (<code>unsafe</code> is forbidden crate-wide). It is <b><code>no_std</code></b> and needs no allocator. It owns value representation and nothing else — strings and identifiers travel as compact <a href="https://crates.io/crates/intern-lang"><code>Symbol</code></a> handles, resolved against the interner that issued them.
    </p>
</div>

<hr>
<br>

## Performance First

Every operation an interpreter runs per instruction is sub-nanosecond. Latest local Criterion means (`cargo bench --bench bench`, Linux x86_64 / WSL2, Rust stable, release build):

| Operation                  | Time     |
|----------------------------|---------:|
| Construct `nil`            |  0.09 ns |
| Construct `int` / `bool`   |  0.20 ns |
| Construct `float`          |  0.31 ns |
| `is_int` (kind test)       |  0.20 ns |
| `as_int` (read payload)    |  0.29 ns |
| `unpack` (per value)       |  0.32 ns |
| `==` (two ints)            |  0.34 ns |
| **Size of `Value`**        | **8 bytes** |

Numbers vary by CPU and environment; run the suite on your target to establish a baseline. A `Value` is `Copy` and register-sized, so passing one costs the same as passing a `u64`.

<br>
<hr>

## Features

- **One 8-byte type** — `nil`, `bool`, `i32`, `f64`, and interned `Symbol`, all in a single `Copy` word.
- **NaN-boxed** — real floats stored as themselves; every other kind hidden in spare quiet-NaN payloads.
- **Fully safe** — no `unsafe`, no pointers, no raw transmutes. `#![forbid(unsafe_code)]`.
- **`no_std`, no `alloc`** — pure integer and float arithmetic; runs anywhere.
- **Ergonomic and compact** — build with `Value::int(…)`, test with `is_*`, read with `as_*`, and `match` on the [`Unpacked`](./docs/API.md#unpacked) view when you need every kind at once.
- **`serde` ready** — opt-in `Serialize`/`Deserialize` for `Value`.
- **Property-tested invariants** — round-trip and kind-exclusivity checked across randomized inputs with `proptest`.

<br>
<hr>

## Installation

```toml
[dependencies]
value-lang = "1"

# With serde support:
value-lang = { version = "1", features = ["serde"] }

# no_std (drops the std forwarding to intern-lang):
value-lang = { version = "1", default-features = false }
```

**MSRV is 1.85+** (Rust 2024 edition).

<hr>
<br>

## Quick Start

```rust
use value_lang::{Unpacked, Value};

// One eight-byte type for every runtime value.
let stack = [
    Value::int(2),
    Value::float(0.5),
    Value::bool(true),
    Value::nil(),
];

// Test a single kind and read it back.
assert!(stack[0].is_int());
assert_eq!(stack[0].as_int(), Some(2));
assert_eq!(stack[1].as_float(), Some(0.5));

// Or match on every kind at once.
fn describe(v: Value) -> &'static str {
    match v.unpack() {
        Unpacked::Nil => "nil",
        Unpacked::Bool(_) => "bool",
        Unpacked::Int(_) => "int",
        Unpacked::Float(_) => "float",
        Unpacked::Sym(_) => "sym",
    }
}
assert_eq!(describe(stack[3]), "nil");

// A Value is register-sized and Copy.
assert_eq!(core::mem::size_of::<Value>(), 8);
```

### Interned names

Strings and identifiers are carried as [`Symbol`](https://crates.io/crates/intern-lang) handles. value-lang stores the handle; the interner owns the bytes.

```rust
use intern_lang::Interner;
use value_lang::Value;

let mut interner = Interner::new();
let name = interner.intern("total");

let v = Value::sym(name);
assert_eq!(v.as_sym(), Some(name));
assert_eq!(interner.resolve(v.as_sym().unwrap()), Some("total"));
```

<hr>
<br>

## How NaN-boxing works

A 64-bit IEEE-754 double is `sign(1) | exponent(11) | mantissa(52)`. A value is *boxed* — not a real float — when the exponent is all ones and the top two mantissa bits are set. No finite double or infinity matches that pattern, and every genuine `NaN` is folded onto one canonical encoding on the way in, so the boxed space belongs to value-lang alone.

Within a boxed word, a fixed quiet-NaN header sits in the high bits, a small kind tag names the kind, and the low 32 bits hold the payload: an `i32` bit-for-bit, or a `Symbol`'s non-zero id. Decoding is one mask to check the header and a compare on the tag. There are no branches on the copy path and no heap indirection.

Because floats keep IEEE-754 semantics, equality does too: two `Float` values compare with `f64` rules — so `NaN != NaN` and `0.0 == -0.0` — while every other kind compares by bit pattern. `Value` therefore does not implement `Eq`/`Hash`; see [`docs/API.md`](./docs/API.md#value) for the details and how to build a key.

<hr>
<br>

## API Overview

For the complete reference with examples, see [`docs/API.md`](./docs/API.md).

- [`Value`](./docs/API.md#value) — the eight-byte NaN-boxed value; constructors, `is_*` predicates, `as_*` accessors, `bits`, `unpack`.
- [`Unpacked`](./docs/API.md#unpacked) — the tagged-union view for exhaustive `match`.
- [`Symbol`](./docs/API.md#symbol) — the interned string handle a `Value` can carry (re-exported from `intern-lang`).

<br>

### Feature Flags

| Feature | Default | Description                                                        |
|---------|:-------:|--------------------------------------------------------------------|
| `std`   | ✅      | Forwards `std` to `intern-lang`. The crate itself is `no_std`.     |
| `serde` | ❌      | `Serialize` / `Deserialize` for `Value` (symbols as their raw id). |

<hr>
<br>

## Testing

```bash
cargo test                 # unit + property + doctests
cargo test --all-features  # adds the serde round-trip tests
cargo bench --bench bench  # Criterion hot-path benchmarks
```

The property suite in [`tests/roundtrip.rs`](./tests/roundtrip.rs) checks the core invariants — pack/unpack is the identity, every `f64` bit pattern classifies as a float, and the kinds are mutually exclusive — across randomized inputs.

<hr>
<br>

## Cross-Platform Support

The representation is pure arithmetic with no platform-specific code, so it behaves identically everywhere Rust runs. CI covers **Linux**, **macOS**, and **Windows** on both stable and the 1.85 MSRV.

<hr>
<br>

## Contributing

See <a href="./REPS.md"><code>REPS.md</code></a> for the engineering standards and the definition of done. Before a PR: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` must be clean.

<br>

<div id="license">
    <h2>License</h2>
    <p>Licensed under either of</p>
    <ul>
        <li><b>Apache License, Version 2.0</b> &mdash; <a href="./LICENSE-APACHE">LICENSE-APACHE</a></li>
        <li><b>MIT License</b> &mdash; <a href="./LICENSE-MIT">LICENSE-MIT</a></li>
    </ul>
    <p>at your option.</p>
</div>

<div align="center">
  <h2></h2>
  <sup>COPYRIGHT <small>&copy;</small> 2026 <strong>James Gober <me@jamesgober.com>.</strong></sup>
</div>
