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

use crate::bitstream::bit_highbit32;

pub mod bitstream;

use log::log_enabled;
use log::Level::Trace;
use log::*;

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

type CountsTable = [usize; FSE_MAX_SYMBOL_VALUE as usize + 1];
type NormCountsTable = [i16; FSE_MAX_SYMBOL_VALUE as usize + 1];

fn fse_tablestep(table_size: usize) -> usize {
    ((table_size) >> 1) + ((table_size) >> 3) + 3
}

// #define FSE_TABLESTEP(tableSize) (((tableSize)>>1) + ((tableSize)>>3) + 3)

#[derive(Debug)]
struct Counts {
    counts: CountsTable,
    total: usize,
}

fn fse_min_table_log(src_size: usize, max_symbol_value: u32) -> u32 {
    assert!(src_size > 1); // not supported
    let min_bits_src: u32 = bit_highbit32(src_size as u32) + 1;
    let min_bits_symbols: u32 = bit_highbit32(max_symbol_value) + 2;
    min_bits_src.min(min_bits_symbols)
}

/// dynamically downsize 'table_log' when conditions are met.
/// It saves CPU time, by using smaller tables, while preserving or even improving compression ratio.
/// @return : recommended table_log (necessarily <= 'maxTableLog')
pub fn fse_optimal_table_log(max_table_log: u32, src_size: usize, max_symbol_value: u32) -> u32 {
    let mut table_log = max_table_log;

    // magic number minus, https://github.com/Cyan4973/FiniteStateEntropy/blob/5b3f8551695351d2a16d383c55bd7cddfd5c3813/lib/fse_compress.c#L341
    let max_bits_src = bit_highbit32(src_size as u32 - 1) - 2;
    let min_bits = fse_min_table_log(src_size, max_symbol_value);

    table_log = table_log.min(max_bits_src); // accuracy can be reduced
    table_log = table_log.max(min_bits); // Need a minimum to safely represent all symbol values

    table_log = table_log.min(FSE_MAX_TABLELOG);
    table_log = table_log.max(FSE_MIN_TABLELOG);

    table_log
}

#[test]
fn test_table_log_limit() {
    // Max value of min bits required imposed by FSE_MAX_SYMBOL_VALUE (too many min max :)
    let min_bits_symbols: u32 = bit_highbit32(FSE_MAX_SYMBOL_VALUE) + 2;
    assert_eq!(min_bits_symbols, 9);

    // make sure the upper bound FSE_MAX_TABLELOG is not smaller than the upper bound imposed by number of symbols
    assert!(min_bits_symbols < FSE_MAX_TABLELOG);
}

/// creates a table with the counts of each symbol
pub fn count_simple(input: &[u8]) -> CountsTable {
    let mut counts = [0; 256];

    for byte in input {
        counts[*byte as usize] += 1
    }
    counts
}

/// creates a table with the counts of each symbol
pub fn count_unrolled(input: &[u8]) -> CountsTable {
    let mut counts = Box::new([0; 256]);

    for byte in input.chunks_exact(8) {
        counts[byte[0] as usize] += 1;
        counts[byte[1] as usize] += 1;
        counts[byte[2] as usize] += 1;
        counts[byte[3] as usize] += 1;
        counts[byte[4] as usize] += 1;
        counts[byte[5] as usize] += 1;
        counts[byte[6] as usize] += 1;
        counts[byte[7] as usize] += 1;
    }
    *counts
}

// should be a generic eventually
type FseFunctionType = u8;

#[derive(Debug, Clone, Copy, Default)]
pub struct FseSymbolCompressionTransform {
    deltaFindState: i32,
    deltaNbBits: u32,
}

impl FseSymbolCompressionTransform {
    /// Approximate maximum cost of a symbol, in bits.
    ///
    /// Fractional get rounded up (i.e : a symbol with a normalized frequency of 3 gives the same result as a frequency of 2)
    /// note 1 : assume symbolValue is valid (<= maxSymbolValue)
    /// note 2 : if freq[symbolValue]==0, @return a fake cost of tableLog+1 bits */
    pub fn fse_get_max_nb_bits(&self) -> u32 {
        self.deltaNbBits + ((1 << 16) - 1) >> 16
    }

