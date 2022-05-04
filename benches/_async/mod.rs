mod actix;
mod xactor;
mod xtor;

use std::fmt::Display;

use criterion::{BenchmarkId, Criterion};

pub fn bench_actix(c: &mut Criterion) {
    let tests = gen_tests("actix");
    let mut group = c.benchmark_group("xtor");
    for spec in tests {
        group.bench_with_input(BenchmarkId::from_parameter(&spec), &spec, |b, spec| {
            b.iter(|| actix::run(*spec));
        });
    }
    group.finish();
}
pub fn bench_xactor(c: &mut Criterion) {
    let tests = gen_tests("xactor");
    let mut group = c.benchmark_group("xtor");
    for spec in tests {
        group.bench_with_input(BenchmarkId::from_parameter(&spec), &spec, |b, spec| {
            b.iter(|| xactor::run(*spec));
        });
    }
    group.finish();
}
pub fn bench_xtor(c: &mut Criterion) {
    let tests = gen_tests("xtor");
    let mut group = c.benchmark_group("xtor");
    for spec in tests {
        group.bench_with_input(BenchmarkId::from_parameter(&spec), &spec, |b, spec| {
            b.iter(|| xtor::run(*spec));
        });
    }
    group.finish();
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Spec {
    name: &'static str,
    parallel: usize,
    number: usize,
}

impl Display for Spec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "spec<{}:{}x{}>", self.name, self.parallel, self.number)
    }
}

fn gen_tests(name: &'static str) -> Vec<Spec> {
    let mut specs = vec![];
    for i in 0..=4 {
        for j in 1..=3 {
            let spec = Spec {
                name,
                parallel: 1 << i,
                number: usize::pow(10, j),
            };
            specs.push(spec);
        }
    }
    specs
}
