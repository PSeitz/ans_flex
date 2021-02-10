use crate::hist::NormCountsTable;
use crate::*;
use log::log_enabled;
use log::Level::{Debug, Trace};
use log::*;

#[derive(Debug, Clone)]
pub struct CompressionTable {
    pub state_table: Vec<u16>,
    pub symbol_tt: Vec<FseSymbolCompressionTransform>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FseSymbolCompressionTransform {
    pub deltaFindState: i32,
    pub deltaNbBits: u32,
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

pub fn fse_min_table_log(src_size: usize, max_symbol_value: u32) -> u32 {
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
    mut max_symbol_value: u32,
) -> CompressionTable {
    let table_size = 1 << table_log;
    debug!("table_size {:?}", table_size);
    let step = fse_tablestep(table_size);
    let table_mask = table_size - 1;
    let mut high_threshold = table_size - 1;
    max_symbol_value = max_symbol_value.min(FSE_MAX_SYMBOL_VALUE);
    let mut cumul = vec![0_u32; max_symbol_value as usize + 2];

    // tmp table - TODO Could be externally allocated and reused
    // This is the classical table symbol table, where the state equals its position in the table
    // In the classical approach, they are illustrated like this
    // State    A    B    C
    // 1        2    3    5
    // 2        4    6    10
    // 3        7    8    15
    //
    let mut table_symbol = vec![0_u8; table_size];

    // out table
    // get_ans_table_size will return usually a smaller value that table_size
    // Currently not clear why - 05.02.2021
    // let mut compression_table = vec![0_u32, get_ans_table_size(table_log, max_symbol_value)];

    // table.resize(get_ans_table_size(table_log, max_symbol_value) as usize, 0_u32);

    // symbol start positions
    debug!("max_symbol_value{:?}", max_symbol_value);
    for u in 1..=max_symbol_value as usize + 1 {
        if norm_counts[u - 1] == -1 {
            // Low proba symbol
            cumul[u] = cumul[u - 1] + 1;
            table_symbol[high_threshold] = (u - 1) as u8;
            high_threshold -= 1;
        } else {
            cumul[u] = cumul[u - 1] + norm_counts[u - 1] as u32;
        }
        // print!("{:?}, ", cumul[u]);
        trace!("{:?}", cumul[u]);
    }
    cumul[max_symbol_value as usize + 1] = table_size as u32 + 1;
    trace!("{:?}", cumul[max_symbol_value as usize + 1]);

    // Spread symbols int the symbol table
    // the distribution is not perfect, but close enough
    {
        let mut position = 0;
        for symbol in 0..=max_symbol_value {
            let freq = norm_counts[symbol as usize];
            for _ in 0..freq {
                table_symbol[position] = symbol as u8;
                position = (position + step) & table_mask;
                while position > high_threshold {
                    position = (position + step) & table_mask; // Low proba area
                }
            }
        }

        if log_enabled!(Trace) {
            for position in 0..table_size {
                trace!("table_symbol[{:?}] {:?}", position, table_symbol[position]);
            }
        }

        assert!(position == 0);
    }

    let mut state_table = vec![0_u16; cumul[max_symbol_value as usize + 1] as usize];
    // Build Table
    {
        for u in 0..table_size {
            let s = table_symbol[u];
            state_table[cumul[s as usize] as usize] = table_size as u16 + u as u16; // state_table : sorted by symbol order; gives next state value
            cumul[s as usize] += 1;
        }

        if log_enabled!(Trace) {
            for (position, val) in state_table.iter().enumerate() {
                trace!("state_table[{:?}] {:?}", position, val);
            }
            // trace!("state_table[{:?}] {:?}", 0, state_table[0]);
        }
    }

    // The symbol transformation table will help encoding input streams
    let mut symbol_tt = vec![FseSymbolCompressionTransform::default(); max_symbol_value as usize + 1];

    // Build Symbol Transformation Table
    {
        let mut total = 0_i32;
        for symbol in 0..=max_symbol_value as usize {
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
                    // trace!(
                    //     "symbol_tt[{:?}] deltaNbBits {:?} deltaFindState {:?}",
                    //     symbol,
                    //     symbol_tt[symbol].deltaNbBits,
                    //     symbol_tt[symbol].deltaFindState
                    // );
                }
            }
        }
    }

    if log_enabled!(Debug) {
        for symbol in 0..=max_symbol_value as usize {
            let weight = norm_counts[symbol];
            if weight == 0 {
                continue;
            }
            let s_tt = symbol_tt[symbol];
            debug!(
                "symbol:{:?} w:{:?} deltaNbBits:{:?} deltaFindState:{:?} maxBits:{:?} fracBits:{:?} ",
                symbol,
                weight,
                s_tt.deltaNbBits,
                s_tt.deltaFindState,
                s_tt.fse_get_max_nb_bits(),
                s_tt.fse_bit_cost(table_log, 8) as f32 / 256.0
            );
        }
    }

    CompressionTable {
        state_table,
        symbol_tt,
    }
}