    /// Approximate symbol cost, as fractional value, using fixed-point format (accuracy_log fractional bits)
    ///
    /// note 1 : assume symbolValue is valid (<= maxSymbolValue)
    /// note 2 : if freq[symbolValue]==0, @return a fake cost of tableLog+1 bits */
    pub fn fse_bit_cost(&self, table_log: u32, accuracy_log: u32) -> u32 {
        let min_nb_bits: u32 = self.deltaNbBits >> 16;
        let threshold: u32 = (min_nb_bits + 1) << 16;

        assert!(table_log < 16);
        assert!(accuracy_log < 31 - table_log);
        let table_size = 1 << table_log;

        let delta_from_threshold: u32 = threshold - (self.deltaNbBits + table_size);
        let normalized_delta_from_threshold: u32 =
            (delta_from_threshold << accuracy_log) >> table_log; /* linear interpolation (very approximate) */
        let bit_multiplier: u32 = 1 << accuracy_log;
        assert!(self.deltaNbBits + table_size <= threshold);
        assert!(normalized_delta_from_threshold <= bit_multiplier);
        return (min_nb_bits + 1) * bit_multiplier - normalized_delta_from_threshold;
    }
}

/// Creating an ANSTable consists of following steps
///
/// 1. count symbol occurrence from source[] into table count[] (see hist.h)
/// 2. normalize counters so that sum(count[]) == Power_of_2 (2^table_log)
/// 3. save normalized counters to memory buffer using writeNCount()
/// 4. build encoding table 'CTable' from normalized counters
/// provides the minimum logSize to safely represent a distribution
///
/// build_table is step 4
///
/// get_normalized_counts() will ensure that sum of frequencies is == 2 ^ tableLog.
pub fn build_table(
    norm_counts: &NormCountsTable,
    table_log: u32,
    src_size: usize,
    max_symbol_value: u32,
) {
    let table_size = 1 << table_log;
    debug!("table_size {:?}", table_size);
    let step = fse_tablestep(table_size);
    let table_mask = table_size - 1;
    let mut highThreshold = table_size - 1;

    let mut cumul = vec![0_u32; max_symbol_value as usize + 2];

    // tmp table - TODO Could be externally allocated and reused
    // This is the classical table symbol table, where the state equals its position in the table
    // In the classical approach, they are illustrated like this
    // State    A    B    C
    // 1        2    3    5
    // 2        4    6    10
    // 3        7    8    15
    //
    let mut tableSymbol = vec![0_u8; table_size];

    // out table
    // get_ans_table_size will return usually a smaller value that table_size
    // Currently not clear why - 05.02.2021
    // let mut compression_table = vec![0_u32, get_ans_table_size(table_log, max_symbol_value)];

    // table.resize(get_ans_table_size(table_log, max_symbol_value) as usize, 0_u32);

    // symbol start positions
    debug!("max_symbol_value{:?}", max_symbol_value);
    for u in 1..max_symbol_value as usize + 1 {
        if norm_counts[u - 1] == -1 {
            // Low proba symbol
            cumul[u] = cumul[u - 1] + 1;
            tableSymbol[highThreshold] = (u - 1) as u8;
            highThreshold -= 1;
        } else {
            cumul[u] = cumul[u - 1] + norm_counts[u - 1] as u32;
        }
        print!("{:?}, ", cumul[u]);
        // trace!("{:?}", cumul[u]);
    }
    cumul[max_symbol_value as usize + 1] = table_size as u32 + 1;
    trace!("{:?}", cumul[max_symbol_value as usize + 1]);

    // Spread symbols int the symbol table
    // the distribution is no perfect, but close enough
    {
        let mut position = 0;
        for symbol in 0..max_symbol_value {
            let freq = norm_counts[symbol as usize];
            for _ in 0..freq {
                tableSymbol[position] = symbol as u8;
                position = (position + step) & table_mask;
                while position > highThreshold {
                    position = (position + step) & table_mask; // Low proba area
                }
            }
        }

        if log_enabled!(Trace) {
            for position in 0..table_size {
                trace!("tableSymbol[{:?}] {:?}", position, tableSymbol[position]);
            }
        }

        assert!(position == 0);
    }

    let mut table_u16 = vec![0_u16; cumul[max_symbol_value as usize + 1] as usize];
    // Build Table
    {
        for u in 0..table_size {
            let s = tableSymbol[u];
            table_u16[cumul[s as usize] as usize] = table_size as u16 + u as u16; // table_u16 : sorted by symbol order; gives next state value
            cumul[s as usize] += 1;
        }

        // if log_enabled!(Trace) {
        //     for (position, val) in table_u16.iter().enumerate() {
        //         trace!("table_u16[{:?}] {:?}", position, val);
        //     }
        //     // trace!("table_u16[{:?}] {:?}", 0, table_u16[0]);
        // }
    }

    // The symbol transformation table will help encoding input streams
    let mut symbol_tt = vec![FseSymbolCompressionTransform::default(); max_symbol_value as usize];

    // Build Symbol Transformation Table
    {
        let mut total = 0_i32;
        for symbol in 0..max_symbol_value as usize {
            let norm_count = norm_counts[symbol as usize];
            match norm_count {
                0 => {
                    symbol_tt[symbol as usize].deltaNbBits =
                        ((table_log + 1) << 16) - (1 << table_log)
                }
                -1 | 1 => {
                    symbol_tt[symbol].deltaNbBits = (table_log << 16) - (1 << table_log);
                    symbol_tt[symbol].deltaFindState = total - 1;
                    total += 1;
                }
                _ => {
                    let max_bits_out: u32 =
                        table_log - bit_highbit32(norm_counts[symbol] as u32 - 1);
                    let min_state_plus: u32 = (norm_counts[symbol] as u32) << max_bits_out;
                    symbol_tt[symbol].deltaNbBits = (max_bits_out << 16) - min_state_plus;
                    symbol_tt[symbol].deltaFindState = total - norm_counts[symbol] as i32;
                    total += norm_counts[symbol] as i32;
                }
            }
        }
    }
}

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

