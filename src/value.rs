//! The NaN-boxed [`Value`] and its [`Unpacked`] view.
//!
//! This module holds the whole runtime representation. [`Value`] is the compact,
//! eight-byte, `Copy` handle a bytecode interpreter passes around by value;
//! [`Unpacked`] is the tagged-union view you match on when you need to branch on
//! the kind. The two are duals: `Value::from(unpacked)` and `value.unpack()` round
//! trip losslessly.

use core::fmt;

use intern_lang::Symbol;

/// A runtime value, packed into a single 64-bit word by NaN-boxing.
///
/// A dynamic interpreter spends most of its time moving values between the stack,
/// locals, and the operand of an instruction. Representing each one as a Rust
/// `enum` costs sixteen bytes (a tag word plus the widest payload) and a branch on
/// every copy. NaN-boxing folds the kind *and* the payload into the bit pattern of
/// one `f64`-sized word, so a `Value` is `Copy`, eight bytes wide, and needs no
/// discriminant alongside it.
///
/// The trick is that IEEE-754 leaves a large block of bit patterns unused: every
/// quiet-NaN encoding names the same abstract "not a number". A `Value` stores a
/// real [`f64`] as itself, and hides every other kind — [`nil`](Value::nil),
/// booleans, 32-bit integers, and interned [`Symbol`]s — inside quiet-NaN payloads
/// that no genuine float ever produces. Reading a value back is a mask and a
/// compare; see [`unpack`](Value::unpack).
///
/// The encoding is entirely safe: it is built from [`f64::to_bits`] and
/// [`f64::from_bits`] and integer arithmetic, with no pointers and no `unsafe`.
/// Because it never boxes a pointer, it carries no heap data — strings and other
/// interned identities travel as [`Symbol`] handles, resolved elsewhere against the
/// interner that issued them.
///
/// # Equality
///
/// Equality follows IEEE-754 for floats and identity for everything else: two
/// [`Float`](Unpacked::Float) values compare with `f64` semantics, so `NaN != NaN`
/// and `0.0 == -0.0`; all other kinds (including a `Float` against a non-`Float`)
/// compare by bit pattern. Because `NaN != NaN`, `Value` deliberately does **not**
/// implement [`Eq`] or [`Hash`]; if you need a hashable key, branch on
/// [`unpack`](Value::unpack) and hash the parts, or use [`bits`](Value::bits) when
/// you have separately ensured no float is `NaN`.
///
/// # Examples
///
/// ```
/// use value_lang::{Unpacked, Value};
///
/// let answer = Value::int(42);
/// let ratio = Value::float(0.375);
///
/// assert!(answer.is_int());
/// assert_eq!(answer.as_int(), Some(42));
/// assert_eq!(ratio.as_float(), Some(0.375));
///
/// // Branch on the kind with `unpack`.
/// match answer.unpack() {
///     Unpacked::Int(n) => assert_eq!(n, 42),
///     _ => unreachable!(),
/// }
///
/// // A `Value` is eight bytes and `Copy`.
/// assert_eq!(core::mem::size_of::<Value>(), 8);
/// ```
#[derive(Clone, Copy)]
pub struct Value(u64);

// --- NaN-box layout -------------------------------------------------------
//
// A 64-bit IEEE-754 double is `sign(1) | exponent(11) | mantissa(52)`. A value is
// "boxed" (not a real float) when the exponent is all ones and the top two mantissa
// bits are set — the `QNAN` pattern below. No finite double or infinity matches it,
// and every genuine NaN is folded onto `CANON_NAN` on the way in, so the boxed space
// is ours alone.
//
// Within a boxed word: bits 62..=50 are the fixed `QNAN` header, a 3-bit tag lives at
// bits 34..=32, and a 32-bit payload occupies the low word. Booleans and nil need no
// payload; `Int` stores an `i32` bit-for-bit; `Sym` stores a `Symbol`'s `NonZeroU32`.

/// Quiet-NaN header: exponent all ones plus the two top mantissa bits.
const QNAN: u64 = 0x7ffc_0000_0000_0000;

