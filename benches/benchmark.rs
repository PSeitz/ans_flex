extern crate criterion;

use ans_flex::hist::count_multi;
use ans_flex::hist::count_blocked_unsafe;
use self::criterion::*;
use ans_flex::compress;
use ans_flex::count_simple;

const COMPRESSION1K: &'static [u8] = include_bytes!("compression_1k.txt");
const COMPRESSION34K: &'static [u8] = include_bytes!("compression_34k.txt");
const COMPRESSION65K: &'static [u8] = include_bytes!("compression_65k.txt");
const COMPRESSION66K: &'static [u8] = include_bytes!("compression_66k_JSON.txt");
// const COMPRESSION95K_VERY_GOOD_LOGO: &'static [u8] = include_bytes!("logo.jpg");

const ALL: &[&[u8]] = &[
    COMPRESSION1K as &[u8],
    COMPRESSION34K as &[u8],
    COMPRESSION65K as &[u8],
    COMPRESSION66K as &[u8],
    // COMPRESSION95K_VERY_GOOD_LOGO as &[u8],
];

// fn count(c: &mut Criterion) {
//     let mut group = c.benchmark_group("count");
//     for input in ALL.iter() {
//         let input_bytes = input.len() as u64;
//         group.throughput(Throughput::Bytes(input_bytes));
//         group.bench_with_input(
//             BenchmarkId::new("count_simple", input_bytes),
//             &input,
//             |b, i| {
//                 b.iter(|| count_simple(i));
//             },
//         );
//         group.bench_with_input(
//             BenchmarkId::new("count_multi", input_bytes),
//             &input,
//             |b, i| {
//                 b.iter(|| count_multi(i));
//             },
//         );
//         group.bench_with_input(
//             BenchmarkId::new("count_blocked_unsafe", input_bytes),
//             &input,
//             |b, i| {
//                 b.iter(|| count_blocked_unsafe(i));
//             },
//         );
//     }
//     group.finish();
// }
fn compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression");
    for input in ALL.iter() {
        let input_bytes = input.len() as u64;
        group.throughput(Throughput::Bytes(input_bytes));
        group.bench_with_input(
            BenchmarkId::new("compression_ans", input_bytes),
            &input,
            |b, i| {
                b.iter(|| compress(i));
            },
        );
    }
    group.finish();
}

// criterion_group!(benches, count, compression);
criterion_group!(benches, compression);
criterion_main!(benches);