/// Normalize the frequencies.
///
/// get_normalized_counts() will ensure that sum of frequencies is == 2 ^ tableLog.
pub fn get_normalized_counts(
    counts: &CountsTable,
    table_log: u32,
    src_size: usize,
    max_symbol_value: u32,
) -> NormCountsTable {
    debug!("table_log: {:?}", table_log);
    if table_log < fse_min_table_log(src_size, max_symbol_value) {
        panic!("Too small tableLog, compression potentially impossible");
    };

    let total = src_size as u64;
    // Variable length arrays are not yet supported in Rust, [0_i16;max_symbol_value] would be enough for the counts;
    // https://doc.rust-lang.org/beta/unstable-book/language-features/unsized-locals.html
    let mut norm_counts = [0_i16; 256];

    /// rest to beat table
    const RTB_TABLE: [u32; 8] = [0, 473195, 504333, 520860, 550000, 700000, 750000, 830000];

    let scale: u64 = 62 - table_log as u64;

    let step: u64 = (1 << 62) / total; // <== here, one division ! .. okay
    let v_step: u64 = 1 << (scale - 20);

    // keeps track of the sum of occurences of symbols to match 1 << table_log
    let mut still_to_distribute: i32 = 1 << table_log;

    let mut largest: u32 = 0;
    let mut largest_p: i16 = 0;

    let low_threshold = (total >> table_log) as u32;
    for symbol in 0..=max_symbol_value as usize {
        let symbol_count = counts[symbol];

        // this is not yet supported, rle case
        assert!(symbol_count as u64 != total);
        if symbol_count == 0 {
            continue;
        }
        if (symbol_count as u32) < low_threshold {
            norm_counts[symbol] = -1;
            still_to_distribute -= 1;
        } else {
            let mut proba: i16 = ((symbol_count as u64 * step) >> scale) as i16;
            if proba < 8 {
                let rest_to_beat = v_step * RTB_TABLE[proba as usize] as u64;
                if (symbol_count as u64 * step) - ((proba as u64) << scale) > rest_to_beat {
                    proba += 1;
                }
            }

            if proba > largest_p {
                largest_p = proba;
                largest = symbol as u32;
            }
            norm_counts[symbol] = proba;
            still_to_distribute -= proba as i32;
        }
    }

    debug!("still_to_distribute: {}", still_to_distribute);
    if -still_to_distribute >= (norm_counts[largest as usize] as i32 >> 1) {
        unimplemented!()
        /* corner case, need another normalization method */
        // size_t const errorCode = FSE_normalizeM2(norm_counts, tableLog, count, total, maxSymbolValue);
        // if (FSE_isError(errorCode)) return errorCode;
    } else {
        // assign rest to match total norm counts = 1 << table_log
        norm_counts[largest as usize] += still_to_distribute as i16;
    }

    if log_enabled!(Trace) {
        let mut n_total = 0;
        for symbol in 0..=max_symbol_value as usize {
            if norm_counts[symbol] != 0 {
                trace!("{}: {}", symbol, norm_counts[symbol]);
            }
            n_total += norm_counts[symbol].abs();
        }

        if n_total != (1 << table_log) {
            error!("Warning Total {} != table_log {}", n_total, 1 << table_log);
        }
    }

    norm_counts
}