/// Canonical float `NaN`. Every incoming `NaN` is folded onto this pattern, which
/// sits *outside* the [`QNAN`] boxed space so it reads back as a float.
const CANON_NAN: u64 = 0x7ff8_0000_0000_0000;

/// Bit offset of the 3-bit kind tag.
const TAG_SHIFT: u32 = 32;
/// Mask selecting the tag once shifted down.
const TAG_MASK: u64 = 0x7;

const TAG_NIL: u64 = 1;
const TAG_FALSE: u64 = 2;
const TAG_TRUE: u64 = 3;
const TAG_INT: u64 = 4;
const TAG_SYM: u64 = 5;

/// Builds the header for a boxed word carrying `tag`.
const fn boxed(tag: u64) -> u64 {
    QNAN | (tag << TAG_SHIFT)
}

const NIL_BITS: u64 = boxed(TAG_NIL);
const FALSE_BITS: u64 = boxed(TAG_FALSE);
const TRUE_BITS: u64 = boxed(TAG_TRUE);

impl Value {
    /// The unit value, `nil` — the absence of any other value.
    ///
    /// This is what an interpreter yields for an expression with no result, an
    /// uninitialised local, or a missing map entry. It is also [`Value::default`].
    ///
    /// # Examples
    ///
    /// ```
    /// use value_lang::Value;
    ///
    /// let v = Value::nil();
    /// assert!(v.is_nil());
    /// assert_eq!(v, Value::default());
    /// ```
    #[inline]
    #[must_use]
    pub const fn nil() -> Self {
        Self(NIL_BITS)
    }

    /// A boolean value.
    ///
    /// # Examples
    ///
    /// ```
    /// use value_lang::Value;
    ///
    /// assert_eq!(Value::bool(true).as_bool(), Some(true));
    /// assert_eq!(Value::bool(false).as_bool(), Some(false));
    /// ```
    #[inline]
    #[must_use]
    pub const fn bool(b: bool) -> Self {
        Self(if b { TRUE_BITS } else { FALSE_BITS })
    }

    /// A 32-bit signed integer.
    ///
    /// Integers are stored as an `i32` because that is what fits losslessly beside
    /// the tag in a NaN-box payload; use [`float`](Value::float) when you need the
    /// full magnitude and precision range of a double.
    ///
    /// # Examples
    ///
    /// ```
    /// use value_lang::Value;
    ///
    /// assert_eq!(Value::int(-7).as_int(), Some(-7));
    /// assert_eq!(Value::int(i32::MAX).as_int(), Some(i32::MAX));
    /// ```
    #[inline]
    #[must_use]
    pub const fn int(n: i32) -> Self {
        // `n as u32` reinterprets the two's-complement bit pattern; `as_int`
        // reverses it. The high 32 bits stay zero, so only the tag names the kind.
        Self(boxed(TAG_INT) | (n as u32 as u64))
    }

    /// A floating-point value.
    ///
    /// Any finite double, both infinities, and `NaN` are accepted. Every `NaN` is
    /// stored as one canonical bit pattern, so a round trip through `Value`
    /// normalises `NaN` payloads (the value is still `NaN`, and still compares
    /// unequal to itself).
    ///
    /// # Examples
    ///
    /// ```
    /// use value_lang::Value;
    ///
    /// assert_eq!(Value::float(2.5).as_float(), Some(2.5));
    /// assert_eq!(Value::float(f64::INFINITY).as_float(), Some(f64::INFINITY));
    /// assert!(Value::float(f64::NAN).as_float().unwrap().is_nan());
    /// ```
    #[inline]
    #[must_use]
    pub fn float(f: f64) -> Self {
        // Fold every NaN onto one pattern that lies outside the boxed space, so no
        // computed NaN can ever be mistaken for a boxed kind.
        if f.is_nan() {
            Self(CANON_NAN)
        } else {
            Self(f.to_bits())
        }
    }

