extern crate criterion;

use self::criterion::*;
use ans_flex::compress;
use ans_flex::decompress;
use ans_flex::FSE_DEFAULT_TABLELOG;
use common::count_simple;
use common::fse_optimal_table_log;
use common::get_max_symbol_value;
use common::get_normalized_counts;
use hufflpuff::build_tree_fast;
use hufflpuff::count_simple as huff_cnt_simple;

const COMPRESSION1K: &[u8] = include_bytes!("../test_data/compression_1k.txt");
const COMPRESSION34K: &[u8] = include_bytes!("../test_data/compression_34k.txt");
const COMPRESSION65K: &[u8] = include_bytes!("../test_data/compression_65k.txt");
const COMPRESSION66K: &[u8] = include_bytes!("../test_data/compression_66k_JSON.txt");
const COMPRESSION19K: &[u8] = include_bytes!("../test_data/v4_uuids_19k.txt");
const COMPRESSION93K: &[u8] = include_bytes!("../test_data/v4_uuids_93k.txt");
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
        group.bench_with_input(BenchmarkId::new("ans_flex", input_bytes), &input, |b, i| {
            b.iter(|| compress(i));
        });
        group.bench_with_input(
            BenchmarkId::new("build_tree_fast huffl", input_bytes),
            &input,
            |b, i| {
                b.iter(|| {
                    let counts = huff_cnt_simple(i);
                    build_tree_fast(&counts)
                });
            },
        );
    }
    group.finish();
}

fn decompression(c: &mut Criterion) {
    let mut group = c.benchmark_group("decompression");
    for input in ALL.iter() {
        let out = compress(input);
        let input_bytes = input.len() as u64;
        group.throughput(Throughput::Bytes(input_bytes));
        group.bench_with_input(
            BenchmarkId::new("ans_flex_complete", input_bytes),
            &out.get_compressed_data(),
            |b, i| {
                b.iter(|| {
                    let counts = count_simple(input);
                    let max_symbol_value = get_max_symbol_value(&counts);
                    let table_log =
                        fse_optimal_table_log(FSE_DEFAULT_TABLELOG, input.len(), max_symbol_value);
                    let norm_counts =
                        get_normalized_counts(&counts, table_log, input.len(), max_symbol_value);
                    decompress(i, &norm_counts, table_log, input.len(), max_symbol_value)
                });
            },
        );
        let counts = count_simple(input);
        let max_symbol_value = get_max_symbol_value(&counts);
        let table_log = fse_optimal_table_log(FSE_DEFAULT_TABLELOG, input.len(), max_symbol_value);
        let norm_counts = get_normalized_counts(&counts, table_log, input.len(), max_symbol_value);
        group.bench_with_input(
            BenchmarkId::new("ans_flex_reuse", input_bytes),
            &out.get_compressed_data(),
            |b, i| {
                b.iter(|| decompress(i, &norm_counts, table_log, input.len(), max_symbol_value));
            },
        );
    }
    group.finish();
}

// criterion_group!(benches, count, compression);
criterion_group!(benches, compression, decompression);
criterion_main!(benches);
