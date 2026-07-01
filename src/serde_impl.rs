//! `serde` support for [`Value`], behind the `serde` feature.
//!
//! A [`Value`] is a bit pattern, not a self-describing shape, so it serializes
//! through its [`Unpacked`](crate::Unpacked) kinds instead. A symbol is written as
//! its raw 32-bit id — the same form [`Symbol`] itself uses — because the id is all
//! that survives without the issuing interner. Deserializing a symbol rejects `0`,
//! which is never a valid id.

use core::fmt;

use intern_lang::Symbol;
use serde::de::{self, Deserializer};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use crate::{Unpacked, Value};

/// The wire shape of a [`Value`]: one externally-tagged enum, symbols as raw ids.
#[derive(Serialize, Deserialize)]
enum Repr {
    Nil,
    Bool(bool),
    Int(i32),
    Float(f64),
    Sym(u32),
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let repr = match self.unpack() {
            Unpacked::Nil => Repr::Nil,
            Unpacked::Bool(b) => Repr::Bool(b),
            Unpacked::Int(n) => Repr::Int(n),
            Unpacked::Float(f) => Repr::Float(f),
            Unpacked::Sym(s) => Repr::Sym(s.as_u32()),
        };
        repr.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = match Repr::deserialize(deserializer)? {
            Repr::Nil => Value::nil(),
            Repr::Bool(b) => Value::bool(b),
            Repr::Int(n) => Value::int(n),
            Repr::Float(f) => Value::float(f),
            Repr::Sym(id) => {
                let symbol = Symbol::from_u32(id).ok_or_else(|| de::Error::custom(ZeroSymbol))?;
                Value::sym(symbol)
            }
        };
        Ok(value)
    }
}

/// A zero-sized error message, so the custom error needs no allocation.
struct ZeroSymbol;

impl fmt::Display for ZeroSymbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid symbol id 0")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use crate::Value;
    use intern_lang::Symbol;

    #[test]
    fn test_json_roundtrip_every_kind() {
        let s = Symbol::from_u32(11).unwrap();
        for v in [
            Value::nil(),
            Value::bool(true),
            Value::int(-5),
            Value::float(2.5),
            Value::sym(s),
        ] {
            let json = serde_json::to_string(&v).unwrap();
            let back: Value = serde_json::from_str(&json).unwrap();
            assert_eq!(back, v);
        }
    }

    #[test]
    fn test_symbol_serializes_as_raw_id() {
        let s = Symbol::from_u32(42).unwrap();
        let json = serde_json::to_string(&Value::sym(s)).unwrap();
        assert_eq!(json, r#"{"Sym":42}"#);
    }

    #[test]
    fn test_zero_symbol_id_is_rejected() {
        let err = serde_json::from_str::<Value>(r#"{"Sym":0}"#);
        assert!(err.is_err());
    }
}