    /// An interned [`Symbol`] — a compact handle for a string or identifier.
    ///
    /// The symbol's 32-bit id is packed directly into the value. It is only
    /// meaningful with the interner that issued it; `Value` stores the handle, not
    /// the bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use intern_lang::Interner;
    /// use value_lang::Value;
    ///
    /// let mut interner = Interner::new();
    /// let name = interner.intern("total");
    ///
    /// let v = Value::sym(name);
    /// assert_eq!(v.as_sym(), Some(name));
    /// assert_eq!(interner.resolve(v.as_sym().unwrap()), Some("total"));
    /// ```
    #[inline]
    #[must_use]
    pub fn sym(s: Symbol) -> Self {
        Self(boxed(TAG_SYM) | (s.as_u32() as u64))
    }

    /// Returns `true` when this value is [`nil`](Value::nil).
    #[inline]
    #[must_use]
    pub fn is_nil(self) -> bool {
        self.0 == NIL_BITS
    }

    /// Returns `true` when this value is a boolean.
    #[inline]
    #[must_use]
    pub fn is_bool(self) -> bool {
        self.0 == TRUE_BITS || self.0 == FALSE_BITS
    }

    /// Returns `true` when this value is a 32-bit integer.
    #[inline]
    #[must_use]
    pub fn is_int(self) -> bool {
        self.is_boxed() && self.tag() == TAG_INT
    }

    /// Returns `true` when this value is a float.
    ///
    /// Every value that is not a boxed kind is a float, including the infinities
    /// and `NaN`.
    #[inline]
    #[must_use]
    pub fn is_float(self) -> bool {
        !self.is_boxed()
    }

    /// Returns `true` when this value is an interned [`Symbol`].
    #[inline]
    #[must_use]
    pub fn is_sym(self) -> bool {
        self.is_boxed() && self.tag() == TAG_SYM
    }

    /// Returns the boolean, or `None` if this value is not a boolean.
    #[inline]
    #[must_use]
    pub fn as_bool(self) -> Option<bool> {
        match self.0 {
            TRUE_BITS => Some(true),
            FALSE_BITS => Some(false),
            _ => None,
        }
    }

    /// Returns the integer, or `None` if this value is not an integer.
    #[inline]
    #[must_use]
    pub fn as_int(self) -> Option<i32> {
        if self.is_int() {
            Some(self.0 as u32 as i32)
        } else {
            None
        }
    }

    /// Returns the float, or `None` if this value is not a float.
    ///
    /// This does **not** convert an [`int`](Value::int) to a float; it returns
    /// `None` for every non-float kind. Convert explicitly if you want coercion.
    #[inline]
    #[must_use]
    pub fn as_float(self) -> Option<f64> {
        if self.is_float() {
            Some(f64::from_bits(self.0))
        } else {
            None
        }
    }

    /// Returns the interned [`Symbol`], or `None` if this value is not a symbol.
    #[inline]
    #[must_use]
    pub fn as_sym(self) -> Option<Symbol> {
        if self.is_sym() {
            Symbol::from_u32(self.0 as u32)
        } else {
            None
        }
    }

    /// Returns the raw 64-bit NaN-box encoding.
    ///
    /// This is the exact bit pattern the value occupies. It is stable for a given
    /// value and useful for building a custom hash or a compact serialization, but
    /// note that two `NaN` floats share one canonical pattern and `0.0`/`-0.0` do
    /// not — so raw bits are identity, not numeric equality.
    #[inline]
    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Expands this value into its [`Unpacked`] tagged-union form for matching.
    ///
    /// This is the reverse of [`Value::from`]`(unpacked)`. Use it when you need to
    /// branch on every kind at once rather than test one kind with an `as_*`
    /// accessor.
    ///
    /// # Examples
    ///
    /// ```
    /// use value_lang::{Unpacked, Value};
    ///
    /// fn describe(v: Value) -> &'static str {
    ///     match v.unpack() {
    ///         Unpacked::Nil => "nil",
    ///         Unpacked::Bool(_) => "bool",
    ///         Unpacked::Int(_) => "int",
    ///         Unpacked::Float(_) => "float",
    ///         Unpacked::Sym(_) => "sym",
    ///     }
    /// }
    ///
    /// assert_eq!(describe(Value::int(1)), "int");
    /// assert_eq!(describe(Value::nil()), "nil");
    /// ```
    #[inline]
    #[must_use]
    pub fn unpack(self) -> Unpacked {
        if self.is_float() {
            return Unpacked::Float(f64::from_bits(self.0));
        }
        // Boxed: decode against the fixed patterns and the tag. The chain is total
        // because construction only ever produces these five tags.
        if self.0 == NIL_BITS {
            Unpacked::Nil
        } else if self.0 == FALSE_BITS {
            Unpacked::Bool(false)
        } else if self.0 == TRUE_BITS {
            Unpacked::Bool(true)
        } else if self.tag() == TAG_INT {
            Unpacked::Int(self.0 as u32 as i32)
        } else {
            // The only remaining tag is `Sym`. `from_u32` cannot fail here — a
            // symbol id is `NonZeroU32` — but if a hand-forged bit pattern ever
            // carried a zero payload we degrade to `nil` rather than panic.
            match Symbol::from_u32(self.0 as u32) {
                Some(s) => Unpacked::Sym(s),
                None => Unpacked::Nil,
            }
        }
    }

