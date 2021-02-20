/*!
ans_flex is a FSE/ANS implementation in Rust, a compressor in the family of entropy encoders (statistical compression).

FSE ([Finite State Entropy](https://github.com/Cyan4973/FiniteStateEntropy/)) is a ANS variant from Yann Collet. Main advantage is, that it requires only additions,
masks, and shifts.

ANS (Asymetric Numeral Systems) was introduced by Jarek Duda and is a popular compression standard
used in compression algorithms like zstd, due to its high compression ration and reasonable
compression speed. In comparison to huffman it has the advantage to using fractional bits, when encoding symbols.

If you want a better understanding of ANS, I can recommend "Understanding Compression" by Colton
McAnlis and Aleks Haecky as the foundation and then diving into the blog posts of [Charles Bloom](http://cbloomrants.blogspot.com/2014/01/1-30-14-understanding-ans-1.html)
and [Yann Collet](https://fastcompression.blogspot.com/2013/12/finite-state-entropy-new-breed-of.html)
The [ANS paper](https://arxiv.org/pdf/1311.2540.pdf) from Jarek Duda is also interesting, but without a solid
foundation in math and compression it will be difficult to follow.

*/

use crate::table::DecompressionTable;
use crate::decompress::fse_decompress as other_fse_decompress;
use crate::table::build_decompression_table;
use crate::hist::NormCountsTable;
use crate::bitstream::bit_highbit32;
use crate::bitstream::BitCstream;
use crate::compress::fse_compress;
pub use crate::hist::count_simple;
use crate::hist::get_max_symbol_value;
use crate::hist::get_normalized_counts;
use crate::hist::CountsTable;
use crate::table::build_compression_table;
use crate::table::fse_optimal_table_log;

pub mod bitstream;
pub mod compress;
pub mod decompress;
pub mod hist;
pub mod table;

pub const FSE_DEFAULT_MEMORY_USAGE: u32 = 13;
pub const FSE_MAX_MEMORY_USAGE: u32 = 14; // 16kb
pub const FSE_DEFAULT_TABLELOG: u32 = FSE_DEFAULT_MEMORY_USAGE - 2;
pub const FSE_MIN_TABLELOG: u32 = 5;

pub const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;
pub const FSE_MAX_TABLELOG: u32 = FSE_MAX_MEMORY_USAGE - 2;
pub const FSE_MAX_TABLESIZE: usize = 1 << FSE_MAX_TABLELOG;
pub const FSE_MAXTABLESIZE_MASK: usize = FSE_MAX_TABLESIZE - 1;
pub const FSE_MAX_SYMBOL_VALUE: u32 = u8::MAX as u32;

pub const HIST_WKSP_SIZE_U32: usize = 1024;
pub const HIST_WKSP_SIZE: usize = HIST_WKSP_SIZE_U32 * core::mem::size_of::<usize>();

fn fse_tablestep(table_size: usize) -> usize {
    ((table_size) >> 1) + ((table_size) >> 3) + 3
}

#[derive(Debug)]
struct Counts {
    counts: CountsTable,
    total: usize,
}

// should be a generic eventually
// type FseFunctionType = u8;

pub fn get_ans_table_size(mut table_log: u32, max_symbol_value: u32) -> u32 {
    table_log = table_log.min(FSE_TABLELOG_ABSOLUTE_MAX);

    let size = 1 + (1 << (table_log - 1)) + ((max_symbol_value + 1) * 2);
    size
}

#[test]
fn test_get_ans_table_size() {
    assert_eq!(get_ans_table_size(10, 0), 515);
    assert_eq!(get_ans_table_size(10, 255), 1025);
    assert_eq!(get_ans_table_size(FSE_DEFAULT_TABLELOG, 255), 1537);
}

pub fn compress(input: &[u8]) -> BitCstream {
    let counts = count_simple(&input);
    let max_count = *counts.iter().max().unwrap() as usize;
    if max_count == input.len() {
        panic!("use rle");
    }; // only a single symbol in src : rle
    if max_count == 1 {
        panic!("not compressible");
    }; // each symbol present maximum once => not compressible
    if max_count < (input.len() >> 7) {
        panic!("not compressible enough");
    }; // Heuristic : not compressible enough

    let max_symbol_value = get_max_symbol_value(&counts);

    let table_log = fse_optimal_table_log(FSE_DEFAULT_TABLELOG, input.len(), max_symbol_value);

    let norm_counts = get_normalized_counts(&counts, table_log, input.len(), max_symbol_value);
    let comp_tables = build_compression_table(&norm_counts, table_log, max_symbol_value);

    let out = fse_compress(&input, &comp_tables, table_log);
    out
}


pub fn decompress(compressed: &[u8], norm_counts: &NormCountsTable, table_log: u32, orig_size: usize, max_symbol_value: u32) -> Vec<u8> {
    let mut output = vec![0_u8, 0];
    output.resize(orig_size, 0);

    let decomp_table = build_decompression_table(
        norm_counts,
        table_log,
        max_symbol_value,
    );

    fse_decompress(&mut output,&compressed, &decomp_table, table_log);
    output
}

pub fn fse_decompress(output: &mut Vec<u8>, input: &[u8], table: &DecompressionTable, table_log: u32) {
    other_fse_decompress(output, &input, &table, table_log)
}

#[cfg(test)]
mod tests {

    use crate::hist::count_simple;
    use std::sync::Once;

    static INIT: Once = Once::new();