fn get_column_heights(counts: &CountsTable) -> Vec<u32> {
    let max_val = get_table_max_val(&counts);
    let total = get_num_symbols(&counts);

    let sorted_counts = get_sorted_symbols(&counts);

    let mut is_first = true; // first == most probable
    let column_heigths = sorted_counts
        .iter()
        .map(|entry| {
            let prob = counts[entry.symbol as usize] as f32 / total as f32;
            let mut val = (max_val as f32 * prob).floor() as u32;

            if is_first {
                is_first = false;
                val += 1;
            }
            val
        })
        .collect::<Vec<_>>();

    column_heigths
}

fn get_most_probable_symbol(counts: &CountsTable) -> u8 {
    get_sorted_symbols(&counts)[0].symbol
}

#[derive(Debug)]
struct SymbolAndCount {
    symbol: u8,
    count: usize,
}

impl SymbolAndCount {
    fn get_prob(&self, total: usize) -> f32 {
        self.count as f32 / total as f32
    }
}

fn get_sorted_symbols(counts: &CountsTable) -> Vec<SymbolAndCount> {
    let mut symbols = counts
        .into_iter()
        .enumerate()
        .filter(|(_, val)| **val != 0)
        .map(|(symbol, val)| SymbolAndCount {
            symbol: symbol as u8,
            count: *val,
        })
        .collect::<Vec<_>>();

    // symbols.sort_by(|symb_cnt| symb_cnt.count);
    symbols.sort_by(|a, b| b.count.cmp(&a.count));

    symbols
}

fn get_table_max_val(counts: &CountsTable) -> u32 {
    // magic_extra_bits is some value between 2 and 8
    // the higher the value, the better the compression, but it costs performance
    let magic_extra_bits = 4;

    let num_symbols = get_num_unique_symbols(&counts);
    let num_precision_bits = (num_symbols as f32).log2() as u32 + magic_extra_bits;
    let max_val = 2_u32.pow(num_precision_bits) - 1;
    max_val
}

fn get_num_unique_symbols(counts: &CountsTable) -> usize {
    counts.into_iter().filter(|el| **el != 0).count()
}

fn get_num_symbols(counts: &CountsTable) -> usize {
    counts.into_iter().sum()
}

#[cfg(test)]
mod tests {

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
    fn test_statistic_fns() {
        setup();
        let test_data = get_test_data();

        let counts = count_simple(&test_data);
        assert_eq!(counts[A_BYTE as usize], 45);
        assert_eq!(counts[B_BYTE as usize], 35);
        assert_eq!(counts[C_BYTE as usize], 20);

        let table_log =
            fse_optimal_table_log(FSE_DEFAULT_TABLELOG, test_data.len(), FSE_MAX_SYMBOL_VALUE);

        let norm_counts = get_normalized_counts(&counts, table_log, test_data.len(), 255);

        assert_eq!(get_num_unique_symbols(&counts), 3);

        let sorted_counts = get_sorted_symbols(&counts);
        assert_eq!(sorted_counts[0].symbol, A_BYTE);

        let max_val = get_table_max_val(&counts);
        assert_eq!(max_val, 31);

        let column_heigths = get_column_heights(&counts);
        assert_eq!(column_heigths, &[14, 10, 6]);
        // assert_eq!(get_column_height(&counts, A_BYTE), 14);
        // assert_eq!(get_column_height(&counts, B_BYTE), 10);
        // assert_eq!(get_column_height(&counts, C_BYTE), 6);
    }

    #[test]
    fn test_create_table() {
        setup();
        let test_data = get_test_data();
        let counts = count_simple(&test_data);
        assert_eq!(counts[A_BYTE as usize], 45);
        assert_eq!(counts[B_BYTE as usize], 35);
        assert_eq!(counts[C_BYTE as usize], 20);

        let table_log =
            fse_optimal_table_log(FSE_DEFAULT_TABLELOG, test_data.len(), FSE_MAX_SYMBOL_VALUE);

        let norm_counts = get_normalized_counts(&counts, table_log, test_data.len(), 255);
        build_table(
            &norm_counts,
            table_log,
            test_data.len(),
            FSE_MAX_SYMBOL_VALUE,
        );
    }
}