    /// Whether the word encodes a boxed (non-float) kind.
    #[inline]
    const fn is_boxed(self) -> bool {
        (self.0 & QNAN) == QNAN
    }

    /// The 3-bit kind tag. Only meaningful when [`is_boxed`](Value::is_boxed).
    #[inline]
    const fn tag(self) -> u64 {
        (self.0 >> TAG_SHIFT) & TAG_MASK
    }
}

impl Default for Value {
    /// The default value is [`nil`](Value::nil).
    #[inline]
    fn default() -> Self {
        Self::nil()
    }
}

impl PartialEq for Value {
    /// See the [type-level note on equality](Value#equality): floats compare with
    /// `f64` semantics, everything else by bit pattern.
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.is_float() && other.is_float() {
            f64::from_bits(self.0) == f64::from_bits(other.0)
        } else {
            self.0 == other.0
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Value").field(&self.unpack()).finish()
    }
}

impl From<bool> for Value {
    #[inline]
    fn from(b: bool) -> Self {
        Self::bool(b)
    }
}

impl From<i32> for Value {
    #[inline]
    fn from(n: i32) -> Self {
        Self::int(n)
    }
}

impl From<f64> for Value {
    #[inline]
    fn from(f: f64) -> Self {
        Self::float(f)
    }
}

impl From<Symbol> for Value {
    #[inline]
    fn from(s: Symbol) -> Self {
        Self::sym(s)
    }
}

impl From<Unpacked> for Value {
    #[inline]
    fn from(u: Unpacked) -> Self {
        match u {
            Unpacked::Nil => Self::nil(),
            Unpacked::Bool(b) => Self::bool(b),
            Unpacked::Int(n) => Self::int(n),
            Unpacked::Float(f) => Self::float(f),
            Unpacked::Sym(s) => Self::sym(s),
        }
    }
}

