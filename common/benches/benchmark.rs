extern crate criterion;

use self::criterion::*;
use common::count_blocked_unsafe;
use common::count_multi;
use common::count_multi2;
use common::count_simple;

const COMPRESSION1K: &[u8] = include_bytes!("../../test_data/compression_1k.txt");
const COMPRESSION34K: &[u8] = include_bytes!("../../test_data/compression_34k.txt");
const COMPRESSION65K: &[u8] = include_bytes!("../../test_data/compression_65k.txt");
const COMPRESSION66K: &[u8] = include_bytes!("../../test_data/compression_66k_JSON.txt");
const COMPRESSION19K: &[u8] = include_bytes!("../../test_data/v4_uuids_19k.txt");
const COMPRESSION93K: &[u8] = include_bytes!("../../test_data/v4_uuids_93k.txt");
// const COMPRESSION95K_VERY_GOOD_LOGO: &'static [u8] = include_bytes!("logo.jpg");

const ALL: &[&[u8]] = &[
    COMPRESSION1K as &[u8],
    COMPRESSION34K as &[u8],
    COMPRESSION65K as &[u8],
    COMPRESSION66K as &[u8],
    COMPRESSION19K as &[u8],
    COMPRESSION93K as &[u8],
    // COMPRESSION95K_VERY_GOOD_LOGO as &[u8],
];

fn count(c: &mut Criterion) {
    let mut group = c.benchmark_group("count");
    for input in ALL.iter() {
        let input_bytes = input.len() as u64;
        group.throughput(Throughput::Bytes(input_bytes));
        group.bench_with_input(
            BenchmarkId::new("count_simple", input_bytes),
            &input,
            |b, i| {
                b.iter(|| count_simple(i));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("count_multi", input_bytes),
            &input,
            |b, i| {
                b.iter(|| count_multi(i));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("count_multi2", input_bytes),
            &input,
            |b, i| {
                b.iter(|| count_multi2(i));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("count_blocked_unsafe", input_bytes),
            &input,
            |b, i| {
                b.iter(|| count_blocked_unsafe(i));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, count);
criterion_main!(benches);
