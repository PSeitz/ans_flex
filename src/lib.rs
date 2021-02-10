/*!
ans_flex is a FSE/ANS implementation in Rust, a compressor in the family of entropy encoders (statistical compression).

FSE (Finite State Entropy) is a ANS variant from Yann Collet. Main advantage is, that it requires only additions,
masks, and shifts.

ANS (Asymetric Numeral Systems) was introduced by Jarek Duda and is the defacto compression standard
used in popular compression algorithms like zstd, due to its high compression ration and reasonable
compression speed. In comparison to huffman it has the advantage to using fractional bits, when encoding symbols.

If you want a better understanding of ANS, I can recommend "Understanding Compression" by Colton
McAnlis and Aleks Haecky as the foundation and then diving into the blog posts of [Charles Bloom](http://cbloomrants.blogspot.com/2014/01/1-30-14-understanding-ans-1.html)
and [Yann Collet](https://fastcompression.blogspot.com/2013/12/finite-state-entropy-new-breed-of.html)
The [ANS paper](https://arxiv.org/pdf/1311.2540.pdf) from Jarek Duda is also interesting, but without a solid
foundation in math and compression it will be difficult to follow.

*/

use crate::hist::get_max_symbol_value;
use crate::bitstream::bit_highbit32;
use crate::bitstream::BitCstream;
use crate::compress::fse_compress_using_ctable_generic;
pub use crate::hist::count_simple;
use crate::hist::get_normalized_counts;
use crate::hist::CountsTable;
use crate::table::build_table;
use crate::table::fse_optimal_table_log;

pub mod bitstream;
pub mod compress;
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

// #define FSE_TABLESTEP(tableSize) (((tableSize)>>1) + ((tableSize)>>3) + 3)

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
    if max_count == input.len() { panic!("use rle");};   /* only a single symbol in src : rle */
    if max_count == 1 { panic!("not compressible");};         /* each symbol present maximum once => not compressible */
    if max_count < (input.len() >> 7) { panic!("not compressible enough");};   /* Heuristic : not compressible enough */

    let max_symbol_value = get_max_symbol_value(&counts);

    let table_log = fse_optimal_table_log(FSE_DEFAULT_TABLELOG, input.len(), max_symbol_value);

    let norm_counts = get_normalized_counts(&counts, table_log, input.len(), max_symbol_value);
    let comp_tables = build_table(
        &norm_counts,
        table_log,
        max_symbol_value,
    );

    let out = fse_compress_using_ctable_generic(&input, &comp_tables, table_log);
    out
}

// fn get_column_heights(counts: &CountsTable) -> Vec<u32> {
//     let max_val = get_table_max_val(&counts);
//     let total = get_num_symbols(&counts);

//     let sorted_counts = get_sorted_symbols(&counts);

//     let mut is_first = true; // first == most probable
//     let column_heigths = sorted_counts
//         .iter()
//         .map(|entry| {
//             let prob = counts[entry.symbol as usize] as f32 / total as f32;
//             let mut val = (max_val as f32 * prob).floor() as u32;

//             if is_first {
//                 is_first = false;
//                 val += 1;
//             }
//             val
//         })
//         .collect::<Vec<_>>();

//     column_heigths
// }

// fn get_most_probable_symbol(counts: &CountsTable) -> u8 {
//     get_sorted_symbols(&counts)[0].symbol
// }

// #[derive(Debug)]
// struct SymbolAndCount {
//     symbol: u8,
//     count: usize,
// }

// impl SymbolAndCount {
//     fn get_prob(&self, total: usize) -> f32 {
//         self.count as f32 / total as f32
//     }
// }

// fn get_sorted_symbols(counts: &CountsTable) -> Vec<SymbolAndCount> {
//     let mut symbols = counts
//         .into_iter()
//         .enumerate()
//         .filter(|(_, val)| **val != 0)
//         .map(|(symbol, val)| SymbolAndCount {
//             symbol: symbol as u8,
//             count: *val,
//         })
//         .collect::<Vec<_>>();

//     // symbols.sort_by(|symb_cnt| symb_cnt.count);
//     symbols.sort_by(|a, b| b.count.cmp(&a.count));

//     symbols
// }

// fn get_table_max_val(counts: &CountsTable) -> u32 {
//     // magic_extra_bits is some value between 2 and 8
//     // the higher the value, the better the compression, but it costs performance
//     let magic_extra_bits = 4;

//     let num_symbols = get_num_unique_symbols(&counts);
//     let num_precision_bits = (num_symbols as f32).log2() as u32 + magic_extra_bits;
//     let max_val = 2_u32.pow(num_precision_bits) - 1;
//     max_val
// }

// fn get_num_unique_symbols(counts: &CountsTable) -> usize {
//     counts.into_iter().filter(|el| **el != 0).count()
// }

// fn get_num_symbols(counts: &CountsTable) -> usize {
//     counts.into_iter().sum()
// }

// #[cfg(test)]
// mod tests {

//     use crate::table::fse_optimal_table_log;
//     use crate::hist::count_simple;
//     use crate::hist::get_normalized_counts;

//     use std::sync::Once;

//     static INIT: Once = Once::new();

//     /// Setup function that is only run once, even if called multiple times.
//     fn setup() {
//         INIT.call_once(|| {
//             env_logger::init();
//         });
//     }

//     use super::*;

//     const A_BYTE: u8 = "a".as_bytes()[0];
//     const B_BYTE: u8 = "b".as_bytes()[0];
//     const C_BYTE: u8 = "c".as_bytes()[0];

//     fn get_test_data() -> Vec<u8> {
//         use std::io::Read;
//         let mut buffer = Vec::new();
//         std::io::repeat(A_BYTE)
//             .take(45)
//             .read_to_end(&mut buffer)
//             .unwrap(); // 45% prob
//         std::io::repeat(B_BYTE)
//             .take(35)
//             .read_to_end(&mut buffer)
//             .unwrap(); // 35% prob
//         std::io::repeat(C_BYTE)
//             .take(20)
//             .read_to_end(&mut buffer)
//             .unwrap(); // 20% prob

//         buffer
//     }

//     #[test]
//     fn test_statistic_fns() {
//         setup();
//         let test_data = get_test_data();

//         let counts = count_simple(&test_data);
//         assert_eq!(counts[A_BYTE as usize], 45);
//         assert_eq!(counts[B_BYTE as usize], 35);
//         assert_eq!(counts[C_BYTE as usize], 20);

//         let table_log =
//             fse_optimal_table_log(FSE_DEFAULT_TABLELOG, test_data.len(), FSE_MAX_SYMBOL_VALUE);

//         let norm_counts = get_normalized_counts(&counts, table_log, test_data.len(), 255);

//         assert_eq!(get_num_unique_symbols(&counts), 3);

//         let sorted_counts = get_sorted_symbols(&counts);
//         assert_eq!(sorted_counts[0].symbol, A_BYTE);

//         let max_val = get_table_max_val(&counts);
//         assert_eq!(max_val, 31);

//         let column_heigths = get_column_heights(&counts);
//         assert_eq!(column_heigths, &[14, 10, 6]);
//         // assert_eq!(get_column_height(&counts, A_BYTE), 14);
//         // assert_eq!(get_column_height(&counts, B_BYTE), 10);
//         // assert_eq!(get_column_height(&counts, C_BYTE), 6);
//     }

// }

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

    #[test]
    fn test_compress() {
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

        // for index in 0..100 {
        //     println!("{:?} {:?}", index, test_data[index]);
        // }
    }
}
