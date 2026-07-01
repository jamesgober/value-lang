<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br><b>value-lang</b><br>
    <sub><sup>API REFERENCE</sup></sub>
</h1>
<div align="center">
    <sup>
        <a href="../README.md" title="Project Home"><b>HOME</b></a>
        <span>&nbsp;│&nbsp;</span>
        <span>API</span>
        <span>&nbsp;│&nbsp;</span>
        <a href="../CHANGELOG.md" title="Changelog"><b>CHANGELOG</b></a>
    </sup>
</div>
<br>

Runtime value representation for interpreted languages, by NaN-boxing. The entire
public surface is one value type, one view enum, and a re-exported symbol handle.

- **Version:** 1.0.0
- **MSRV:** Rust 1.85 (2024 edition)
- **`no_std`:** yes (no `alloc` required)
- **Stability:** stable — the surface below is frozen (see [Stability](#stability)).

## Table of Contents

- **[Stability](#stability)**
- **[Installation](#installation)**
- **[Quick Start](#quick-start)**
- **[Public API](#public-api)**
  - [`Value`](#value)
    - [Constructors](#value-constructors)
    - [Kind predicates](#value-predicates)
    - [Accessors](#value-accessors)
    - [`bits`](#value-bits)
    - [`unpack`](#value-unpack)
    - [Trait implementations](#value-traits)
  - [`Unpacked`](#unpacked)
  - [`Symbol`](#symbol)
- **[Serde support](#serde-support)**
- **[Design notes](#design-notes)**
  - [The NaN-box layout](#the-nan-box-layout)
  - [Equality and hashing](#equality-and-hashing)
  - [Why `i32` integers](#why-i32-integers)

<br>

## Stability

As of **1.0.0** the public API documented here is **stable and frozen**. The crate
follows [Semantic Versioning](https://semver.org):

- Nothing in the frozen surface — the `Value` and `Unpacked` types, their methods and
  trait implementations, the re-exported `Symbol`, the `serde` representation, and the
  `std` / `serde` feature flags — will be removed or changed in a breaking way within
  the `1.x` series. A breaking change means a new major version.
- `1.x` releases may **add** to the surface (new methods, new trait impls, new
  variants gated appropriately) without breaking existing code.
- The **serialized form** of a `Value` under `serde` (an externally-tagged enum with
  symbols as their raw id) is part of the contract and will not change within `1.x`.
- The **NaN-box bit layout** behind [`bits`](#value-bits) is an implementation detail
  and is deliberately *not* promised: treat `bits` as an opaque, self-consistent token
  (stable within a build), not a wire format. Use `serde` for persistence.

MSRV (Rust 1.85) is treated as a compatibility surface: a raise is a minor, documented
change, never a patch.

<br>

## Installation

```toml
[dependencies]
value-lang = "1"

# Optional serde support:
value-lang = { version = "1", features = ["serde"] }

# no_std (drops the std forwarding to intern-lang):
value-lang = { version = "1", default-features = false }
```

<br>

## Quick Start

```rust
use value_lang::{Unpacked, Value};

let v = Value::int(42);

assert!(v.is_int());
assert_eq!(v.as_int(), Some(42));

match v.unpack() {
    Unpacked::Int(n) => assert_eq!(n, 42),
    _ => unreachable!(),
}
```

<br>
<hr>

## Public API

Everything is exported from the crate root: [`Value`](#value), [`Unpacked`](#unpacked),
and [`Symbol`](#symbol).

<br>

<h2 id="value"><code>Value</code></h2>

```rust
pub struct Value(/* private */);
```

A runtime value packed into a single 64-bit word by NaN-boxing. `Value` is `Copy`,
eight bytes wide, and carries no discriminant alongside it — the kind is folded into
the bit pattern. A real `f64` is stored as itself; `nil`, booleans, 32-bit integers,
and interned symbols hide inside quiet-NaN payloads that no genuine float produces.

The encoding is entirely safe (built from `f64::to_bits` / `f64::from_bits` and
integer arithmetic) and never boxes a pointer, so a `Value` carries no heap data.

<h3 id="value-constructors">Constructors</h3>

Each constructor takes one payload and returns a `Value` of that kind. `nil`, `bool`,
and `int` are `const fn`.

| Constructor | Signature | Description |
|---|---|---|
| `nil` | `const fn nil() -> Value` | The unit value — the absence of any other value. Also `Value::default()`. |
| `bool` | `const fn bool(b: bool) -> Value` | A boolean. |
| `int` | `const fn int(n: i32) -> Value` | A 32-bit signed integer, stored losslessly. |
| `float` | `fn float(f: f64) -> Value` | A double. Any finite value, both infinities, and `NaN` are accepted; every `NaN` is folded onto one canonical pattern. |
| `sym` | `fn sym(s: Symbol) -> Value` | An interned [`Symbol`](#symbol) handle. |

**Parameters**

- `bool(b)` — `b: bool`, the boolean to store.
- `int(n)` — `n: i32`. Use [`float`](#value-constructors) when you need a wider range or fractional values; `i32` is what fits losslessly beside the tag in a NaN-box payload.
- `float(f)` — `f: f64`, any double. `NaN` inputs round-trip as a canonical `NaN` (still `NaN`, still unequal to itself).
- `sym(s)` — `s: Symbol`, a handle issued by an [`Interner`](https://docs.rs/intern-lang). The value stores the handle, not the string bytes.

**Examples**

```rust
use value_lang::Value;

// Immediates.
let nothing = Value::nil();
let flag = Value::bool(true);
let count = Value::int(-7);
let ratio = Value::float(0.375);

assert!(nothing.is_nil());
assert_eq!(flag.as_bool(), Some(true));
assert_eq!(count.as_int(), Some(-7));
assert_eq!(ratio.as_float(), Some(0.375));

// `const` constructors work in const context.
const ZERO: Value = Value::int(0);
assert_eq!(ZERO.as_int(), Some(0));
```

```rust
use value_lang::Value;

// Extremes round-trip exactly.
assert_eq!(Value::int(i32::MIN).as_int(), Some(i32::MIN));
assert_eq!(Value::int(i32::MAX).as_int(), Some(i32::MAX));
assert_eq!(Value::float(f64::INFINITY).as_float(), Some(f64::INFINITY));
assert!(Value::float(f64::NAN).as_float().unwrap().is_nan());
```

```rust
use intern_lang::Interner;
use value_lang::Value;

let mut interner = Interner::new();
let name = interner.intern("width");
let v = Value::sym(name);

assert_eq!(v.as_sym(), Some(name));
assert_eq!(interner.resolve(v.as_sym().unwrap()), Some("width"));
```

<h3 id="value-predicates">Kind predicates</h3>

Each predicate reports whether the value is of one kind. They are mutually exclusive:
exactly one returns `true` for any value.

| Method | Signature | `true` when the value is… |
|---|---|---|
| `is_nil` | `fn is_nil(self) -> bool` | `nil` |
| `is_bool` | `fn is_bool(self) -> bool` | a boolean |
| `is_int` | `fn is_int(self) -> bool` | a 32-bit integer |
| `is_float` | `fn is_float(self) -> bool` | a float (including the infinities and `NaN`) |
| `is_sym` | `fn is_sym(self) -> bool` | an interned symbol |

**Examples**

```rust
use value_lang::Value;

let v = Value::float(1.5);
assert!(v.is_float());
assert!(!v.is_int());

// Exactly one predicate holds for any value.
let flags = [v.is_nil(), v.is_bool(), v.is_int(), v.is_float(), v.is_sym()];
assert_eq!(flags.iter().filter(|f| **f).count(), 1);
```

```rust
use value_lang::Value;

// `is_float` is true for every non-boxed kind, NaN and infinity included.
assert!(Value::float(f64::NAN).is_float());
assert!(Value::float(f64::NEG_INFINITY).is_float());
```

<h3 id="value-accessors">Accessors</h3>

Each accessor returns `Some(payload)` when the value is of that kind, or `None`
otherwise. They never coerce — [`as_float`](#value-accessors) on an `int` returns
`None`, not `Some(n as f64)`.

| Method | Signature | Returns |
|---|---|---|
| `as_bool` | `fn as_bool(self) -> Option<bool>` | the boolean, or `None` |
| `as_int` | `fn as_int(self) -> Option<i32>` | the integer, or `None` |
| `as_float` | `fn as_float(self) -> Option<f64>` | the float, or `None` |
| `as_sym` | `fn as_sym(self) -> Option<Symbol>` | the symbol, or `None` |

**Examples**

```rust
use value_lang::Value;

let v = Value::int(10);
assert_eq!(v.as_int(), Some(10));

// Accessors do not coerce across kinds.
assert_eq!(v.as_float(), None);
assert_eq!(v.as_bool(), None);
assert_eq!(Value::float(10.0).as_int(), None);
```

```rust
use value_lang::Value;

// Sum only the integers on a mixed stack.
let stack = [Value::int(2), Value::float(0.5), Value::int(3), Value::nil()];
let total: i32 = stack.iter().filter_map(|v| v.as_int()).sum();
assert_eq!(total, 5);
```

<h3 id="value-bits"><code>bits</code></h3>

```rust
const fn bits(self) -> u64
```

Returns the raw 64-bit NaN-box encoding. Stable for a given value, and useful for a
custom hash or a compact serialization. Note that raw bits are *identity*, not
numeric equality: two `NaN` floats share one canonical pattern, while `0.0` and
`-0.0` have distinct bits (see [Equality and hashing](#equality-and-hashing)).

**Examples**

```rust
use value_lang::Value;

// Every NaN folds onto one pattern.
let a = Value::float(f64::NAN);
let b = Value::float(f64::from_bits(0x7ff8_0000_dead_beef));
assert_eq!(a.bits(), b.bits());

// Signed zeros stay distinct in the bits.
assert_ne!(Value::float(0.0).bits(), Value::float(-0.0).bits());
```

<h3 id="value-unpack"><code>unpack</code></h3>

```rust
fn unpack(self) -> Unpacked
```

Expands the value into its [`Unpacked`](#unpacked) tagged-union form so you can
`match` on every kind at once. This is the inverse of `Value::from(unpacked)`; the
two round-trip losslessly.

**Parameters:** none. **Returns:** the [`Unpacked`](#unpacked) view.

**Examples**

```rust
use value_lang::{Unpacked, Value};

fn type_name(v: Value) -> &'static str {
    match v.unpack() {
        Unpacked::Nil => "nil",
        Unpacked::Bool(_) => "bool",
        Unpacked::Int(_) => "int",
        Unpacked::Float(_) => "float",
        Unpacked::Sym(_) => "sym",
    }
}

assert_eq!(type_name(Value::int(1)), "int");
assert_eq!(type_name(Value::nil()), "nil");
```

```rust
use value_lang::{Unpacked, Value};

// unpack / from round-trip.
let v = Value::float(2.5);
assert_eq!(v.unpack(), Unpacked::Float(2.5));
assert_eq!(Value::from(v.unpack()), v);
```

<h3 id="value-traits">Trait implementations</h3>

| Trait | Notes |
|---|---|
| `Copy`, `Clone` | `Value` is a single 64-bit word. |
| `Default` | `Value::default()` is [`nil`](#value-constructors). |
| `Debug` | Prints the unpacked kind, e.g. `Value(Int(5))`. |
| `PartialEq` | Floats compare with `f64` semantics; every other kind by bit pattern. **No `Eq`/`Hash`** — see [Equality and hashing](#equality-and-hashing). |
| `From<bool>`, `From<i32>`, `From<f64>`, `From<Symbol>` | Shorthand for the matching constructor. |
| `From<Unpacked>` | Packs a tagged-union view back into a `Value`. |

**Examples**

```rust
use value_lang::{Unpacked, Value};

// `From` shorthands.
let a: Value = 42_i32.into();
let b: Value = true.into();
let c: Value = 1.5_f64.into();
assert_eq!(a, Value::int(42));
assert_eq!(b, Value::bool(true));
assert_eq!(c, Value::float(1.5));

// Debug names the kind.
assert_eq!(format!("{:?}", Value::int(5)), "Value(Int(5))");

// Pack an Unpacked view.
assert_eq!(Value::from(Unpacked::Nil), Value::nil());
```

<br>
<hr>

<h2 id="unpacked"><code>Unpacked</code></h2>

```rust
pub enum Unpacked {
    Nil,
    Bool(bool),
    Int(i32),
    Float(f64),
    Sym(Symbol),
}
```

The tagged-union view of a [`Value`](#value), for exhaustive matching. A `Value`
hides its kind inside a bit pattern; `Unpacked` names it. Obtain one with
[`Value::unpack`](#value-unpack) and convert back with `Value::from`.

`Unpacked` derives `Clone`, `Copy`, `Debug`, and `PartialEq`. Like `Value`, it is not
`Eq` because it holds an `f64`.

**Variants**

| Variant | Payload | Corresponds to |
|---|---|---|
| `Nil` | — | [`Value::nil`](#value-constructors) |
| `Bool(bool)` | the boolean | [`Value::bool`](#value-constructors) |
| `Int(i32)` | the integer | [`Value::int`](#value-constructors) |
| `Float(f64)` | the double | [`Value::float`](#value-constructors) |
| `Sym(Symbol)` | the symbol handle | [`Value::sym`](#value-constructors) |

**Examples**

```rust
use value_lang::{Unpacked, Value};

let v = Value::bool(false);
assert_eq!(v.unpack(), Unpacked::Bool(false));

// Build a Value from a view.
assert_eq!(Value::from(Unpacked::Int(3)), Value::int(3));
```

```rust
use value_lang::{Unpacked, Value};

// A tiny evaluator that only understands integer addition.
fn add(a: Value, b: Value) -> Option<Value> {
    match (a.unpack(), b.unpack()) {
        (Unpacked::Int(x), Unpacked::Int(y)) => Some(Value::int(x + y)),
        _ => None,
    }
}

assert_eq!(add(Value::int(2), Value::int(3)), Some(Value::int(5)));
assert_eq!(add(Value::int(2), Value::nil()), None);
```

<br>
<hr>

<h2 id="symbol"><code>Symbol</code></h2>

```rust
pub use intern_lang::Symbol;
```

A compact, `Copy` handle for an interned string, re-exported from
[`intern-lang`](https://crates.io/crates/intern-lang). A `Value` can carry a `Symbol`
so identifiers and string constants cost four bytes and compare as integers. The
handle is only meaningful with the [`Interner`](https://docs.rs/intern-lang) that
issued it; `value-lang` stores the handle, never the bytes.

See the [`intern-lang` docs](https://docs.rs/intern-lang) for the full symbol and
interner API. The two operations you need alongside `Value` are `Interner::intern`
(string → `Symbol`) and `Interner::resolve` (`Symbol` → string).

**Example**

```rust
use intern_lang::Interner;
use value_lang::Value;

let mut interner = Interner::new();
let a = interner.intern("alpha");
let b = interner.intern("alpha"); // interning again yields the same symbol
assert_eq!(a, b);

let v = Value::sym(a);
assert_eq!(interner.resolve(v.as_sym().unwrap()), Some("alpha"));
```

<br>
<hr>

## Serde support

With the `serde` feature enabled, `Value` implements `serde::Serialize` and
`serde::Deserialize`. A `Value` is a bit pattern, not a self-describing shape, so it
serializes through its kinds as an externally-tagged enum. A symbol is written as its
raw 32-bit id — the same form `Symbol` itself uses — because the id is all that
survives without the issuing interner. Deserializing a symbol rejects `0`, which is
never a valid id.

```rust
# #[cfg(feature = "serde")]
# {
use value_lang::Value;

let v = Value::int(-5);
let json = serde_json::to_string(&v).unwrap();
assert_eq!(json, r#"{"Int":-5}"#);

let back: Value = serde_json::from_str(&json).unwrap();
assert_eq!(back, v);
# }
```

Persist symbols alongside the interner that produced them; a raw id is meaningless
against a different interner.

<br>
<hr>

## Design notes

### The NaN-box layout

A 64-bit IEEE-754 double is `sign(1) | exponent(11) | mantissa(52)`. A value is
*boxed* — not a real float — when the exponent is all ones and the top two mantissa
bits are set (the quiet-NaN header). No finite double or infinity matches that
pattern, and every genuine `NaN` is folded onto one canonical encoding on the way in,
so the boxed space is unambiguous.

Within a boxed word: the fixed header occupies the high bits, a 3-bit kind tag names
the kind, and the low 32 bits hold the payload — an `i32` bit-for-bit, or a symbol's
non-zero id. Decoding a value is one mask to test the header and a compare on the
tag. Classifying a float is a single mask-and-compare.

### Equality and hashing

`Value: PartialEq` follows IEEE-754 for floats and identity for everything else. Two
`Float` values compare with `f64` semantics — so `NaN != NaN` and `0.0 == -0.0` —
while all other kinds (including a `Float` against a non-`Float`) compare by bit
pattern. In particular, `Value::int(1) != Value::float(1.0)`: distinct kinds are
never equal.

Because `NaN != NaN`, `Value` deliberately does **not** implement `Eq` or `Hash`. If
you need a hashable key, branch on [`unpack`](#value-unpack) and hash the parts, or
use [`bits`](#value-bits) once you have separately ensured no float is `NaN`.

### Why `i32` integers

The NaN-box payload is 32 bits wide once the header and tag are accounted for, so an
`i32` is exactly what fits losslessly as an immediate. Values outside that range —
larger magnitudes or fractional numbers — are represented as `f64` via
[`float`](#value-constructors), which uses the full double. Keeping the integer
constructor total (every `i32` maps to a distinct `Value` and back, with no fallible
path) is a deliberate simplicity choice.

<br>
<hr>

<div align="center">
    <sup>
        <a href="../README.md" title="Project Home"><b>HOME</b></a>
        <span>&nbsp;│&nbsp;</span>
        <span>API</span>
        <span>&nbsp;│&nbsp;</span>
        <a href="../CHANGELOG.md" title="Changelog"><b>CHANGELOG</b></a>
    </sup>
</div>

<sub>Copyright &copy; 2026 <strong>James Gober</strong>.</sub>
