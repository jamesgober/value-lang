//! Criterion benchmarks for the `Value` hot paths.
//!
//! These measure the operations an interpreter performs per instruction: packing a
//! value, testing its kind, reading the payload back, unpacking to the enum, and
//! comparing two values. Run with `cargo bench --bench bench`.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use value_lang::{Unpacked, Value};

fn bench_construct(c: &mut Criterion) {
    let mut group = c.benchmark_group("construct");
    group.bench_function("int", |b| b.iter(|| Value::int(black_box(42))));
    group.bench_function("float", |b| b.iter(|| Value::float(black_box(9.5))));
    group.bench_function("bool", |b| b.iter(|| Value::bool(black_box(true))));
    group.bench_function("nil", |b| b.iter(Value::nil));
    group.finish();
}

fn bench_inspect(c: &mut Criterion) {
    let v = Value::int(7);
    let mut group = c.benchmark_group("inspect");
    group.bench_function("is_int", |b| b.iter(|| black_box(v).is_int()));
    group.bench_function("as_int", |b| b.iter(|| black_box(v).as_int()));
    group.finish();
}

fn bench_unpack(c: &mut Criterion) {
    let values = [
        Value::nil(),
        Value::bool(true),
        Value::int(123),
        Value::float(9.5),
    ];
    let _ = c.bench_function("unpack_mixed", |b| {
        b.iter(|| {
            let mut acc = 0i64;
            for v in black_box(&values) {
                acc += match v.unpack() {
                    Unpacked::Int(n) => n as i64,
                    Unpacked::Float(f) => f as i64,
                    Unpacked::Bool(true) => 1,
                    _ => 0,
                };
            }
            acc
        })
    });
}

fn bench_eq(c: &mut Criterion) {
    let a = Value::int(100);
    let b = Value::int(100);
    let _ = c.bench_function("eq_int", |bn| bn.iter(|| black_box(a) == black_box(b)));
}

criterion_group!(
    benches,
    bench_construct,
    bench_inspect,
    bench_unpack,
    bench_eq
);
criterion_main!(benches);