/// The tagged-union view of a [`Value`], for exhaustive matching.
///
/// A [`Value`] hides its kind inside a bit pattern; `Unpacked` names it. Obtain one
/// with [`Value::unpack`], and convert back with [`Value::from`]. This is the type
/// to `match` on when an interpreter dispatches on the kind of an operand.
///
/// # Examples
///
/// ```
/// use value_lang::{Unpacked, Value};
///
/// let v = Value::float(1.5);
/// assert_eq!(v.unpack(), Unpacked::Float(1.5));
/// assert_eq!(Value::from(Unpacked::Int(3)), Value::int(3));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unpacked {
    /// The unit value. See [`Value::nil`].
    Nil,
    /// A boolean. See [`Value::bool`].
    Bool(bool),
    /// A 32-bit signed integer. See [`Value::int`].
    Int(i32),
    /// A double-precision float. See [`Value::float`].
    Float(f64),
    /// An interned symbol handle. See [`Value::sym`].
    Sym(Symbol),
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn test_value_size_is_one_word() {
        assert_eq!(core::mem::size_of::<Value>(), 8);
        assert_eq!(core::mem::align_of::<Value>(), 8);
    }

    #[test]
    fn test_nil_roundtrips_and_is_default() {
        let v = Value::nil();
        assert!(v.is_nil());
        assert_eq!(v.unpack(), Unpacked::Nil);
        assert_eq!(v, Value::default());
        assert!(!v.is_bool());
        assert!(!v.is_int());
        assert!(!v.is_float());
        assert!(!v.is_sym());
    }

    #[test]
    fn test_bool_roundtrips_both_values() {
        for b in [true, false] {
            let v = Value::bool(b);
            assert!(v.is_bool());
            assert_eq!(v.as_bool(), Some(b));
            assert_eq!(v.unpack(), Unpacked::Bool(b));
        }
        assert_ne!(Value::bool(true), Value::bool(false));
    }

    #[test]
    fn test_int_roundtrips_including_extremes() {
        for n in [0, 1, -1, i32::MIN, i32::MAX, 123_456, -987_654] {
            let v = Value::int(n);
            assert!(v.is_int());
            assert_eq!(v.as_int(), Some(n));
            assert_eq!(v.unpack(), Unpacked::Int(n));
        }
    }

    #[test]
    fn test_float_roundtrips_including_infinities() {
        for f in [
            0.0,
            -0.0,
            1.5,
            -2.5,
            f64::MIN,
            f64::MAX,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            let v = Value::float(f);
            assert!(v.is_float());
            assert_eq!(v.as_float(), Some(f));
        }
    }

    #[test]
    fn test_float_nan_is_canonical_and_unequal_to_itself() {
        let v = Value::float(f64::NAN);
        assert!(v.is_float());
        assert!(v.as_float().unwrap().is_nan());
        // IEEE-754: NaN never equals NaN, even bit-identical.
        assert_ne!(v, v);
        // A NaN with a noisy payload folds onto the same canonical pattern.
        let noisy = Value::float(f64::from_bits(0x7ff8_0000_dead_beef));
        assert_eq!(v.bits(), noisy.bits());
    }

    #[test]
    fn test_float_signed_zero_compares_equal() {
        assert_eq!(Value::float(0.0), Value::float(-0.0));
        // ...but keeps distinct bits.
        assert_ne!(Value::float(0.0).bits(), Value::float(-0.0).bits());
    }

    #[test]
    fn test_sym_roundtrips() {
        let s = Symbol::from_u32(7).unwrap();
        let v = Value::sym(s);
        assert!(v.is_sym());
        assert_eq!(v.as_sym(), Some(s));
        assert_eq!(v.unpack(), Unpacked::Sym(s));
    }

    #[test]
    fn test_wrong_accessor_returns_none() {
        let v = Value::int(1);
        assert_eq!(v.as_bool(), None);
        assert_eq!(v.as_float(), None);
        assert_eq!(v.as_sym(), None);
        assert_eq!(Value::float(1.0).as_int(), None);
    }

    #[test]
    fn test_distinct_kinds_never_compare_equal() {
        // A float 1.0 and an int 1 are different values.
        assert_ne!(Value::int(1), Value::float(1.0));
        assert_ne!(Value::nil(), Value::bool(false));
        assert_ne!(Value::int(0), Value::nil());
    }

    #[test]
    fn test_from_impls_match_constructors() {
        assert_eq!(Value::from(true), Value::bool(true));
        assert_eq!(Value::from(9_i32), Value::int(9));
        assert_eq!(Value::from(1.25_f64), Value::float(1.25));
        assert_eq!(Value::from(Unpacked::Nil), Value::nil());
    }

    #[test]
    fn test_unpack_from_roundtrips() {
        let s = Symbol::from_u32(3).unwrap();
        for v in [
            Value::nil(),
            Value::bool(true),
            Value::bool(false),
            Value::int(-42),
            Value::float(12.5),
            Value::sym(s),
        ] {
            assert_eq!(Value::from(v.unpack()), v);
        }
    }

    #[test]
    fn test_debug_names_the_kind() {
        extern crate alloc;
        use alloc::format;
        assert_eq!(format!("{:?}", Value::int(5)), "Value(Int(5))");
        assert_eq!(format!("{:?}", Value::nil()), "Value(Nil)");
    }
}
