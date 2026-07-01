//! # value_lang
//!
//! Compact runtime value representation for interpreted languages, by NaN-boxing.
//!
//! A dynamically-typed interpreter needs one type to stand for every runtime
//! value — nil, a boolean, a number, an interned name — and it copies that type on
//! nearly every instruction. `value-lang` packs all of those into a single 64-bit
//! [`Value`]: eight bytes, [`Copy`], and free of any discriminant word, because the
//! kind is folded into the bit pattern itself. The technique is *NaN-boxing* — real
//! floats are stored as themselves, and every other kind hides inside the quiet-NaN
//! encodings that no genuine float produces.
//!
//! The whole representation is safe Rust: it is built from [`f64::to_bits`] and
//! [`f64::from_bits`] and integer arithmetic, with no pointers and no `unsafe`
//! (`unsafe` is forbidden crate-wide). It owns value representation and nothing
//! else — strings and identifiers travel as [`Symbol`] handles from
//! [`intern-lang`](intern_lang), resolved against the interner that issued them.
//!
//! ## At a glance
//!
//! - [`Value`] — the eight-byte NaN-boxed handle. Build one with
//!   [`nil`](Value::nil), [`bool`](Value::bool), [`int`](Value::int),
//!   [`float`](Value::float), or [`sym`](Value::sym); test it with the `is_*`
//!   predicates; read it back with the `as_*` accessors.
//! - [`Unpacked`] — the tagged-union view. Call [`Value::unpack`] to `match` on
//!   every kind at once, and [`Value::from`] to pack one back.
//! - [`Symbol`] — re-exported from `intern-lang`; the compact string handle a
//!   `Value` can carry.
//!
//! ## Example
//!
//! ```
//! use value_lang::{Unpacked, Value};
//!
//! // One eight-byte type for every runtime value.
//! let stack = [Value::int(2), Value::float(0.5), Value::bool(true), Value::nil()];
//!
//! // Dispatch on the kind by unpacking.
//! let ints: i32 = stack
//!     .iter()
//!     .filter_map(|v| v.as_int())
//!     .sum();
//! assert_eq!(ints, 2);
//!
//! assert_eq!(stack[1].unpack(), Unpacked::Float(0.5));
//! assert_eq!(core::mem::size_of::<Value>(), 8);
//! ```
//!
//! ## `no_std`
//!
//! The crate is `no_std`-compatible and does not even require `alloc`: the
//! representation is pure integer and float arithmetic. The default `std` feature
//! is additive and only forwards `std` to `intern-lang`. Disable default features
//! to build for a bare target. The optional `serde` feature adds `Serialize` /
//! `Deserialize` for [`Value`].

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(unused_must_use)]
#![deny(unused_results)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::unreachable)]

mod value;

#[cfg(feature = "serde")]
mod serde_impl;

pub use intern_lang::Symbol;
pub use value::{Unpacked, Value};
