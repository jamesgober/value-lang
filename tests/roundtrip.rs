//! Property tests for the NaN-box invariants.
//!
//! The core promise of the representation is that packing a value and reading it
//! back yields the same value, for every kind and every payload — and that the
//! kinds never collide. These properties assert that across randomized inputs.

#![allow(clippy::unwrap_used)]

use intern_lang::Symbol;
use proptest::prelude::*;
use value_lang::{Unpacked, Value};

proptest! {
    #[test]
    fn prop_int_roundtrips(n in any::<i32>()) {
        let v = Value::int(n);
        prop_assert!(v.is_int());
        prop_assert_eq!(v.as_int(), Some(n));
        prop_assert_eq!(v.unpack(), Unpacked::Int(n));
    }

    #[test]
    fn prop_finite_float_roundtrips(f in proptest::num::f64::NORMAL) {
        let v = Value::float(f);
        prop_assert!(v.is_float());
        prop_assert_eq!(v.as_float(), Some(f));
    }

    #[test]
    fn prop_any_float_stays_float(bits in any::<u64>()) {
        // Every possible f64 bit pattern, NaNs included, must classify as a float
        // and never as a boxed kind.
        let f = f64::from_bits(bits);
        let v = Value::float(f);
        prop_assert!(v.is_float());
        prop_assert!(!v.is_int());
        prop_assert!(!v.is_nil());
        prop_assert!(!v.is_bool());
        prop_assert!(!v.is_sym());
    }

    #[test]
    fn prop_symbol_roundtrips(id in 1u32..=u32::MAX) {
        let s = Symbol::from_u32(id).unwrap();
        let v = Value::sym(s);
        prop_assert!(v.is_sym());
        prop_assert_eq!(v.as_sym(), Some(s));
    }

    #[test]
    fn prop_unpack_from_is_identity(n in any::<i32>(), f in any::<f64>(), b in any::<bool>()) {
        for v in [Value::int(n), Value::float(f), Value::bool(b), Value::nil()] {
            // Round-trip through the tagged-union view. NaN needs a bit compare
            // because it is never `==` to itself.
            prop_assert_eq!(Value::from(v.unpack()).bits(), v.bits());
        }
    }

    #[test]
    fn prop_kinds_are_mutually_exclusive(n in any::<i32>()) {
        let v = Value::int(n);
        let flags = [v.is_nil(), v.is_bool(), v.is_int(), v.is_float(), v.is_sym()];
        prop_assert_eq!(flags.iter().filter(|f| **f).count(), 1);
    }
}
