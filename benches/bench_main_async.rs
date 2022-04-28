use criterion::{criterion_main, Criterion};

mod _async;

criterion::criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(1000);
    targets =
    _async::bench_actix,
    _async::bench_xactor,
    _async::bench_xtor
);

criterion_main!(benches);
