<h1 align="center">
    <img width="90px" height="auto" src="https://raw.githubusercontent.com/jamesgober/jamesgober/main/media/icons/hexagon-3.svg" alt="Triple Hexagon">
    <br><b>CHANGELOG</b>
</h1>
<p>
  All notable changes to <code>value-lang</code> will be documented in this file. The format is based on <a href="https://keepachangelog.com/en/1.1.0/">Keep a Changelog</a>,
  and this project adheres to <a href="https://semver.org/spec/v2.0.0.html/">Semantic Versioning</a>.
</p>

---

## [Unreleased]

### Added

### Changed

### Fixed

### Security

---

## [0.2.0] - 2026-07-01

The core runtime value representation. This release delivers the whole point of the
crate: a compact, NaN-boxed `Value` and its tagged-union view, fully safe and
`no_std`.

### Added

- `Value` — a runtime value packed into a single 64-bit word by NaN-boxing. Eight
  bytes, `Copy`, no discriminant. Constructors `nil`, `bool`, `int`, `float`, `sym`;
  kind predicates `is_nil` / `is_bool` / `is_int` / `is_float` / `is_sym`; accessors
  `as_bool` / `as_int` / `as_float` / `as_sym`; raw `bits`; and `unpack`.
- `Unpacked` — the tagged-union view (`Nil`, `Bool`, `Int`, `Float`, `Sym`) for
  exhaustive matching, with `Value: From<Unpacked>` and `Value::unpack` as duals.
- `Symbol` re-exported from `intern-lang`; a `Value` carries interned string handles
  packed into the tag.
- `From<bool>`, `From<i32>`, `From<f64>`, `From<Symbol>` for `Value`.
- Optional `serde` feature: `Serialize` / `Deserialize` for `Value` (symbols as their
  raw id; a `0` id is rejected on deserialize).
- Criterion benchmarks for the value hot paths (`benches/bench.rs`) and a `proptest`
  property suite for the round-trip and kind-exclusivity invariants
  (`tests/roundtrip.rs`).

### Changed

- Wired the `intern-lang` dependency (the roadmap's "wires intern" milestone) and
  forwarded the `std` feature to it.
- Adopted the full crate-level lint header (`forbid(unsafe_code)`, `deny(missing_docs)`
  and the REPS clippy set).
- Aligned the `clippy.toml` MSRV with `Cargo.toml` (1.85).

### Fixed

- The manifest `keywords` and `categories` were unquoted bare identifiers, so the
  crate did not parse as valid TOML. They are now proper string arrays.

---

## [0.1.0] - 2026-06-18

Initial scaffold and repository bootstrap. No domain logic yet &mdash; this release establishes the structure, tooling, and quality gates the implementation will be built on.

### Added

- `Cargo.toml` with crate metadata, Rust 2024 edition, MSRV 1.85.
- Dual `Apache-2.0 OR MIT` license files.
- `README.md`, `CHANGELOG.md`, and a documentation skeleton.
- `REPS.md` compliance baseline.
- `.github/workflows/ci.yml` CI matrix; `deny.toml`, `clippy.toml`, `rustfmt.toml`.
- `dev/DIRECTIVES.md` and `dev/ROADMAP.md` (committed engineering standards + plan).

[Unreleased]: https://github.com/jamesgober/value-lang/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/jamesgober/value-lang/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jamesgober/value-lang/releases/tag/v0.1.0
