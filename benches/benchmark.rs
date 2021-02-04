extern crate criterion;

use ans_flex::count_unrolled;
use ans_flex::count_simple;
use std::iter;
use self::criterion::*;

const COMPRESSION1K: &'static [u8] = include_bytes!("compression_1k.txt");
const COMPRESSION34K: &'static [u8] = include_bytes!("compression_34k.txt");
const COMPRESSION65K: &'static [u8] = include_bytes!("compression_65k.txt");
const COMPRESSION66K: &'static [u8] = include_bytes!("compression_66k_JSON.txt");
const COMPRESSION95K_VERY_GOOD_LOGO: &'static [u8] = include_bytes!("logo.jpg");

const ALL: &[&[u8]] = &[
    COMPRESSION1K as &[u8],
    COMPRESSION34K as &[u8],
    COMPRESSION65K as &[u8],
    COMPRESSION66K as &[u8],
    COMPRESSION95K_VERY_GOOD_LOGO as &[u8],
];

fn from_elem(c: &mut Criterion) {

    let mut group = c.benchmark_group("count");
    for input in ALL.iter() {
        let input_bytes = input.len() as u64;
        group.throughput(Throughput::Bytes(input_bytes));
        group.bench_with_input(BenchmarkId::new("count_simple", input_bytes), &input, |b, i| {
            b.iter(|| count_simple(i));
        });
        group.bench_with_input(BenchmarkId::new("count_unrolled", input_bytes), &input, |b, i| {
            b.iter(|| count_unrolled(i));
        });
    }
    group.finish();
}

criterion_group!(benches, from_elem);
criterion_main!(benches);