    /// Setup function that is only run once, even if called multiple times.
    fn setup() {
        INIT.call_once(|| {
            env_logger::init();
        });
    }

    use super::*;

    const A_BYTE: u8 = "a".as_bytes()[0];
    const B_BYTE: u8 = "b".as_bytes()[0];
    const C_BYTE: u8 = "c".as_bytes()[0];

    fn get_test_data() -> Vec<u8> {
        use std::io::Read;
        let mut buffer = Vec::new();
        std::io::repeat(A_BYTE)
            .take(45)
            .read_to_end(&mut buffer)
            .unwrap(); // 45% prob
        std::io::repeat(B_BYTE)
            .take(35)
            .read_to_end(&mut buffer)
            .unwrap(); // 35% prob
        std::io::repeat(C_BYTE)
            .take(20)
            .read_to_end(&mut buffer)
            .unwrap(); // 20% prob

        buffer
    }


    fn get_test_data_flexible(size: usize) -> Vec<u8> {
        use std::io::Read;
        let mut buffer = Vec::new();
        std::io::repeat(A_BYTE)
            .take((size as f32 * 0.45) as u64)
            .read_to_end(&mut buffer)
            .unwrap(); // 45% prob
        std::io::repeat(B_BYTE)
            .take((size as f32 * 0.35) as u64)
            .read_to_end(&mut buffer)
            .unwrap(); // 35% prob
        std::io::repeat(C_BYTE)
            .take((size as f32 * 0.20) as u64)
            .read_to_end(&mut buffer)
            .unwrap(); // 20% prob

        buffer
    }

    #[test]
    fn test_compress_1() {
        setup();
        let test_data = get_test_data();
        let counts = count_simple(&test_data);
        assert_eq!(counts[A_BYTE as usize], 45);
        assert_eq!(counts[B_BYTE as usize], 35);
        assert_eq!(counts[C_BYTE as usize], 20);

        let out = compress(&test_data);
        dbg!(out.data_pos);
        dbg!(out.bit_pos);
        dbg!(out.bit_container);
    }

    #[test]
    fn test_roundtrip() {
        setup();
        let test_data = get_test_data();
        inverse(&test_data);
    }

    #[test]
    fn test_roundtrip_multi_sizes() {
        setup();

        for num_elems in 15..1000 {
            // println!("{:?}", num_elems);
            // let test_data = get_test_data_flexible(num_elems);
            let test_data = get_test_data_flexible(num_elems);

            // use std::io::Write;
            // std::fs::File::create("../FiniteStateEntropy/programs/test_data_100").unwrap().write_all(&test_data).unwrap();

            inverse(&test_data);
        }
    }

    #[test]
    fn test_66k_json() {
        setup();
        const TEST_DATA: &'static [u8] = include_bytes!("../benches/compression_66k_JSON.txt");
        inverse(TEST_DATA);
    }
    #[test]
    fn test_65k_text() {
        setup();
        const TEST_DATA: &'static [u8] = include_bytes!("../benches/compression_65k.txt");
        inverse(TEST_DATA);
    }
    #[test]
    fn test_34k_text() {
        setup();
        const TEST_DATA: &'static [u8] = include_bytes!("../benches/compression_34k.txt");
        inverse(TEST_DATA);
    }
    #[test]
    fn test_1k_text() {
        setup();
        const TEST_DATA: &'static [u8] = include_bytes!("../benches/compression_1k.txt");
        inverse(TEST_DATA);
    }
    
    #[test]
    fn test_v4_uuids_19_k() {
        setup();
        const TEST_DATA: &'static [u8] = include_bytes!("../benches/v4_uuids_19k.txt");
        inverse(TEST_DATA);
    }
    #[test]
    fn test_v4_uuids_93_k() {
        setup();
        const TEST_DATA: &'static [u8] = include_bytes!("../benches/v4_uuids_93k.txt");
        inverse(TEST_DATA);
    }
    
    fn inverse(test_data: &[u8]) {
        setup();
        let out = compress(&test_data);
        // dbg!(&out.get_compressed_data().len());
        // dbg!(out.bit_pos);
        // dbg!(out.bit_container);

        let counts = count_simple(&test_data);
        let max_symbol_value = get_max_symbol_value(&counts);
        let table_log = fse_optimal_table_log(FSE_DEFAULT_TABLELOG, test_data.len(), max_symbol_value);
        let norm_counts = get_normalized_counts(&counts, table_log, test_data.len(), max_symbol_value);

        let decompressed = decompress(&out.get_compressed_data(), &norm_counts, table_log, test_data.len(), max_symbol_value);
        assert_eq!(decompressed, test_data);
    }
}

// const COMPRESSION1K: &'static [u8] = include_bytes!("compression_1k.txt");
// const COMPRESSION34K: &'static [u8] = include_bytes!("compression_34k.txt");
// const COMPRESSION65K: &'static [u8] = include_bytes!("compression_65k.txt");
// const COMPRESSION66K: &'static [u8] = include_bytes!("compression_66k_JSON.txt");
// // const COMPRESSION95K_VERY_GOOD_LOGO: &'static [u8] = include_bytes!("logo.jpg");

// const ALL: &[&[u8]] = &[
//     COMPRESSION1K as &[u8],
//     COMPRESSION34K as &[u8],
//     COMPRESSION65K as &[u8],
//     COMPRESSION66K as &[u8],
//     // COMPRESSION95K_VERY_GOOD_LOGO as &[u8],
// ];